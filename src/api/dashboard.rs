use actix_web::{HttpResponse, Result as ActixResult, get};

#[get("/")]
pub async fn dashboard_redirect() -> ActixResult<HttpResponse> {
    // Redirect to the React dashboard (built from frontend/, served as static files)
    Ok(HttpResponse::Found()
        .append_header(("Location", "/static/app/"))
        .finish())
}
