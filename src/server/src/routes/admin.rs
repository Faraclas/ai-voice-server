use axum::response::{Html, IntoResponse};

pub async fn admin_ui_handler() -> impl IntoResponse {
    // We use include_str! to compile the HTML directly into the binary at compile time.
    // This adds absolutely zero IO overhead or memory footprint at runtime.
    Html(include_str!("admin.html"))
}
