use std::process::{Command, Stdio};
use tauri_plugin_cli::CliExt;

#[tauri::command]
fn run_command(cmd: String) -> Result<String, String> {
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let args = if cfg!(target_os = "windows") {
        vec!["/C", &cmd]
    } else {
        vec!["-c", &cmd]
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
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_cli::init())
        .invoke_handler(tauri::generate_handler![run_command])
        .setup(|app| {
            match app.cli().matches() {
                Ok(_matches) => {
                    println!("CLI initialized");
                }
                Err(e) => {
                    eprintln!("CLI error: {}", e);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
