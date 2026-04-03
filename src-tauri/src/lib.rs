use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

struct PtyState {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

#[tauri::command]
fn init_pty(window: tauri::Window) -> Result<String, String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new_default_prog();
    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let writer_state = Arc::new(Mutex::new(writer));

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    window.app_handle().manage(PtyState {
        writer: writer_state.clone(),
    });

    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = window.emit("pty-data", data);
                }
                _ => break,
            }
        }
    });

    Ok("PTY Initialized".into())
}

#[tauri::command]
fn write_to_pty(state: State<'_, PtyState>, input: String) -> Result<(), String> {
    let mut writer = state.writer.lock().unwrap();
    writer.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
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
