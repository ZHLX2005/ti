use std::process::{Command, Stdio};
use tauri::Emitter;

#[tauri::command]
fn init_shell(window: tauri::Window) -> Result<String, String> {
    // 简单的命令执行演示
    let _ = window.emit("pty-data", "终端已连接\r\n$ ");
    Ok("Shell initialized".into())
}

#[tauri::command]
fn write_to_shell(window: tauri::Window, input: String) -> Result<String, String> {
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let args = if cfg!(target_os = "windows") {
        vec!["/C", &input]
    } else {
        vec!["-c", &input]
    };

    let output = Command::new(shell)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let result = if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let _ = window.emit("pty-data", &result);
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![init_shell, write_to_shell])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
