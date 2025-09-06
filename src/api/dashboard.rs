use actix_web::{HttpResponse, Result as ActixResult, get};

#[get("/")]
pub async fn dashboard_redirect() -> ActixResult<HttpResponse> {
    // Redirect to the static dashboard HTML file
    Ok(HttpResponse::Found()
        .append_header(("Location", "/static/dashboard.html"))
        .finish())
}
