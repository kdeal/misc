use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use crate::repositories::get_repository_names_in_directory;
use anyhow::Context;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

const SERVER_PORT: u16 = 41000;

#[derive(Clone)]
struct ServerState {
    repositories_directory: PathBuf,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct RepositoriesResponse {
    base_directory: String,
    repositories: Vec<String>,
}

pub fn server_port_for_directory(directory: &Path) -> u16 {
    let _ = directory;
    SERVER_PORT
}

pub fn server_addr_for_directory(directory: &Path) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], server_port_for_directory(directory)))
}

pub fn app(repositories_directory: PathBuf) -> Router {
    Router::new()
        .route("/repositories", get(list_repositories))
        .with_state(ServerState {
            repositories_directory,
        })
}

pub async fn serve(repositories_directory: PathBuf) -> anyhow::Result<()> {
    let addr = server_addr_for_directory(&repositories_directory);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind server to {addr}"))?;

    println!("Serving repositories at http://{addr}/repositories");

    axum::serve(listener, app(repositories_directory))
        .await
        .context("Repository server exited unexpectedly")
}

async fn list_repositories(
    State(state): State<ServerState>,
) -> Result<Json<RepositoriesResponse>, (axum::http::StatusCode, String)> {
    let repositories_directory = state.repositories_directory;
    let repositories_directory_display = repositories_directory.display().to_string();
    let repositories = tokio::task::spawn_blocking(move || {
        get_repository_names_in_directory(&repositories_directory)
    })
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to join repository task: {error}"),
        )
    })
    .and_then(|result| {
        result.map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list repositories: {error}"),
            )
        })
    })?;

    Ok(Json(RepositoriesResponse {
        base_directory: repositories_directory_display,
        repositories,
    }))
}

#[cfg(test)]
mod tests {
    use super::{app, server_port_for_directory, SERVER_PORT};
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn server_port_is_fixed() {
        let port_one = server_port_for_directory(std::path::Path::new("/tmp/repos-a"));
        let port_two = server_port_for_directory(std::path::Path::new("/tmp/repos-a"));
        let port_three = server_port_for_directory(std::path::Path::new("/tmp/repos-b"));

        assert_eq!(port_one, port_two);
        assert_eq!(port_one, port_three);
        assert_eq!(port_one, SERVER_PORT);
    }

    #[tokio::test]
    async fn repositories_endpoint_returns_repository_list() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        fs::create_dir_all(temp_dir.path().join("alpha/.git")).expect("failed to create git repo");
        fs::create_dir_all(temp_dir.path().join("nested/beta/.jj"))
            .expect("failed to create jj repo");

        let response = app(temp_dir.path().to_path_buf())
            .oneshot(
                Request::builder()
                    .uri("/repositories")
                    .body(Body::empty())
                    .expect("failed to build request"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body");
        let response_body: serde_json::Value =
            serde_json::from_slice(&body).expect("failed to parse json");

        assert_eq!(
            response_body,
            serde_json::json!({
                "base_directory": temp_dir.path().display().to_string(),
                "repositories": ["alpha", "nested/beta"]
            })
        );
    }
}
