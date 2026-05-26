// crates/gateway/src/routes.rs
// HTTP routes for public software registry

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::softwares::{Software, SoftwareRegistry};

#[derive(Clone)]
pub struct AppState {
    pub registry: SoftwareRegistry,
}

pub fn software_routes() -> Router<AppState> {
    Router::new()
        .route("/softwares", get(list_softwares))
        .route("/softwares/:name", get(get_software))
        .route("/softwares/category/:category", get(list_by_category))
}

/// GET /softwares — list all public software
pub async fn list_softwares(
    State(state): State<AppState>,
) -> (StatusCode, Json<Vec<Software>>) {
    let softwares = state.registry.list();
    (StatusCode::OK, Json(softwares))
}

/// GET /softwares/:name — get specific software metadata
pub async fn get_software(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.registry.get(&name) {
        Some(software) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": software
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Software '{}' not found", name)
            })),
        ),
    }
}

/// GET /softwares/category/:category — list by category
pub async fn list_by_category(
    Path(category): Path<String>,
    State(state): State<AppState>,
) -> (StatusCode, Json<Vec<Software>>) {
    let softwares = state.registry.by_category(&category);
    (StatusCode::OK, Json(softwares))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_software_routes() {
        let state = AppState {
            registry: SoftwareRegistry::new(),
        };

        let softwares = state.registry.list();
        assert!(!softwares.is_empty());
    }
}
