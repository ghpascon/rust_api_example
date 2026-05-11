use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::Serialize;

use crate::{
    models::tag::{TagInput, TagRecord},
    services::tag_list::{TagList, TagListError},
};

pub fn app(tag_list: Arc<TagList>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/tags", post(add_tag).get(get_all_tags))
        .route(
            "/api/tags/{identifier}",
            get(get_tag_by_id).delete(remove_tag),
        )
        .route("/api/tags/by-epc/{epc}", get(get_tag_by_epc))
        .route("/api/tags/by-tid/{tid}", get(get_tag_by_tid))
        .route(
            "/api/tags/before/{timestamp_ms}",
            delete(remove_before_timestamp),
        )
        .with_state(tag_list)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn add_tag(
    State(tag_list): State<Arc<TagList>>,
    Json(payload): Json<TagInput>,
) -> Result<(StatusCode, Json<ApiResponse<TagRecord>>), ApiError> {
    let created = tag_list.add(payload).await.map_err(ApiError::bad_request)?;
    Ok((StatusCode::CREATED, Json(ApiResponse { data: created })))
}

async fn get_all_tags(State(tag_list): State<Arc<TagList>>) -> Json<ApiResponse<Vec<TagRecord>>> {
    let tags = tag_list.get_all().await;
    Json(ApiResponse { data: tags })
}

async fn get_tag_by_id(
    State(tag_list): State<Arc<TagList>>,
    Path(identifier): Path<String>,
) -> Result<Json<ApiResponse<TagRecord>>, ApiError> {
    let tag = tag_list
        .get_by_id(&identifier)
        .await
        .ok_or_else(|| ApiError::not_found("tag not found"))?;
    Ok(Json(ApiResponse { data: tag }))
}

async fn get_tag_by_epc(
    State(tag_list): State<Arc<TagList>>,
    Path(epc): Path<String>,
) -> Result<Json<ApiResponse<TagRecord>>, ApiError> {
    let tag = tag_list
        .get_by_epc(&epc)
        .await
        .ok_or_else(|| ApiError::not_found("tag not found for epc"))?;
    Ok(Json(ApiResponse { data: tag }))
}

async fn get_tag_by_tid(
    State(tag_list): State<Arc<TagList>>,
    Path(tid): Path<String>,
) -> Result<Json<ApiResponse<TagRecord>>, ApiError> {
    let tag = tag_list
        .get_by_tid(&tid)
        .await
        .ok_or_else(|| ApiError::not_found("tag not found for tid"))?;
    Ok(Json(ApiResponse { data: tag }))
}

async fn remove_tag(
    State(tag_list): State<Arc<TagList>>,
    Path(identifier): Path<String>,
) -> Result<Json<ApiResponse<TagRecord>>, ApiError> {
    let removed = tag_list
        .remove(&identifier)
        .await
        .ok_or_else(|| ApiError::not_found("tag not found"))?;
    Ok(Json(ApiResponse { data: removed }))
}

async fn remove_before_timestamp(
    State(tag_list): State<Arc<TagList>>,
    Path(timestamp_ms): Path<u128>,
) -> Json<ApiResponse<RemoveBeforeTimestampResponse>> {
    let removed = tag_list.remove_before_timestamp(timestamp_ms).await;
    Json(ApiResponse {
        data: RemoveBeforeTimestampResponse {
            removed_count: removed.len(),
            removed,
        },
    })
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct RemoveBeforeTimestampResponse {
    removed_count: usize,
    removed: Vec<TagRecord>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(error: TagListError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: error.to_string(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::app;
    use crate::services::tag_list::TagList;

    #[tokio::test]
    async fn api_supports_create_and_lookup() {
        let app = app(Arc::new(TagList::new()));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "epc": "abcd1234",
                            "tid": "00112233445566778899aabb",
                            "ant": 1,
                            "rssi": -55
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tags/by-epc/ABCD1234")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let value: Value = serde_json::from_slice(&body).expect("body should be valid json");

        assert_eq!(
            value["data"]["identifier"],
            Value::String("00112233445566778899AABB".into())
        );
        assert_eq!(value["data"]["epc"], Value::String("ABCD1234".into()));
    }
}
