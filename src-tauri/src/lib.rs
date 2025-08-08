#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use tauri::Emitter;

#[macro_use]
extern crate lazy_static;
extern crate shellexpand;
extern crate shlex;

lazy_static! {
    // Initialize with an empty string. We'll set the real path on startup.
    static ref CURRENT_DIR: Mutex<String> = Mutex::new(String::new());
}

#[tauri::command]
fn handle_command(command: String, app: tauri::AppHandle) {
    thread::spawn(move || {
        let mut parts = command.trim().split_whitespace();
        let program = parts.next().unwrap_or("").to_string();
        let args: Vec<&str> = parts.collect();

        if program == "cd" {
            let current_dir = CURRENT_DIR.lock().unwrap().clone();
            let target_arg = args.join(" ");

            let cd_target_command = match target_arg.trim() {
                "" | "~" | "~/" => "cd".to_string(),
                other => format!("cd {}", shlex::quote(other)),
            };

            let command_to_run = format!(
                "cd {} && {} && pwd",
                shlex::quote(&current_dir),
                cd_target_command
            );

            match Command::new("wsl").arg("-e").arg("sh").arg("-c").arg(&command_to_run).output() {
                Ok(output) => {
                    if output.status.success() {
                        let new_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let mut dir_state = CURRENT_DIR.lock().unwrap();
                        *dir_state = new_path.clone();
                        app.emit("path-update", new_path).unwrap();
                    } else {
                        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
                        app.emit("terminal-output", error_message).unwrap();
                    }
                }
                Err(e) => {
                    app.emit("terminal-output", format!("Failed to execute command: {}", e)).unwrap();
                }
            }
            return;
        }

        if program.is_empty() { return; }

        if program == "help" {
            let help_message = "BUILT-IN COMMANDS:\n- help: Shows this message\n- cd [dir]: Changes directory\n- clear: Clears the terminal screen\n\nAll other commands are passed to WSL.";
            app.emit("terminal-output", help_message).unwrap();
            return;
        }

        let current_dir = CURRENT_DIR.lock().unwrap().clone();
        
        let command_str = format!(
            "cd {} && {} {}",
            shlex::quote(&current_dir),
            shlex::quote(&program),
            args.iter().map(|a| shlex::quote(a)).collect::<Vec<_>>().join(" ")
        );

        let mut child = match Command::new("wsl")
            .arg("-e").arg("sh").arg("-c").arg(&command_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                app.emit("terminal-output", format!("Failed to start command: {}", e)).unwrap();
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    app.emit("terminal-output", line_content).unwrap();
                }
            }
        }
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    app.emit("terminal-output", line_content).unwrap();
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // --- NEW: Get the absolute home directory path from WSL on startup ---
    let wsl_home_path = Command::new("wsl")
        .arg("-e")
        .arg("sh")
        .arg("-c")
        .arg("pwd") // A new shell's default directory is the home directory
        .output()
        .expect("Failed to execute WSL to get home directory.")
        .stdout;

    let home_path_str = String::from_utf8_lossy(&wsl_home_path).trim().to_string();

    // Store this absolute path in our global state immediately.
    if !home_path_str.is_empty() {
        let mut dir_state = CURRENT_DIR.lock().unwrap();
        *dir_state = home_path_str;
    }
    // --- End of new startup logic ---

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![handle_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}