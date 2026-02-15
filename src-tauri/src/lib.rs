use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_store::StoreExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Command};
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use machineid_rs::{IdBuilder, Encryption};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use tokio::sync::RwLock;

mod http_server;
use http_server::{ServerState, AccountInfo};
use axum::{
    extract::{Json, State as AxumState},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

struct ProcessManager(Arc<Mutex<HashMap<String, AsyncGroupChild>>>);

#[derive(Clone)]
struct HttpServerState {
    server_state: Arc<ServerState>,
    app_handle: AppHandle,
}

#[derive(Debug, serde::Serialize)]
struct AccountResponse {
    name: String,
    csrf: String,
    avatar: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ApiResponse<T> {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, serde::Deserialize)]
struct SyncCsrfRequest {
    csrf: String,
}

#[derive(Debug, serde::Deserialize)]
struct UserInfoResponse {
    #[serde(default)]
    status: Option<bool>,  // 改为 bool，因为 API 返回的是 true/false
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    qid: Option<String>,
    // 添加其他可能的字段
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

struct HttpServerHandle {
    handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    state: Arc<ServerState>,
}

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

// 同步版本的解密函数（用于 HTTP 处理器）
fn secure_decrypt_sync(encrypted_base64: String) -> Result<String, String> {
    let decoded = general_purpose::STANDARD.decode(encrypted_base64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let key = get_hw_hash();
    let mut result = Vec::with_capacity(decoded.len());
    for (i, &byte) in decoded.iter().enumerate() {
        result.push(byte ^ key[i % key.len()]);
    }

    String::from_utf8(result).map_err(|_| "Decryption error: invalid key".to_string())
}

// 验证 API Key
fn verify_api_key(headers: &HeaderMap, api_key: &str) -> bool {
    if api_key.is_empty() {
        return true;
    }
    headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|key| key == api_key)
        .unwrap_or(false)
}

// 自定义的 /accounts 处理器，返回解密后的 CSRF
async fn handle_get_accounts(
    AxumState(state): AxumState<HttpServerState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let api_key = state.server_state.api_key.read().await;
    
    if !verify_api_key(&headers, &api_key) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<Vec<AccountResponse>> {
                success: false,
                message: "Invalid API Key".to_string(),
                data: None,
            }),
        );
    }

    let accounts = state.server_state.accounts.read().await.clone();
    
    println!("GET /accounts - returning {} accounts", accounts.len());
    
    // 解密所有账号的 CSRF
    let mut response_accounts = Vec::new();
    for acc in accounts {
        match secure_decrypt_sync(acc.csrf.clone()) {
            Ok(decrypted_csrf) => {
                println!("  - {} (csrf: {}...)", acc.name, &decrypted_csrf[..decrypted_csrf.len().min(10)]);
                response_accounts.push(AccountResponse {
                    name: acc.name,
                    csrf: decrypted_csrf,
                    avatar: acc.avatar,
                });
            }
            Err(e) => {
                eprintln!("Failed to decrypt CSRF for {}: {}", acc.name, e);
            }
        }
    }
    
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Success".to_string(),
            data: Some(response_accounts),
        }),
    )
}

