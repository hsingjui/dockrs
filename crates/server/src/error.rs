use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

/// 统一 API 成功响应。
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub data: T,
    pub msg: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            data,
            msg: "操作成功".to_owned(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// 统一 API 错误：保留 HTTP status，并返回 `{ code, data, msg }` 结构。
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    msg: String,
}

impl ApiError {
    fn new(status: StatusCode, msg: impl Into<String>) -> Self {
        Self {
            status,
            msg: msg.into(),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, msg)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, msg)
    }

    pub fn method_not_allowed(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::METHOD_NOT_ALLOWED, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = ApiResponse {
            code: status.as_u16(),
            data: json!({}),
            msg: self.msg,
        };
        (status, Json(body)).into_response()
    }
}

/// 将 Axum 的 JSON 提取失败转换为统一错误响应，避免框架默认返回格式泄漏到 API。
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(req, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|err| {
                tracing::debug!(error = %err, "解析 JSON 请求失败");
                if matches!(err, JsonRejection::BytesRejection(_)) {
                    ApiError::internal("读取请求失败")
                } else {
                    ApiError::bad_request("请求参数格式错误")
                }
            })
    }
}

pub async fn not_found() -> ApiError {
    ApiError::not_found("请求资源不存在")
}

pub async fn method_not_allowed() -> ApiError {
    ApiError::method_not_allowed("请求方法不被允许")
}
