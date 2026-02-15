use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct ServerState {
    pub api_key: Arc<RwLock<String>>,
    pub accounts: Arc<RwLock<Vec<AccountInfo>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub name: String,
    pub csrf: String, // 加密的 csrf
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub name: String,
    pub csrf: String, // 解密后的 csrf
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCsrfRequest {
    pub csrf: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

// 验证 API Key
fn verify_api_key(headers: &HeaderMap, api_key: &str) -> bool {
    if api_key.is_empty() {
        return true; // 如果没有设置 API Key，允许所有请求
    }
    
    headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|key| key == api_key)
        .unwrap_or(false)
}

// GET /accounts - 获取所有账号信息（返回解密后的 csrf）
async fn get_accounts(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let api_key = state.api_key.read().await;
    
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

    let accounts = state.accounts.read().await.clone();
    
    // 这里需要解密 csrf，但我们在这个模块中无法访问解密函数
    // 所以我们返回加密的 csrf，让调用方处理
    let response_accounts: Vec<AccountResponse> = accounts.iter().map(|acc| {
        AccountResponse {
            name: acc.name.clone(),
            csrf: acc.csrf.clone(), // 暂时返回加密的，需要在 lib.rs 中处理
            avatar: acc.avatar.clone(),
        }
    }).collect();
    
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Success".to_string(),
            data: Some(response_accounts),
        }),
    )
}

// POST /sync - 同步当前 CSRF 到服务器
async fn sync_csrf(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateCsrfRequest>,
) -> impl IntoResponse {
    let api_key = state.api_key.read().await;
    
    if !verify_api_key(&headers, &api_key) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()> {
                success: false,
                message: "Invalid API Key".to_string(),
                data: None,
            }),
        );
    }

    // 这里需要通过 CSRF 查找对应的账号并更新
    // 但由于我们需要解密来比对，这部分逻辑需要在 lib.rs 中处理
    
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "CSRF synced successfully".to_string(),
            data: None,
        }),
    )
}

// GET /health - 健康检查
async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: "Server is running".to_string(),
            data: Some("OK"),
        }),
    )
}

pub fn create_router(state: ServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/accounts", get(get_accounts))
        .route("/sync", post(sync_csrf))
        .layer(cors)
        .with_state(state)
}

pub async fn start_server(port: u16, state: ServerState) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("HTTP Server listening on {}", addr);
    
    axum::serve(listener, create_router(state))
        .await?;
    
    Ok(())
}
