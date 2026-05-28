use axum::response::{Html, IntoResponse};

pub async fn dashboard() -> impl IntoResponse {
    Html(include_str!("dashboard/index.html"))
}
