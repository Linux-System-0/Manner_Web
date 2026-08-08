use axum::http::{Request, header};
use axum::response::Response;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

/// 在 axum 顶层 RouteFuture 之后剥离 `allow` 响应头。
///
/// 背景：`allow` 头由 axum 的顶层 RouteFuture 在全部 `.layer()` 中间件返回之后统一注入
/// （用于 405 与 OPTIONS 预检响应）。因此普通中间件无法拦截，必须在服务层包裹整个
/// Router 服务，才能在所有响应上移除该头，消除未认证路由/方法枚举依据。
#[derive(Clone)]
pub struct StripAllowHeader<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for StripAllowHeader<S>
where
    S: Service<Request<B>, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let mut response = inner.call(req).await?;
            response.headers_mut().remove(header::ALLOW);
            Ok(response)
        })
    }
}

/// MakeService 包装：为每个连接产出的 Router 服务套上 [`StripAllowHeader`]。
#[derive(Clone)]
pub struct StripAllowMakeService<M> {
    inner: M,
}

impl<M> StripAllowMakeService<M> {
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<M, T, S> Service<T> for StripAllowMakeService<M>
where
    M: Service<T, Response = S, Error = Infallible> + Clone + Send + 'static,
    M::Future: Send + 'static,
    S: Service<Request<axum::body::Body>, Response = Response, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = StripAllowHeader<S>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, target: T) -> Self::Future {
        let mut inner = self.inner.clone();
        let fut = inner.call(target);
        Box::pin(async move {
            let service = fut.await?;
            Ok(StripAllowHeader { inner: service })
        })
    }
}
