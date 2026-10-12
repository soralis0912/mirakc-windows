use std::sync::LazyLock;

use tokio::sync::Notify;

// Used for requesting shutdown from places other than signal handlers such as the Windows
// service control handler.
static SHUTDOWN_REQUESTED: LazyLock<Notify> = LazyLock::new(Notify::new);

#[allow(unused)]
pub fn request() {
    SHUTDOWN_REQUESTED.notify_one();
}

// Waits for a signal or a request to stop the process.
#[cfg(unix)]
pub async fn wait() {
    use tokio::signal::unix::SignalKind;
    use tokio::signal::unix::signal;

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => {
            tracing::info!("SIGINT received");
        }
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received");
        }
        _ = SHUTDOWN_REQUESTED.notified() => {
            tracing::info!("Shutdown requested");
        }
    }
}

// Waits for a console control event or a request to stop the process.
#[cfg(windows)]
pub async fn wait() {
    use tokio::signal::windows;

    let mut ctrl_c = windows::ctrl_c().unwrap();
    let mut ctrl_break = windows::ctrl_break().unwrap();
    let mut ctrl_close = windows::ctrl_close().unwrap();
    let mut ctrl_shutdown = windows::ctrl_shutdown().unwrap();

    tokio::select! {
        _ = ctrl_c.recv() => {
            tracing::info!("CTRL_C received");
        }
        _ = ctrl_break.recv() => {
            tracing::info!("CTRL_BREAK received");
        }
        _ = ctrl_close.recv() => {
            tracing::info!("CTRL_CLOSE received");
        }
        _ = ctrl_shutdown.recv() => {
            tracing::info!("CTRL_SHUTDOWN received");
        }
        _ = SHUTDOWN_REQUESTED.notified() => {
            tracing::info!("Shutdown requested");
        }
    }
}
