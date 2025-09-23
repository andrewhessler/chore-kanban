use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, SqlitePool, sqlite::SqliteConnectOptions};
use std::net::SocketAddr;

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct Chore {
    id: usize,
    chore_name: String,
    frequency: Option<usize>,
    last_completed_at: Option<usize>,
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

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/index.html", get(index_handler))
        .route("/assets/{*file}", get(static_handler))
        .route("/get-chores", get(get_chores_handler))
        .route("/{id}/mark-complete", post(mark_complete_handler))
        .with_state(connection_pool);

    println!("listening on {addr}");
    _ = axum::serve(listener, app).await;
    Ok(())
}

async fn index_handler() -> impl IntoResponse {
    static_handler("/index.html".parse::<Uri>().unwrap()).await
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();

    StaticFile(path)
}

async fn get_chores_handler(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<ChoreResponse>, AppError> {
    let chores = get_chores(&pool).await?;
    Ok(Json(ChoreResponse { chores }))
}

#[derive(Deserialize, Clone)]
pub struct MarkCompleteRequest {
    pub clear_ticket: bool,
}

async fn mark_complete_handler(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    Json(request): Json<MarkCompleteRequest>,
) -> Result<Json<ChoreResponse>, AppError> {
    mark_complete(&id, &pool, request.clear_ticket).await?;

    let chores = get_chores(&pool).await?;
    Ok(Json(ChoreResponse { chores }))
}

async fn mark_complete(chore_id: &str, pool: &Pool<Sqlite>, clear: bool) -> anyhow::Result<()> {
    if clear {
        sqlx::query!(
            r"
        UPDATE chores SET last_completed_at = NULL WHERE id = ?1
        ",
            chore_id
        )
        .execute(pool)
        .await?;
    } else {
        sqlx::query!(
            r"
            UPDATE chores SET last_completed_at = unixepoch() WHERE id = ?1
            ",
            chore_id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn get_chores(pool: &Pool<Sqlite>) -> anyhow::Result<Vec<Chore>> {
    let records = sqlx::query!(
        r"
        SELECT * FROM chores
        ",
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .iter()
        .map(|record| Chore {
            id: record.id as usize,
            chore_name: record.display_name.clone(),
            frequency: record.frequency_hours.map(|val| val as usize),
            last_completed_at: record.last_completed_at.map(|val| val as usize),
        })
        .collect())
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

#[derive(Embed)]
#[folder = "src/client/dist/"]
struct Asset;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}
