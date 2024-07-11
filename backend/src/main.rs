use std::{collections::HashMap, sync::Arc};

use axum::{
    http::Uri,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use steam::{LoginUser, SteamShortcut};
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

include!(concat!(env!("OUT_DIR"), "/index_html.rs"));

mod models;
mod routes;
mod steam;
mod vendor;

#[derive(Clone)]
pub struct SharedAppState {
    pub app_info: Arc<vendor::vdfr::AppInfo>,
    pub steam_users: Arc<HashMap<u64, LoginUser>>,
    pub users_shortcuts: Arc<HashMap<u64, HashMap<u32, SteamShortcut>>>,
}

#[tokio::main]
async fn main() {
    // fallback to current working directory
    let decky_log_path = std::env::var("DECKY_LOG_INTO").unwrap_or_else(|_| {
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });

    // this is folder
    let decky_log_path = dunce::canonicalize(decky_log_path).unwrap();
    if !decky_log_path.is_dir() {
        tracing::error!("💥 DECKY_LOG_INTO is not a directory: {:?}", decky_log_path);
        std::process::exit(1);
    }

    let log_file = tracing_appender::rolling::daily(decky_log_path, "backend.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    let version = env!("CARGO_PKG_VERSION");
    tracing::info!("📸 Deck Screenshot Viewer v{}", version);

    let steam_root = dunce::canonicalize(steam::get_steam_root_path()).unwrap();
    tracing::info!("Steam root path: {:?}", steam_root);

    let app_info_path = steam_root.join("appcache/appinfo.vdf");
    if !app_info_path.exists() {
        tracing::error!("💥 appinfo.vdf not found at {:?}", app_info_path);
        std::process::exit(1);
    }

    tracing::info!("Loading appinfo.vdf from {:?}", app_info_path);
    let app_info_reader = std::fs::read(app_info_path).unwrap();
    let app_info = match vendor::vdfr::AppInfo::load(&app_info_reader) {
        Ok(app_info) => app_info,
        Err(e) => {
            tracing::error!("💥 Failed to load appinfo.vdf: {}", e);
            std::process::exit(1);
        }
    };

    drop(app_info_reader);

    let app_info = Arc::new(app_info);
    tracing::info!("Loaded {} apps", app_info.apps.len());
    tracing::info!("Loading registered users...");
    let steam_users = Arc::new(steam::get_steam_users(steam_root));
    tracing::info!("Loaded {} users", steam_users.len());

    // load shortcuts of each users
    let mut users_shortcuts = HashMap::new();
    for user_id in steam_users.keys() {
        let uid3 = steam::steamid64_to_usteamid(*user_id);
        tracing::info!(
            " Loading shortcuts/non-steam apps for user {} ({})",
            user_id,
            uid3
        );
        let shortcuts = steam::load_users_shortcuts(uid3);
        tracing::info!(" Loaded {} shortcuts/non-steam apps", shortcuts.len());
        users_shortcuts.insert(uid3, shortcuts);
    }

    let state = SharedAppState {
        app_info,
        steam_users,
        users_shortcuts: Arc::new(users_shortcuts),
    };

    let decky_plugin_dir = std::env::var("DECKY_PLUGIN_DIR");
    tracing::info!("Decky plugin dir: {:?}", decky_plugin_dir);
    let assets_dir = match decky_plugin_dir {
        Ok(dir) => ServeDir::new(format!("{}/assets/assets", dir)),
        _ => ServeDir::new("assets/assets"),
    };

    let app: Router = Router::new()
        .route("/", get(index))
        .route(
            "/favicon.ico",
            get(|| async { include_bytes!("../../defaults/assets/favicon.ico").to_vec() }),
        )
        .route("/_/health", get(|| async { "ok" }))
        .nest("/api", routes::api_routes(state.clone()))
        .nest_service("/assets", assets_dir)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any))
        .with_state(state);

    let app = app.fallback(handle_404);

    let host_at = std::env::var("HOST").unwrap_or("127.0.0.1".to_string());
    let port_at = std::env::var("PORT").unwrap_or("5158".to_string());

    // run it
    let listener = TcpListener::bind(format!("{}:{}", host_at, port_at))
        .await
        .unwrap();

    tracing::info!(
        "🚀 Fast serving at: http://{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn handle_404(url: Uri) -> Redirect {
    let path = url.to_string();
    tracing::info!("404: {:?}", url);

    let redirect_url = format!("/?redirect={}", urlencoding::encode(&path));
    Redirect::to(&redirect_url)
}

async fn index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>(); // stub!

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl-C, shutting down...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down...");
        }
    }
}
