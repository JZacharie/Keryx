mod domain;
mod application;
mod infrastructure;
mod interfaces;

use axum::{
    http::{header, HeaderValue, Method, StatusCode},
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::infrastructure::auth::verifier::JwtVerifier;
use crate::infrastructure::docker::ContainerManager;
use crate::infrastructure::repositories::s3_job_repository::S3JobRepository;
use crate::domain::ports::job_repository::JobRepository;
use crate::interfaces::http::middleware::auth::auth_middleware;
use crate::interfaces::http::job_handlers::create_job_handler;

#[derive(Clone)]
pub struct AppState {
    pub jwt_verifier: Arc<JwtVerifier>,
    pub job_repository: Arc<dyn JobRepository>,
    pub container_manager: Arc<ContainerManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let public_key_pem = std::env::var("JWT_PUBLIC_KEY")
        .unwrap_or_else(|_| include_str!("../test_public_key.pem").to_string());

    let jwt_verifier = Arc::new(JwtVerifier::new(&public_key_pem)?);

    let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "keryx-jobs".to_string());
    let s3_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&s3_config);
    let job_repository = Arc::new(S3JobRepository::new(s3_client, s3_bucket));

    let docker_network = std::env::var("DOCKER_NETWORK")
        .unwrap_or_else(|_| "keryx_keryx-net".to_string());
    let container_manager = Arc::new(ContainerManager::new(&docker_network)?);
    tracing::info!("ContainerManager initialized on network: {}", docker_network);

    let state = AppState {
        jwt_verifier,
        job_repository,
        container_manager,
    };

    let router = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/health", get(interfaces::http::health::health_check))
        .nest(
            "/api",
            Router::new()
                .route("/secure-ping", get(interfaces::http::health::health_check))
                .route("/jobs", post(create_job_handler))
                .layer(from_fn_with_state(state.jwt_verifier.clone(), auth_middleware)),
        )
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        CorsMakeService {
            inner: router.into_make_service(),
        },
    )
    .await?;

    Ok(())
}

struct CorsMakeService<S> {
    inner: S,
}

impl<S, Target> Service<Target> for CorsMakeService<S>
where
    S: Service<Target>,
    S::Future: Send + 'static,
{
    type Response = CorsService<S::Response>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, target: Target) -> Self::Future {
        let fut = self.inner.call(target);
        Box::pin(async move {
            let inner_svc = fut.await?;
            Ok(CorsService { inner: inner_svc })
        })
    }
}

impl<S: Clone> Clone for CorsMakeService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Clone)]
struct CorsService<S> {
    inner: S,
}

impl<S, ResBody> Service<axum::http::Request<axum::body::Body>> for CorsService<S>
where
    S: Service<axum::http::Request<axum::body::Body>, Response = axum::http::Response<ResBody>>,
    S::Future: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = axum::http::Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<axum::body::Body>) -> Self::Future {
        let is_options = req.method() == Method::OPTIONS;
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        if is_options {
            let mut resp = axum::http::Response::new(ResBody::default());
            *resp.status_mut() = StatusCode::OK;
            set_cors_headers(resp.headers_mut(), &origin);
            return Box::pin(async { Ok(resp) });
        }

        let fut = self.inner.call(req);
        Box::pin(async move {
            let mut resp = fut.await?;
            set_cors_headers(resp.headers_mut(), &origin);
            Ok(resp)
        })
    }
}

fn set_cors_headers(headers: &mut axum::http::HeaderMap, origin: &Option<String>) {
    let allowed_origin = origin
        .as_deref()
        .and_then(|o| HeaderValue::from_str(o).ok())
        .unwrap_or(HeaderValue::from_static("*"));

    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, allowed_origin);
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, Authorization, X-Requested-With"),
    );
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
}