// 自定义的 /sync 处理器，接收浏览器的 CSRF 并验证、保存
async fn handle_sync_csrf(
    AxumState(state): AxumState<HttpServerState>,
    headers: HeaderMap,
    Json(payload): Json<SyncCsrfRequest>,
) -> impl IntoResponse {
    let api_key = state.server_state.api_key.read().await;
    
    if !verify_api_key(&headers, &api_key) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<String> {
                success: false,
                message: "Invalid API Key".to_string(),
                data: None,
            }),
        );
    }

    // 先检查是否已存在
    let accounts = state.server_state.accounts.read().await.clone();
    for acc in &accounts {
        if let Ok(decrypted) = secure_decrypt_sync(acc.csrf.clone()) {
            if decrypted == payload.csrf {
                println!("CSRF already exists for account: {}", acc.name);
                return (
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        message: format!("CSRF synced for account: {}", acc.name),
                        data: Some(acc.name.clone()),
                    }),
                );
            }
        }
    }
    
    // 不存在，验证 CSRF 并添加
    println!("Validating new CSRF...");
    
    // 使用 Tauri Store 读取 API URL
    let store = match state.app_handle.store("settings.json") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to get store: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: "Failed to access settings".to_string(),
                    data: None,
                }),
            );
        }
    };
    
    let api_url = store.get("api_url")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "https://api.hayfrp.com".to_string());
    
    // 调用 API 验证 CSRF
    let client = reqwest::Client::new();
    let user_info_result = client
        .post(format!("{}/user", api_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "csrf": payload.csrf,
            "type": "info"
        }))
        .send()
        .await;
    
    let user_info = match user_info_result {
        Ok(response) => {
            if !response.status().is_success() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse {
                        success: false,
                        message: format!("API request failed: {}", response.status()),
                        data: None,
                    }),
                );
            }
            
            // 先获取原始文本来调试
            let response_text = match response.text().await {
                Ok(text) => {
                    println!("API Response: {}", text);
                    text
                }
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse {
                            success: false,
                            message: format!("Failed to read response: {}", e),
                            data: None,
                        }),
                    );
                }
            };
            
            // 尝试解析 JSON
            match serde_json::from_str::<UserInfoResponse>(&response_text) {
                Ok(info) => info,
                Err(e) => {
                    eprintln!("Failed to parse JSON: {}", e);
                    eprintln!("Response text: {}", response_text);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse {
                            success: false,
                            message: format!("Failed to parse response: {}. Response: {}", e, response_text),
                            data: None,
                        }),
                    );
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to validate CSRF: {}", e),
                    data: None,
                }),
            );
        }
    };
    
    // 检查响应状态
    if user_info.status != Some(true) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                message: user_info.message.unwrap_or_else(|| "Invalid CSRF".to_string()),
                data: None,
            }),
        );
    }
    
    let username = match user_info.username {
        Some(name) => name,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse {
                    success: false,
                    message: "No username in response".to_string(),
                    data: None,
                }),
            );
        }
    };
    
    // 加密 CSRF
    let encrypted_csrf = match secure_encrypt_sync(payload.csrf.clone()) {
        Ok(enc) => enc,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to encrypt CSRF: {}", e),
                    data: None,
                }),
            );
        }
    };
    
    // 生成头像 URL
    let avatar = user_info.qid
        .filter(|qid| !qid.is_empty())
        .map(|qid| format!("https://q1.qlogo.cn/g?b=qq&nk={}&s=640", qid));
    
    // 使用 Tauri Store 更新账号列表
    let mut current_accounts = store.get("accounts")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    
    // 检查是否已存在该用户名
    let mut found = false;
    for acc in current_accounts.iter_mut() {
        if acc.get("name").and_then(|v| v.as_str()) == Some(&username) {
            // 更新现有账号的 CSRF
            if let Some(obj) = acc.as_object_mut() {
                obj.insert("csrf".to_string(), serde_json::Value::String(encrypted_csrf.clone()));
                if let Some(ref av) = avatar {
                    obj.insert("avatar".to_string(), serde_json::Value::String(av.clone()));
                }
            }
            found = true;
            break;
        }
    }
    
    if !found {
        // 添加新账号
        let mut new_account = serde_json::Map::new();
        new_account.insert("name".to_string(), serde_json::Value::String(username.clone()));
        new_account.insert("csrf".to_string(), serde_json::Value::String(encrypted_csrf.clone()));
        if let Some(av) = avatar.clone() {
            new_account.insert("avatar".to_string(), serde_json::Value::String(av));
        }
        new_account.insert("status".to_string(), serde_json::Value::Bool(true));
        
        current_accounts.push(serde_json::Value::Object(new_account));
    }
    
    // 保存到 Tauri Store
    store.set("accounts", serde_json::Value::Array(current_accounts.clone()));
    
    // 保存 store
    if let Err(e) = store.save() {
        eprintln!("Failed to save store: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                success: false,
                message: "Failed to save store".to_string(),
                data: None,
            }),
        );
    }
    
    // 更新内存中的 state
    let new_accounts: Vec<AccountInfo> = current_accounts.iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let csrf = item.get("csrf")?.as_str()?.to_string();
            let avatar = item.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string());
            Some(AccountInfo { name, csrf, avatar })
        })
        .collect();
    
    *state.server_state.accounts.write().await = new_accounts;
    
    println!("Account added/updated: {}", username);
    
    // 发送事件到前端，通知账号列表已更新
    if let Err(e) = state.app_handle.emit("accounts-updated", ()) {
        eprintln!("Failed to emit accounts-updated event: {}", e);
    }
    
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: format!("Account {} synced successfully", username),
            data: Some(username),
        }),
    )
}

// 同步版本的加密函数
fn secure_encrypt_sync(token: String) -> Result<String, String> {
    if token.is_empty() {
        return Err("Token is empty".into());
    }

    let key = get_hw_hash();
    let data_bytes = token.as_bytes();
    let mut result = Vec::with_capacity(data_bytes.len());

    for (i, &byte) in data_bytes.iter().enumerate() {
        result.push(byte ^ key[i % key.len()]);
    }
    Ok(general_purpose::STANDARD.encode(result))
}

// 健康检查
async fn handle_health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Server is running".to_string(),
            data: Some("OK"),
        }),
    )
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

