//! HTTP endpoints: advertisement, lockfile, package bytes.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::boot::Booted;

const JSON: &str = "application/json";
const BYTES: &str = "application/octet-stream";

/// Bind `listen` and serve on a new thread with its own tokio runtime. The socket is bound
/// before the call returns, so a connection made after the return waits in the backlog until
/// the thread accepts it. A bind failure panics with the address.
pub fn spawn_http(
    listen: SocketAddr,
    booted: Arc<Booted>,
    static_dir: Option<PathBuf>,
) -> JoinHandle<()> {
    let socket = std::net::TcpListener::bind(listen)
        .unwrap_or_else(|error| panic!("cannot bind the HTTP listener on {listen}: {error}"));
    socket
        .set_nonblocking(true)
        .expect("the HTTP listener becomes non-blocking");
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("the tokio runtime starts");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(socket)
                .expect("the HTTP listener joins the runtime");
            axum::serve(listener, router(booted, static_dir))
                .await
                .expect("the HTTP server runs");
        });
    })
}

fn router(booted: Arc<Booted>, static_dir: Option<PathBuf>) -> Router {
    let router = Router::new()
        .route("/advertisement.json", get(advertisement))
        .route("/lockfile.json", get(lockfile))
        .route("/packages/{name}", get(package))
        .with_state(booted);
    let router = match static_dir {
        Some(dir) => router.fallback_service(ServeDir::new(dir).precompressed_br()),
        None => router,
    };
    router.layer(CorsLayer::permissive())
}

async fn advertisement(State(booted): State<Arc<Booted>>) -> Response {
    ([(header::CONTENT_TYPE, JSON)], booted.advertisement.clone()).into_response()
}

async fn lockfile(State(booted): State<Arc<Booted>>) -> Response {
    ([(header::CONTENT_TYPE, JSON)], booted.lockfile.clone()).into_response()
}

async fn package(State(booted): State<Arc<Booted>>, Path(name): Path<String>) -> Response {
    if name == booted.package_file_name {
        (
            [(header::CONTENT_TYPE, BYTES)],
            booted.package_bytes.clone(),
        )
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
