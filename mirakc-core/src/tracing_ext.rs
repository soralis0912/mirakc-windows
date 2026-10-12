use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;

use is_terminal::IsTerminal;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

pub fn init_tracing(format: &str) {
    match format {
        "json" => init_json_tracing(),
        _ => init_text_tracing(),
    }
}

// Logs are appended to the specified file.
//
// This is useful when STDOUT and STDERR are not available, such as when running as a Windows
// service.
pub fn init_tracing_with_file<P: AsRef<Path>>(format: &str, path: P) {
    let file = open_log_file(path.as_ref());
    match format {
        "json" => init_json_tracing_with_file(file),
        _ => init_text_tracing_with_file(file),
    }
}

fn open_log_file(path: &Path) -> File {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|err| panic!("Failed to open the log file {path:?}: {err}"))
}

fn init_json_tracing() {
    tracing_subscriber::fmt()
        .json()
        .with_timer(HrTime)
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(std::io::stdout().is_terminal())
        .init();
}

fn init_text_tracing() {
    tracing_subscriber::fmt()
        .with_timer(Rfc3339Micros)
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(std::io::stdout().is_terminal())
        .init();
}

fn init_json_tracing_with_file(file: File) {
    tracing_subscriber::fmt()
        .json()
        .with_timer(HrTime)
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .init();
}

fn init_text_tracing_with_file(file: File) {
    tracing_subscriber::fmt()
        .with_timer(Rfc3339Micros)
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .init();
}

struct HrTime;

impl FormatTime for HrTime {
    fn format_time(&self, w: &mut Writer) -> fmt::Result {
        const NANOS_IN_SEC: i64 = 1_000_000_000;
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap();
        let secs = ts / NANOS_IN_SEC;
        let nanos = ts & NANOS_IN_SEC;
        write!(w, "{secs}.{nanos:09}")
    }
}

struct Rfc3339Micros;

impl FormatTime for Rfc3339Micros {
    fn format_time(&self, w: &mut Writer) -> fmt::Result {
        let time = chrono::Local::now();
        write!(
            w,
            "{}",
            time.to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ctor::ctor(unsafe)]
    fn init_logging() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_file(true)
            .with_line_number(true)
            .with_test_writer()
            .with_timer(Rfc3339Micros)
            .init();
    }
}
