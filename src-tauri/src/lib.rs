use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Command};
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use machineid_rs::{IdBuilder, Encryption};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

struct ProcessManager(Arc<Mutex<HashMap<String, AsyncGroupChild>>>);

fn get_hw_hash() -> Vec<u8> {
    let mut builder = IdBuilder::new(Encryption::SHA256);
    builder.add_component(machineid_rs::HWIDComponent::SystemID);
    let hw_id = builder.build("hayfrp-client-far1sh").expect("Failed to build HWID");

    let mut hasher = Sha256::new();
    hasher.update(hw_id.as_bytes());
    hasher.finalize().to_vec()
}

#[tauri::command]
async fn secure_encrypt(token: String) -> Result<String, String> {
    if token.is_empty() { return Err("Token is empty".into()); }

    let key = get_hw_hash();
    let data_bytes = token.as_bytes();
    let mut result = Vec::with_capacity(data_bytes.len());

    for (i, &byte) in data_bytes.iter().enumerate() {
        result.push(byte ^ key[i % key.len()]);
    }
    Ok(general_purpose::STANDARD.encode(result))
}

#[tauri::command]
async fn secure_decrypt(encrypted_base64: String) -> Result<String, String> {
    let decoded = general_purpose::STANDARD.decode(encrypted_base64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let key = get_hw_hash();
    let mut result = Vec::with_capacity(decoded.len());
    for (i, &byte) in decoded.iter().enumerate() {
        result.push(byte ^ key[i % key.len()]);
    }

    String::from_utf8(result).map_err(|_| "Decryption error: invalid key".to_string())
}

#[tauri::command]
fn run_and_get_frpc(path: String) -> Result<String, String> {
    use std::process::Command;

    let output = Command::new(path)
        .arg("-v")
        .output()
        .map_err(|e| format!("启动失败: {}", e))?;

    if !output.status.success() {
        return Err("程序执行失败".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let last = stdout.lines().last().map(|s| s.trim()).filter(|s| !s.is_empty());

    match last {
        Some(line) => Ok(line.to_string()),
        None => Err("获取不到版本".to_string()),
    }
}

#[tauri::command]
async fn run_program(
    app: AppHandle,
    state: State<'_, ProcessManager>,
    id: String,
    path: String,
    args: Vec<String>,
) -> Result<(), String> {
    let mut cmd = Command::new(path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
        cmd.creation_flags(0x08000000);

    // 显式标注错误类型 e: std::io::Error
    let mut group = cmd.group_spawn()
        .map_err(|e: std::io::Error| e.to_string())?;

    // 从 AsyncGroupChild 提取管道
    let stdout = group.inner().stdout.take().ok_or("get stdout error")?;
    let stderr = group.inner().stderr.take().ok_or("get stderr error")?;

    {
        let mut lock = state.0.lock().unwrap();
        lock.insert(id.clone(), group);
    }

    let app_clone = app.clone();
    let id_clone = id.clone();

    tokio::spawn(async move {
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => { let _ = app_clone.emit(&format!("log-stdout-{}", id_clone), l); }
                        _ => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => { let _ = app_clone.emit(&format!("log-stderr-{}", id_clone), l); }
                        _ => break,
                    }
                }
            }
        }

        let manager_state = app_clone.state::<ProcessManager>();
        let mut lock = manager_state.0.lock().unwrap();
        lock.remove(&id_clone);
        let _ = app_clone.emit(&format!("process-exit-{}", id_clone), "exited");
    });

    Ok(())
}

#[tauri::command]
async fn kill_program(state: State<'_, ProcessManager>, id: String) -> Result<(), String> {
    let child_to_kill = {
        let mut lock = state.0.lock().unwrap();
        lock.remove(&id)
    };

    if let Some(mut group) = child_to_kill {
        group.kill()
            .await
            .map_err(|e: std::io::Error| e.to_string())?;
        Ok(())
    } else {
        Err("No running process found".to_string())
    }
}

#[tauri::command]
async fn is_running(state: State<'_, ProcessManager>, id: String) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    Ok(lock.contains_key(&id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ProcessManager(Arc::new(Mutex::new(HashMap::new()))))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            secure_encrypt,
            secure_decrypt,
            run_and_get_frpc,
            run_program,
            kill_program,
            is_running
        ])
        .setup(|app| {
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", "显示", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
                ],
            )?;

            let _tray = TrayIconBuilder::new()
                .title("HAYFRP")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run error");
}