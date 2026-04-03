use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

#[cfg(windows)]
mod pty {
    use std::ffi::OsStr;
    use std::io::{Read, Write};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Console::*;
    use windows::Win32::System::Threading::*;
    use windows::Win32::System::ProcessStatus::*;

    pub struct PtyReader {
        pipe: HANDLE,
    }

    impl PtyReader {
        pub fn new(pipe: HANDLE) -> Self {
            Self { pipe }
        }

        pub fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut bytes_read: u32 = 0;
            let result = unsafe {
                ReadFile(self.pipe, buf.as_mut_ptr() as *mut _, buf.len() as u32, &mut bytes_read, null_mut())
            };
            if result.is_ok() {
                Ok(bytes_read as usize)
            } else {
                Ok(0)
            }
        }
    }

    pub struct PtyWriter {
        pipe: HANDLE,
    }

    impl PtyWriter {
        pub fn new(pipe: HANDLE) -> Self {
            Self { pipe }
        }

        pub fn write(&self, data: &[u8]) -> std::io::Result<()> {
            let mut bytes_written: u32 = 0;
            unsafe {
                WriteFile(self.pipe, data.as_ptr() as *const _, data.len() as u32, &mut bytes_written, null_mut());
            }
            Ok(())
        }
    }

    pub struct PtyProcess {
        _proc_handle: HANDLE,
        _thread_handle: HANDLE,
        output: PtyReader,
        input: PtyWriter,
    }

    fn to_wide_string(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    impl PtyProcess {
        pub fn spawn() -> std::io::Result<Self> {
            let mut sa = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: null_mut(),
                bInheritHandle: true.into(),
            };

            let mut output_pipe: HANDLE = INVALID_HANDLE_VALUE;
            let mut input_pipe: HANDLE = INVALID_HANDLE_VALUE;
            let mut conpty_pipe: HANDLE = INVALID_HANDLE_VALUE;

            unsafe {
                CreatePipe(&mut output_pipe, &mut conpty_pipe, &mut sa, 0)?;
                CreatePipe(&mut input_pipe, &mut conpty_pipe, &mut sa, 0)?;

                let mut conpty_reader: HANDLE = INVALID_HANDLE_VALUE;
                let mut conpty_writer: HANDLE = INVALID_HANDLE_VALUE;
                DuplicateHandle(
                    GetCurrentProcess(),
                    conpty_pipe,
                    GetCurrentProcess(),
                    &mut conpty_reader,
                    0,
                    false,
                    DUPLICATE_CLOSE_SOURCE | DUPLICATE_SAME_ACCESS,
                )?;
                DuplicateHandle(
                    GetCurrentProcess(),
                    conpty_pipe,
                    GetCurrentProcess(),
                    &mut conpty_writer,
                    0,
                    false,
                    DUPLICATE_CLOSE_SOURCE | DUPLICATE_SAME_ACCESS,
                )?;

                let cmd = to_wide_string("cmd.exe");
                let mut si = STARTUPINFOW {
                    cb: std::mem::size_of::<STARTUPINFOW>() as u32,
                    hStdOutput: conpty_writer,
                    hStdInput: conpty_reader,
                    hStdError: conpty_writer,
                    dwFlags: STARTF_USESTDHANDLES,
                    ..Default::default()
                };

                let mut pi = PROCESS_INFORMATION::default();

                CreateProcessW(
                    null_mut(),
                    cmd.as_ptr() as *mut _,
                    null_mut(),
                    null_mut(),
                    true,
                    CREATE_NEW_CONSOLE,
                    null_mut(),
                    null_mut(),
                    &mut si,
                    &mut pi,
                )?;

                CloseHandle(pi.hThread)?;

                Ok(Self {
                    _proc_handle: pi.hProcess,
                    _thread_handle: pi.hThread,
                    output: PtyReader::new(output_pipe),
                    input: PtyWriter::new(input_pipe),
                })
            }
        }

        pub fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.output.read(buf)
        }

        pub fn write(&self, data: &[u8]) -> std::io::Result<()> {
            self.input.write(data)
        }
    }
}

#[cfg(not(windows))]
mod pty {
    use std::process::{Command, Stdio};
    use std::io::{Read, Write};

    pub struct PtyProcess {
        child: std::process::Child,
        stdin: Option<Box<dyn Write + Send>>,
        stdout: Box<dyn Read + Send>,
    }

    impl PtyProcess {
        pub fn spawn() -> std::io::Result<Self> {
            let shell = if cfg!(target_os = "macos") { "zsh" } else { "bash" };
            let mut child = Command::new(shell)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();

            Ok(Self {
                child,
                stdin: Some(Box::new(stdin)),
                stdout: Box::new(stdout),
            })
        }

        pub fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.stdout.read(buf)
        }

        pub fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
            if let Some(ref mut stdin) = self.stdin {
                stdin.write_all(data)?;
            }
            Ok(())
        }
    }
}

struct PtyState {
    process: Arc<Mutex<Option<pty::PtyProcess>>>,
}

#[tauri::command]
fn init_pty(window: tauri::Window) -> Result<String, String> {
    let process = pty::PtyProcess::spawn().map_err(|e| e.to_string())?;

    window.app_handle().manage(PtyState {
        process: Arc::new(Mutex::new(Some(process))),
    });

    let state = window.app_handle().state::<PtyState>().clone();
    let w = window.clone();

    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            let process = {
                let guard = state.process.lock().unwrap();
                guard.as_mut().map(|p| {
                    let mut temp_buf = [0u8; 1024];
                    match p.read(&mut temp_buf) {
                        Ok(0) => None,
                        Ok(n) => Some(temp_buf[..n].to_vec()),
                        Err(_) => None,
                    }
                })
            };

            match process {
                Some(Some(data)) => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    let _ = w.emit("pty-data", text);
                }
                Some(None) | None => break,
                _ => {}
            }
        }
    });

    Ok("PTY Initialized".into())
}

#[tauri::command]
fn write_to_pty(state: State<'_, PtyState>, input: String) -> Result<(), String> {
    let guard = state.process.lock().unwrap();
    if let Some(ref mut p) = *guard {
        p.write(input.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![init_pty, write_to_pty])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
