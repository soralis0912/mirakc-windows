mod migrate;
mod openapi;
mod rebuild_timeshift;
mod serve;
#[cfg(windows)]
mod service;
mod shutdown;

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " ", env!("VERGEN_GIT_SHA"));

#[derive(Parser)]
#[command(author, about, version = VERSION)]
struct CommandLine {
    /// Path to a configuration file.
    ///
    /// The MIRAKC_CONFIG environment variable is used if this option is not
    /// specified.  Its value has to be an absolute path.
    ///
    /// YAML (*.yml or *.yaml) or TOML (*.toml) formats are supported.
    ///
    /// See docs/config.md for details of the YAML/TOML format.
    #[arg(short, long, env = "MIRAKC_CONFIG", verbatim_doc_comment)]
    config: PathBuf,

    /// Logging format.
    #[arg(long, value_enum, env = "MIRAKC_LOG_FORMAT", default_value = "text")]
    log_format: LogFormat,

    /// Path to a log file.
    ///
    /// Logs are output to STDERR if this option is not specified.
    #[arg(long, env = "MIRAKC_LOG_FILE")]
    log_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, ValueEnum)]
enum LogFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    Migrate(migrate::CommandLine),
    Openapi(openapi::CommandLine),
    RebuildTimeshift(rebuild_timeshift::CommandLine),
    #[cfg(windows)]
    Service(service::CommandLine),
}

fn main() {
    let cl = CommandLine::parse();

    #[cfg(windows)]
    if let Some(Command::Service(ref service_cl)) = cl.command {
        service::main(&cl, service_cl);
        return;
    }

    init_tracing(&cl);
    tokio::runtime::Runtime::new()
        .expect("Failed to create a tokio runtime")
        .block_on(async_main(cl));
}

fn init_tracing(cl: &CommandLine) {
    // Disable logging in the `openapi` command if the OpenAPI document will be output to STDOUT.
    if matches!(cl.command, Some(Command::Openapi(ref cl)) if !cl.has_file()) {
        return;
    }
    let format = match cl.log_format {
        LogFormat::Text => "text",
        LogFormat::Json => "json",
    };
    match cl.log_file {
        Some(ref path) => mirakc_core::tracing_ext::init_tracing_with_file(format, path),
        None => mirakc_core::tracing_ext::init_tracing(format),
    }
}

async fn async_main(cl: CommandLine) {
    let config = mirakc_core::config::load(&cl.config);

    match cl.command {
        Some(Command::Migrate(ref cl)) => migrate::main(&config, cl).await,
        Some(Command::Openapi(ref cl)) => openapi::main(config, cl).await,
        Some(Command::RebuildTimeshift(ref cl)) => rebuild_timeshift::main(config, cl).await,
        #[cfg(windows)]
        Some(Command::Service(_)) => unreachable!(),
        None => {
            if auto_migrate() {
                tracing::info!("Migrating existing data automatically...");
                migrate::main(&config, &Default::default()).await;
            }
            serve::main(config).await
        }
    }
}

fn auto_migrate() -> bool {
    // Always perform auto-migration in release versions.
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    if version.pre.is_empty() {
        return true;
    }

    // DO NOT USE THE FOLLOWING ENVIRONMENT VARIABLE EXCEPT FOR DEBUGGING PURPOSES.
    match std::env::var_os("MIRAKC_DEBUG_FORCE_MIGRATE") {
        Some(v) => v == "1",
        None => false,
    }
}
