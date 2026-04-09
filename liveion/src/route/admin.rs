use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};

use crate::AppState;
use crate::error::AppError;
use crate::result::Result;

pub fn route() -> Router<AppState> {
    Router::new().route(&api::path::cascade("{stream}"), post(cascade))
}

async fn cascade(
    State(state): State<AppState>,
    Path(stream): Path<String>,
    Json(body): Json<api::request::Cascade>,
) -> Result<String> {
    let api::request::Cascade {
        source_url,
        target_url,
        token,
    } = body;

    match (source_url, target_url) {
        (None, None) => {
            return Err(AppError::throw(
                "src and dst cannot be empty at the same time",
            ));
        }
        (Some(_), Some(_)) => {
            return Err(AppError::throw(
                "src and dst cannot be non-empty at the same time",
            ));
        }
        (Some(source_url), None) => {
            state
                .stream_manager
                .cascade_pull(stream, source_url, token)
                .await?;
        }
        (None, Some(target_url)) => {
            state
                .stream_manager
                .cascade_push(stream, target_url, token)
                .await?;
        }
    }
    Ok("".to_string())
}
