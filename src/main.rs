use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{net::SocketAddr, time::SystemTime};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct Chore {
    id: usize,
    chore_name: String,
    frequency: Option<usize>,
    last_completed_at: Option<SystemTime>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct ChoreResponse {
    chores: Vec<Chore>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let connection_options = SqliteConnectOptions::new()
        .filename("chores")
        .create_if_missing(true);
    let connection_pool = SqlitePool::connect_with(connection_options).await?;

    sqlx::migrate!("./migrations").run(&connection_pool).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;

    let app = Router::new().route("/{id}/mark-complete", post(mark_complete));

    println!("listening on {addr}");
    _ = axum::serve(listener, app).await;
    Ok(())
}

async fn mark_complete(Path(id): Path<String>) -> Result<Json<ChoreResponse>, AppError> {
    Ok(Json(ChoreResponse { chores: vec![] }))
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
