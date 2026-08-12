// Manner_Web - 可以在 Linux 系统上运行的企业管理系统
// Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

pub async fn request_logging_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, axum::response::Response> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();

    tracing::info!(method = %method, path = %path, "Request started");

    let response = next.run(req).await;

    let status = response.status();
    let duration = start.elapsed();
    tracing::info!(
        method = %method,
        path = %path,
        status = status.as_u16(),
        duration_ms = duration.as_millis() as u64,
        "Request completed"
    );

    Ok(response)
}

/// 安全加固中间件（最外层）：
/// 1. 将 axum 的 422 反序列化错误（含 serde 字段名/类型等内部结构信息）统一替换为
///    通用错误消息，避免未认证攻击者通过错误消息逆向接口字段契约。
/// 2. 将 axum 的 400（JSON 解析失败）/ 415（Content-Type 不符）纯文本错误统一替换为
///    通用错误消息——其原始文案（"Failed to parse the request body as JSON: ..."、
///    "Expected request with Content-Type: application/json"）会泄露 Rust serde_json /
///    axum 技术栈指纹（F-05）。业务侧 400 错误（AppError::BadRequest）均为
///    application/json 响应，不受影响。
/// 3. 统一附加 X-Content-Type-Options: nosniff 与 X-Frame-Options: DENY。
/// （`allow` 头的剥离见 `strip_allow` 模块——它由 axum 顶层 RouteFuture 注入，本中间件无法拦截。）
pub async fn harden_response_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, axum::response::Response> {
    let mut response = next.run(req).await;

    let is_json_body = |resp: &Response| {
        resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    };

    let is_framework_parse_error = response.status() == StatusCode::UNPROCESSABLE_ENTITY
        || ((response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE)
            && !is_json_body(&response));

    if is_framework_parse_error {
        let generic = crate::utils::response::ApiResponse::<()>::error(40000, "请求参数格式错误");
        let (mut parts, _) = response.into_parts();
        if parts.status == StatusCode::UNPROCESSABLE_ENTITY {
            parts.status = StatusCode::BAD_REQUEST;
        }
        parts.headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        let body = axum::body::Body::from(
            serde_json::to_vec(&generic).unwrap_or_else(|_| b"{}".to_vec()),
        );
        response = Response::from_parts(parts, body);
    }

    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::X_FRAME_OPTIONS,
        header::HeaderValue::from_static("DENY"),
    );
    // F5: HSTS —— 仅在生产 HTTPS 场景有意义（HTTP 下浏览器忽略该头，无副作用）。
    // 生产反代也应配置同值 HSTS（见 docs/nginx-prod.conf.example）。
    response.headers_mut().insert(
        header::STRICT_TRANSPORT_SECURITY,
        header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    // F5: 引用策略 —— 防止登录页/会话 URL 泄漏到第三方站点的 Referer。
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("no-referrer"),
    );
    // F5: 禁止跨域文档策略 —— 防止 IE/老浏览器将该站点文档当作跨域文件加载。
    response.headers_mut().insert(
        header::HeaderName::from_static("x-permitted-cross-domain-policies"),
        header::HeaderValue::from_static("none"),
    );

    Ok(response)
}
