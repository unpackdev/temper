use std::{env, sync::Arc, path::Path};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use warp::Filter;
use dashmap::DashMap;
use enso_temper::{
    config::config, errors::handle_rejection, simulate_routes, SharedSimulationState,
};

#[tokio::main]
async fn main() {
    // Set RUST_LOG environment variable if not already set
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "ts::api=info");
    }
    pretty_env_logger::init(); // Initializes logging

    let config = config(); // Load the application config
    let api_key = config.clone().api_key;

    // Base API route setup
    let api_base = warp::path("api").and(warp::path("v1"));

    // Add API key protection if configured
    let api_base = if let Some(api_key) = api_key {
        log::info!(target: "ts::api", "Running with API key protection");
        let api_key_filter = warp::header::exact("X-API-KEY", Box::leak(api_key.into_boxed_str()));
        api_base.and(api_key_filter).boxed()
    } else {
        api_base.boxed()
    };

    // Shared state for both the HTTP and UDS servers
    let shared_state = Arc::new(SharedSimulationState {
        evms: Arc::new(DashMap::new()),
    });

    // Define Warp routes
    let routes = api_base
        .and(simulate_routes(config.clone(), shared_state.clone()))
        .recover(handle_rejection) // Handle rejection errors
        .with(warp::log("ts::api")); // Enable logging

    // Get HTTP port from configuration
    let http_port = config.port;

    // Define Unix Domain Socket path
    let uds_path = config.clone().uds_path;
    let uds_file = Path::new(uds_path.as_deref().expect("UDS_PATH must be set"));


    // Clean up any existing socket file from previous runs
    if uds_file.exists() {
        std::fs::remove_file(uds_file).expect("Failed to remove existing socket file");
    }

    // Task to run the HTTP server
    let http_server = warp::serve(routes.clone())
        .run(([0, 0, 0, 0], http_port));

    // Task to run the Unix Domain Socket server
    let uds_listener = UnixListener::bind(uds_file).expect("Failed to bind Unix Domain Socket");

    let uds_server = warp::serve(routes)
        .run_incoming(UnixListenerStream::new(uds_listener));

    // Run both servers concurrently
    log::info!(target: "ts::api", "Starting HTTP server on port {}", http_port);
    log::info!(target: "ts::api", "Starting UDS server at {:?}", uds_path);

    // Use `tokio::select!` to run both tasks concurrently
    tokio::select! {
        _ = http_server => {
            log::info!(target: "ts::api", "HTTP server has stopped");
        },
        _ = uds_server => {
            log::info!(target: "ts::api", "UDS server has stopped");
        },
    }
}
