// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc_client;

use ipc_client::IpcClient;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Manager};

#[tauri::command]
fn get_daemon_status() -> Result<String, String> {
    IpcClient::send_command(r#"{"cmd": "status"}"#)
}

#[tauri::command]
fn lock_workstation() -> Result<String, String> {
    IpcClient::send_command(r#"{"cmd": "lock"}"#)
}

#[tauri::command]
fn revoke_device(device_id: String) -> Result<String, String> {
    let cmd = format!(r#"{{"cmd": "revoke", "device_id": "{}"}}"#, device_id);
    IpcClient::send_command(&cmd)
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit Control Center");
    let lock = CustomMenuItem::new("lock".to_string(), "Lock Workstation Now");
    let show = CustomMenuItem::new("show".to_string(), "Open Control Center");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(lock)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                "lock" => {
                    let _ = IpcClient::send_command(r#"{"cmd": "lock"}"#);
                }
                _ => {}
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_daemon_status,
            lock_workstation,
            revoke_device
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenTap Tauri application");
}
