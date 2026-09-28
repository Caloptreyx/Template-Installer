use axum::http::StatusCode;
use shared::{ApiError, response::ApiResponse};

pub mod admin;
pub mod server;

/// 400 response for garde validation failures.
fn validation_error(data: &impl garde::Validate<Context = ()>) -> Option<ApiResponse> {
    shared::utils::validate_data(data).err().map(bad_request)
}

fn bad_request(errors: Vec<String>) -> ApiResponse {
    ApiResponse::new_serialized(ApiError::new_strings_value(errors))
        .with_status(StatusCode::BAD_REQUEST)
}

fn not_found(what: &str) -> ApiResponse {
    ApiResponse::error(format!("{what} not found")).with_status(StatusCode::NOT_FOUND)
}
