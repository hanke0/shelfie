use shelfie_backend::api::router::build_router;
use shelfie_backend::config::Config;
use shelfie_backend::domain::auth::ensure_default_admin;
use shelfie_backend::infra::db;
use shelfie_backend::state::AppState;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_version() {
    println!("shelfie-backend {VERSION}");
    #[cfg(feature = "embed-frontend")]
    println!("  + embed-frontend");
}

fn version_flag_requested() -> bool {
    std::env::args().any(|a| a == "--version" || a == "-V")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if version_flag_requested() {
        print_version();
        return Ok(());
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shelfie_backend=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let pool = db::connect(&config).await?;
    let state = AppState::new(pool, config.clone());
    ensure_default_admin(&state).await?;

    let app = build_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Shelfie listening on {addr}");
    #[cfg(feature = "embed-frontend")]
    tracing::info!("Embedded frontend static files enabled (SPA fallback)");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