#[tauri::command]
async fn reload_accounts(
    app: AppHandle,
    http_server: State<'_, HttpServerHandle>,
) -> Result<(), String> {
    // 使用 Tauri Store API 读取账号列表
    let store = app.store("settings.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;
    
    let accounts_value = store.get("accounts")
        .ok_or("No accounts key found")?;
    
    let accounts_array = accounts_value.as_array()
        .ok_or("accounts is not an array")?;
    
    let accounts_data: Vec<AccountInfo> = accounts_array.iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let name = obj.get("name")?.as_str()?.to_string();
            let csrf = obj.get("csrf")?.as_str()?.to_string();
            let avatar = obj.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string());
            Some(AccountInfo { name, csrf, avatar })
        })
        .collect();
    
    println!("Reloading accounts from Tauri Store: {} accounts found", accounts_data.len());
    for acc in &accounts_data {
        println!("  - {}", acc.name);
    }
    
    // 更新内存中的账号列表
    *http_server.state.accounts.write().await = accounts_data;
    
    println!("Accounts reloaded successfully");
    Ok(())
}

#[tauri::command]
async fn start_http_server(
    app: AppHandle,
    http_server: State<'_, HttpServerHandle>,
) -> Result<(), String> {
    // 使用 Tauri Store 读取配置
    let store = app.store("settings.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;
    
    let port = store.get("http_server_port")
        .and_then(|v| v.as_u64())
        .unwrap_or(3737) as u16;
    
    let api_key = store.get("http_server_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| String::new());
    
    let accounts_data = store.get("accounts")
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let obj = item.as_object()?;
                    let name = obj.get("name")?.as_str()?.to_string();
                    let csrf = obj.get("csrf")?.as_str()?.to_string();
                    let avatar = obj.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string());
                    Some(AccountInfo { name, csrf, avatar })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    
    // 更新 state
    *http_server.state.api_key.write().await = api_key.clone();
    *http_server.state.accounts.write().await = accounts_data;
    
    // 停止旧服务器
    let mut handle_lock = http_server.handle.write().await;
    if let Some(old_handle) = handle_lock.take() {
        old_handle.abort();
    }
    
    // 创建自定义路由
    let state_clone = http_server.state.clone();
    let app_clone = app.clone();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let combined_state = HttpServerState {
        server_state: state_clone,
        app_handle: app_clone,
    };
    
    let router = Router::new()
        .route("/health", get(handle_health))
        .route("/accounts", get(handle_get_accounts))
        .route("/sync", post(handle_sync_csrf))
        .layer(cors)
        .with_state(combined_state);
    
    // 启动新服务器
    let handle = tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                println!("HTTP Server listening on {}", addr);
                if let Err(e) = axum::serve(listener, router).await {
                    eprintln!("HTTP Server error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to bind HTTP Server: {}", e);
            }
        }
    });
    
    *handle_lock = Some(handle);
    
    Ok(())
}

#[tauri::command]
async fn stop_http_server(http_server: State<'_, HttpServerHandle>) -> Result<(), String> {
    let mut handle_lock = http_server.handle.write().await;
    if let Some(handle) = handle_lock.take() {
        handle.abort();
        Ok(())
    } else {
        Err("HTTP Server is not running".to_string())
    }
}

#[tauri::command]
async fn restart_http_server(
    app: AppHandle,
    http_server: State<'_, HttpServerHandle>,
) -> Result<(), String> {
    stop_http_server(http_server.clone()).await.ok();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    start_http_server(app, http_server).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_server_state = Arc::new(ServerState {
        api_key: Arc::new(RwLock::new(String::new())),
        accounts: Arc::new(RwLock::new(Vec::new())),
    });
    
    let http_server_handle = HttpServerHandle {
        handle: Arc::new(RwLock::new(None)),
        state: http_server_state,
    };
    
    tauri::Builder::default()
        .manage(ProcessManager(Arc::new(Mutex::new(HashMap::new()))))
        .manage(http_server_handle)
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
            is_running,
            start_http_server,
            stop_http_server,
            restart_http_server,
            reload_accounts
        ])
        .setup(|app| {
            // 自动启动 HTTP Server
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 等待一下确保应用完全启动
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                // 使用 Tauri Store 读取配置
                if let Ok(store) = app_handle.store("settings.json") {
                    let enabled = store.get("http_server_enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    
                    if enabled {
                        println!("Auto-starting HTTP Server...");
                        let http_server = app_handle.state::<HttpServerHandle>();
                        if let Err(e) = start_http_server(app_handle.clone(), http_server).await {
                            eprintln!("Failed to auto-start HTTP Server: {}", e);
                        } else {
                            println!("HTTP Server auto-started successfully");
                        }
                    }
                }
            });
            
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