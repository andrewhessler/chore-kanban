use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use dotenvy::dotenv;
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, SqlitePool, sqlite::SqliteConnectOptions};
use std::{env, net::SocketAddr};

const SECS_IN_DAY: u64 = 60 * 60 * 24;

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct Chore {
    id: i64,
    chore_name: String,
    overdue: bool,
    on_cadence: bool,
    days_until_overdue: Option<f64>,
    freq_secs: Option<i64>,
    last_completed_at: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct ChoreResponse {
    chores: Vec<Chore>,
}

#[derive(Clone, Debug)]
struct AppState {
    pub pool: Pool<Sqlite>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    let raw_database_url = env::var("DATABASE_URL").expect("DATABASE_URL to be defined");
    let database_url = raw_database_url.split(":").last().unwrap();

    let connection_options = SqliteConnectOptions::new()
        .filename(database_url)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(connection_options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/index.html", get(index_handler))
        .route("/assets/{*file}", get(static_handler))
        .route("/get-chores", get(get_chores_handler))
        .route("/{id}/toggle-chore", post(toggle_chore_handler))
        .with_state(AppState { pool });

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
    State(state): State<AppState>,
) -> Result<Json<ChoreResponse>, AppError> {
    let chores = get_chores(&state.pool).await?;
    Ok(Json(ChoreResponse { chores }))
}

async fn toggle_chore_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ChoreResponse>, AppError> {
    let chore = get_chore_by_id(&id, &state.pool).await?;
    let now = Utc::now().timestamp();

    // If frequency doesn't exist, we set it to overdue by giving it a tiny frequency
    // If frequency is the short circuit frequency (1 hour = 60 * 60 seconds), clear frequency and
    // set last_completed_at
    if chore.freq_secs.is_none() {
        sqlx::query!(
            r"
            UPDATE chores SET frequency_hours = ?1 WHERE id = ?2
            ",
            1,
            id
        )
        .execute(&state.pool)
        .await?;
    } else if chore.freq_secs.unwrap_or(0) == 60 * 60 {
        // else if is silly but they're connected!
        sqlx::query!(
            r"
            UPDATE chores SET last_completed_at = ?1, frequency_hours = NULL WHERE id = ?2
            ",
            now,
            id
        )
        .execute(&state.pool)
        .await?;
    }

    if chore.overdue {
        let last_completed_at = if chore.on_cadence {
            let freq_secs = chore
                .freq_secs
                .expect("A chore on a cadence must have a frequency");

            let mut new_last_completed_at = chore
                .last_completed_at
                .expect("A chore on a cadence must have a last_completed_at");

            while new_last_completed_at < now - freq_secs {
                new_last_completed_at += freq_secs
            }
            new_last_completed_at
        } else {
            now
        };
        // if overdue, update completed_at
        sqlx::query!(
            r"
            UPDATE chores SET last_completed_at = ?1 WHERE id = ?2
            ",
            last_completed_at,
            id
        )
        .execute(&state.pool)
        .await?;
    } else {
        let last_completed_at: i64 = if chore.on_cadence {
            let freq_secs = chore
                .freq_secs
                .expect("A chore on a cadence must have a frequency");

            let existing_last_completed_at = chore
                .last_completed_at
                .expect("A chore on a cadence must have a last_completed_at");

            let mut new_last_completed_at = existing_last_completed_at;

            while new_last_completed_at > now - freq_secs {
                new_last_completed_at -= freq_secs
            }
            new_last_completed_at
        } else {
            now
        };
        // if not overdue, null or revert completed_at so it's overdue
        sqlx::query!(
            r"
            UPDATE chores SET last_completed_at = ?1 WHERE id = ?2
            ",
            last_completed_at,
            id
        )
        .execute(&state.pool)
        .await?;
    }

    let chores = get_chores(&state.pool).await?;
    Ok(Json(ChoreResponse { chores }))
}

#[derive(sqlx::FromRow, Debug)]
struct ChoreRow {
    id: i64,
    display_name: String,
    frequency_hours: Option<i64>,
    on_cadence: i64,
    last_completed_at: Option<i64>,
}

async fn get_chores(pool: &Pool<Sqlite>) -> anyhow::Result<Vec<Chore>> {
    let records = sqlx::query_as!(
        ChoreRow,
        r"
        SELECT * FROM chores
        ",
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .iter()
        .map(|record| map_record_to_chore(record))
        .collect())
}

async fn get_chore_by_id(id: &str, pool: &Pool<Sqlite>) -> anyhow::Result<Chore> {
    let record = sqlx::query_as!(
        ChoreRow,
        r"
        SELECT * FROM chores WHERE id = ?
        ",
        id
    )
    .fetch_one(pool)
    .await?;

    return Ok(map_record_to_chore(&record));
}

fn map_record_to_chore(record: &ChoreRow) -> Chore {
    let freq_in_days: Option<f64> = if let Some(freq) = record.frequency_hours {
        Some(freq as f64 / 24.)
    } else {
        None
    };

    let days_since_last_complete: Option<f64> = if let Some(last) = record.last_completed_at {
        let now_secs = Utc::now().timestamp();
        Some((now_secs - last) as f64 / SECS_IN_DAY as f64)
    } else {
        None
    };

    let overdue_by_freq = if let (Some(days), Some(freq)) = (days_since_last_complete, freq_in_days)
    {
        days > freq
    } else {
        false
    };

    let overdue_by_freq_short_circuit = record.frequency_hours.unwrap_or(0) == 1;

    let overdue = overdue_by_freq || overdue_by_freq_short_circuit;

    let days_until_overdue =
        if let (Some(days), Some(freq)) = (days_since_last_complete, freq_in_days) {
            Some(freq - days)
        } else {
            None
        };

    Chore {
        id: record.id,
        chore_name: record.display_name.clone(),
        days_until_overdue,
        overdue,
        on_cadence: record.on_cadence == 1,
        freq_secs: record.frequency_hours.map(|v| v * 60 * 60),
        last_completed_at: record.last_completed_at,
    }
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
