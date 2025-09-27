use crate::models::Claims;
use crate::telemetry::Metrics;
use crate::{
    auth::Authenticator,
    constants::{API_VERSION, BEARER},
};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    http::header::AUTHORIZATION,
    Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    sync::Arc,
    time::Instant,
};

#[derive(Clone)]
pub struct JwtMiddleware {
    authenticator: Arc<Authenticator>,
}

impl JwtMiddleware {
    pub fn new(authenticator: Arc<Authenticator>) -> Self {
        Self { authenticator }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    #[inline(always)]
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareService {
            service: Arc::new(service),
            authenticator: self.authenticator.clone(),
        }))
    }
}

pub struct JwtMiddlewareService<S> {
    service: Arc<S>,
    authenticator: Arc<Authenticator>,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    #[inline(always)]
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let authenticator = self.authenticator.clone();

        Box::pin(async move {
            let auth_header = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|h| h.to_str().ok());

            if let Some(auth_str) = auth_header {
                if auth_str.starts_with(BEARER) {
                    let Some(token) = auth_str.strip_prefix(BEARER) else {
                        return Err(ErrorUnauthorized("Invalid token"));
                    };

                    match authenticator.validate_token(token) {
                        Ok(claims) => {
                            req.extensions_mut().insert(claims);
                            let res = service.call(req).await?;
                            return Ok(res);
                        }
                        Err(_) => {
                            return Err(ErrorUnauthorized("Invalid token"));
                        }
                    }
                }
            }

            Err(ErrorUnauthorized("Missing or invalid authorization header"))
        })
    }
}

#[inline(always)]
#[allow(dead_code)]
pub fn extract_claims(req: &actix_web::HttpRequest) -> Option<Claims> {
    req.extensions().get::<Claims>().cloned()
}

#[derive(Clone)]
pub struct MetricsMiddleware {
    metrics: Arc<Metrics>,
}

impl MetricsMiddleware {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
}

impl<S, B> Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = MetricsMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    #[inline(always)]
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MetricsMiddlewareService {
            service: Arc::new(service),
            metrics: self.metrics.clone(),
        }))
    }
}

pub struct MetricsMiddlewareService<S> {
    service: Arc<S>,
    metrics: Arc<Metrics>,
}

impl<S, B> Service<ServiceRequest> for MetricsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    #[inline(always)]
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let metrics = self.metrics.clone();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let start_time = Instant::now();

        metrics.active_connections.inc();

        Box::pin(async move {
            let res = service.call(req).await;

            let duration = start_time.elapsed().as_secs_f64();
            let status = match &res {
                Ok(response) => response.status().as_u16().to_string(),
                Err(_) => "500".to_string(),
            };

            metrics.active_connections.dec();
            metrics
                .http_requests_total
                .with_label_values(&[&method, &path, &status, &API_VERSION.to_owned()])
                .inc();
            metrics
                .http_request_duration
                .with_label_values(&[&method, &path, &status])
                .observe(duration);

            res
        })
    }
}
