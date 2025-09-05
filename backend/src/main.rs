// src/main.rs
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;

use axum::{
    extract::{Json, Path, Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse},
    routing::{delete, get, post, put},
    Router,
};
use serde_json::{json, Value};
use tonic::{transport::Server, Request};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info};

mod protogen;
mod services;
mod storage;
mod types;

use protogen::{
    task_service_server::TaskServiceServer,
    user_service_server::UserServiceServer,
    *,
};
use protogen::task_service_server::TaskService;
use protogen::user_service_server::UserService;

use services::{TaskServiceImpl, UserServiceImpl};
use storage::Storage;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    tracing_subscriber::fmt::init();

    // Create storage with persistence
    let storage = Storage::with_persistence("data/storage.json", true);
    
    // Load existing data on startup
    storage.load_from_disk().await?;
    
    // Clone storage for both servers
    let grpc_storage = storage.clone();
    let http_storage = storage.clone();

    // Start gRPC server
    let grpc_handle = tokio::spawn(async move {
        start_grpc_server(grpc_storage.into()).await
    });

    // Start HTTP server
    let http_handle = tokio::spawn(async move {
        start_http_server(http_storage.into()).await
    });

    // Wait for both servers
    tokio::try_join!(grpc_handle, http_handle)?;

    Ok(())
}

async fn start_grpc_server(storage: Arc<Storage>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    
    let task_service = TaskServiceImpl::new(storage.clone());
    let user_service = UserServiceImpl::new(storage);

    info!("Starting gRPC server on {}", addr);

    Server::builder()
        .accept_http1(true)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods([Method::GET, Method::POST])
        )
        .layer(GrpcWebLayer::new())
        .add_service(tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(protogen::DESCRIPTOR_SET)
            .build()?)
        .add_service(TaskServiceServer::new(task_service))
        .add_service(UserServiceServer::new(user_service))
        .serve(addr)
        .await?;

    Ok(())
}

async fn start_http_server(storage: Arc<Storage>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = "0.0.0.0:3001".parse()?;

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/tasks", post(create_task))
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks/:id", get(get_task))
        .route("/api/tasks/:id", put(update_task))
        .route("/api/tasks/:id", delete(delete_task))
        .route("/api/tasks/bulk", put(bulk_update_tasks))
        .route("/api/tasks/analytics", get(get_task_analytics))
        .route("/api/users", post(create_user))
        .route("/api/users", get(list_users))
        .route("/api/users/:id", get(get_user))
        .route("/api/users/:id", put(update_user))
        .route("/api/users/:id", delete(delete_user))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh_token))
        .route("/api/auth/logout", post(logout))
        .route("/api/health", get(health_check))
        .with_state(storage)
        .layer(cors);

    info!("Starting HTTP server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// HTTP handlers
async fn create_task(
    State(storage): State<Arc<Storage>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let request = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    match service.create_task(Request::new(request)).await {
        Ok(res) => match serde_json::to_value(res.into_inner()) {
            Ok(json) => Json(json).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {}", e)).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn get_task(
    State(storage): State<Arc<Storage>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let request = protogen::GetTaskRequest { id, include_comments: true };

    match service.get_task(Request::new(request)).await {
        Ok(res) => match serde_json::to_value(res.into_inner()) {
            Ok(json) => Json(json).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {}", e)).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn list_tasks(
    State(storage): State<Arc<Storage>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let request = protogen::ListTasksRequest {
        page_size: params.get("page_size").and_then(|s| s.parse().ok()).unwrap_or(20),
        page_token: params.get("page_token").cloned().unwrap_or_default(),
        filter: None,
        sort: None,
    };

    match service.list_tasks(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn update_task(
    State(storage): State<Arc<Storage>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let mut request: protogen::UpdateTaskRequest = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    request.id = id;

    match service.update_task(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn delete_task(
    State(storage): State<Arc<Storage>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let request = protogen::DeleteTaskRequest { id, force: false };

    match service.delete_task(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn bulk_update_tasks(
    State(storage): State<Arc<Storage>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let request = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    match service.bulk_update_tasks(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn get_task_analytics(
    State(storage): State<Arc<Storage>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let service = TaskServiceImpl::new(storage);
    let request = protogen::GetTaskAnalyticsRequest {
        start_date: None,
        end_date: None,
        user_ids: vec![],
        group_by: params.get("group_by").cloned().unwrap_or_default(),
    };

    match service.get_task_analytics(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

// User handlers (similar pattern)

async fn create_user(
    State(storage): State<Arc<Storage>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);

    let request = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            format!("Invalid request: {}", e)
        ).into_response(),
    };

    let response = match service.create_user(tonic::Request::new(request)).await {
        Ok(res) => res,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("gRPC error: {}", e)
        ).into_response(),
    };

    match serde_json::to_value(response.into_inner()) {
        Ok(json) => Json(json).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Serialization error: {}", e)
        ).into_response(),
    }
}

async fn get_user(
    State(storage): State<Arc<Storage>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let request = protogen::GetUserRequest { id };

    match service.get_user(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn list_users(
    State(storage): State<Arc<Storage>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let request = protogen::ListUsersRequest {
        page_size: params.get("page_size").and_then(|s| s.parse().ok()).unwrap_or(20),
        page_token: params.get("page_token").cloned().unwrap_or_default(),
        role: 0,
        active_only: params.get("active_only").map_or(true, |v| v == "true"),
    };

    match service.list_users(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn update_user(
    State(storage): State<Arc<Storage>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let mut request: protogen::UpdateUserRequest = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    request.id = id;

    match service.update_user(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn delete_user(
    State(storage): State<Arc<Storage>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let request = protogen::DeleteUserRequest { id };

    match service.delete_user(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("gRPC error: {}", e)).into_response(),
    }
}

async fn login(
    State(storage): State<Arc<Storage>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let request = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    match service.login(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::UNAUTHORIZED, format!("Login failed: {}", e)).into_response(),
    }
}

async fn refresh_token(
    State(storage): State<Arc<Storage>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let request = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    match service.refresh_token(Request::new(request)).await {
        Ok(res) => Json(serde_json::to_value(res.into_inner()).unwrap()).into_response(),
        Err(e) => (StatusCode::UNAUTHORIZED, format!("Token refresh failed: {}", e)).into_response(),
    }
}

async fn logout(
    State(storage): State<Arc<Storage>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let service = UserServiceImpl::new(storage);
    let request: protogen::LogoutRequest = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response(),
    };

    match service.logout(Request::new(request)).await {
        Ok(_) => Json(json!({ "success": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Logout failed: {}", e)).into_response(),
    }
}


async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "healthy": true,
        "version": "0.1.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}