use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use windows_service::define_windows_service;
use windows_service::service::ServiceAccess;
use windows_service::service::ServiceControl;
use windows_service::service::ServiceControlAccept;
use windows_service::service::ServiceErrorControl;
use windows_service::service::ServiceExitCode;
use windows_service::service::ServiceInfo;
use windows_service::service::ServiceStartType;
use windows_service::service::ServiceState;
use windows_service::service::ServiceStatus;
use windows_service::service::ServiceType;
use windows_service::service_control_handler;
use windows_service::service_control_handler::ServiceControlHandlerResult;
use windows_service::service_dispatcher;
use windows_service::service_manager::ServiceManager;
use windows_service::service_manager::ServiceManagerAccess;

use crate::LogFormat;

const DEFAULT_SERVICE_NAME: &str = "mirakc";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

/// Manage mirakc as a Windows service.
///
/// Install the service (run in an elevated command prompt):
///
///   mirakc -c C:\mirakc\config.yml service install
///
/// Then start it by using `sc start mirakc` or the Services management console.
///
/// Logs are written to the file specified with the `--log-file` option.  If it's not specified,
/// `mirakc.log` in the same folder as the config file is used.
#[derive(Args)]
#[clap(verbatim_doc_comment)]
pub struct CommandLine {
    #[command(subcommand)]
    command: ServiceCommand,
}

#[derive(Subcommand)]
enum ServiceCommand {
    /// Install mirakc as a Windows service.
    Install(InstallCommandLine),

    /// Stop and uninstall the Windows service.
    Uninstall(NameArg),

    /// Run as a Windows service.
    ///
    /// This command is invoked by the Service Control Manager.  Don't use it directly.
    #[command(hide = true)]
    Run(RunCommandLine),
}

#[derive(Args)]
struct NameArg {
    /// Service name.
    #[arg(long, default_value = DEFAULT_SERVICE_NAME)]
    name: String,
}

#[derive(Args)]
struct InstallCommandLine {
    #[command(flatten)]
    name: NameArg,

    /// Display name of the service.
    #[arg(long, default_value = "mirakc")]
    display_name: String,

    /// Start type of the service.
    #[arg(long, value_enum, default_value = "auto")]
    start: StartType,

    /// Log filter used in the service.
    ///
    /// The value is set to the RUST_LOG environment variable in the service process.
    #[arg(long, default_value = "info")]
    log_filter: String,
}

#[derive(Args)]
struct RunCommandLine {
    #[command(flatten)]
    name: NameArg,

    /// Log filter.
    #[arg(long)]
    log_filter: Option<String>,
}

#[derive(Clone, ValueEnum)]
enum StartType {
    Auto,
    Manual,
    Disabled,
}

impl From<&StartType> for ServiceStartType {
    fn from(value: &StartType) -> Self {
        match value {
            StartType::Auto => ServiceStartType::AutoStart,
            StartType::Manual => ServiceStartType::OnDemand,
            StartType::Disabled => ServiceStartType::Disabled,
        }
    }
}

pub fn main(cl: &crate::CommandLine, service_cl: &CommandLine) {
    let result = match service_cl.command {
        ServiceCommand::Install(ref install_cl) => install(cl, install_cl),
        ServiceCommand::Uninstall(ref name) => uninstall(&name.name),
        ServiceCommand::Run(_) => run(),
    };
    if let Err(err) = result {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn install(cl: &crate::CommandLine, install_cl: &InstallCommandLine) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let config = std::path::absolute(&cl.config)?;
    let log_file = match cl.log_file {
        Some(ref path) => std::path::absolute(path)?,
        None => default_log_file(&config),
    };
    let log_format = match cl.log_format {
        LogFormat::Text => "text",
        LogFormat::Json => "json",
    };

    let launch_arguments: Vec<OsString> = vec![
        "--config".into(),
        config.into(),
        "--log-format".into(),
        log_format.into(),
        "--log-file".into(),
        log_file.clone().into(),
        "service".into(),
        "run".into(),
        "--name".into(),
        install_cl.name.name.clone().into(),
        "--log-filter".into(),
        install_cl.log_filter.clone().into(),
    ];

    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;
    let info = ServiceInfo {
        name: install_cl.name.name.clone().into(),
        display_name: install_cl.display_name.clone().into(),
        service_type: SERVICE_TYPE,
        start_type: (&install_cl.start).into(),
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        launch_arguments,
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };
    let service = manager.create_service(&info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("A Mirakurun-compatible PVR backend written in Rust")?;

    println!("Installed the service: {}", install_cl.name.name);
    println!("Log file: {}", log_file.display());
    Ok(())
}

fn uninstall(name: &str) -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = manager.open_service(name, access)?;

    // Mark the service for deletion.  It will be deleted after it stops.
    service.delete()?;

    if service.query_status()?.current_state != ServiceState::Stopped {
        service.stop()?;
        for _ in 0..30 {
            if service.query_status()?.current_state == ServiceState::Stopped {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    println!("Uninstalled the service: {name}");
    Ok(())
}

fn default_log_file(config: &Path) -> PathBuf {
    config
        .parent()
        .map(|dir| dir.join("mirakc.log"))
        .unwrap_or_else(|| PathBuf::from("mirakc.log"))
}

define_windows_service!(ffi_service_main, service_main);

fn run() -> anyhow::Result<()> {
    // Blocks until the service stops.
    service_dispatcher::start(service_name(), ffi_service_main)?;
    Ok(())
}

// Arguments passed via `StartService()` are ignored.  The command line arguments of the process
// specified at the installation are used instead.
fn service_main(_arguments: Vec<OsString>) {
    let cl = crate::CommandLine::parse();
    if let Some(crate::Command::Service(CommandLine {
        command: ServiceCommand::Run(ref run_cl),
    })) = cl.command
        && let Some(ref filter) = run_cl.log_filter
        && std::env::var_os("RUST_LOG").is_none()
    {
        // SAFETY: No other threads read or write the environment at this point.
        unsafe { std::env::set_var("RUST_LOG", filter) };
    }

    crate::init_tracing(&cl);
    if let Err(err) = run_service(cl) {
        tracing::error!(%err, "Service failed");
    }
}

fn run_service(cl: crate::CommandLine) -> anyhow::Result<()> {
    let event_handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown | ServiceControl::Preshutdown => {
            tracing::info!(?control, "Stop requested by the Service Control Manager");
            crate::shutdown::request();
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let status_handle = service_control_handler::register(service_name(), event_handler)?;

    let set_status = |state, controls_accepted, wait_hint| {
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: state,
            controls_accepted,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint,
            process_id: None,
        })
    };

    set_status(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        Duration::default(),
    )?;
    tracing::info!("Service started");

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(crate::async_main(cl));
    // Don't wait for remaining tasks such as tuner processes.
    runtime.shutdown_timeout(Duration::from_secs(10));

    tracing::info!("Service stopped");
    set_status(
        ServiceState::Stopped,
        ServiceControlAccept::empty(),
        Duration::default(),
    )?;
    Ok(())
}

// The service name is needed before the service starts.  It's obtained from the command line
// arguments specified at the installation.
fn service_name() -> String {
    let cl = crate::CommandLine::parse();
    match cl.command {
        Some(crate::Command::Service(CommandLine {
            command: ServiceCommand::Run(ref run_cl),
        })) => run_cl.name.name.clone(),
        _ => DEFAULT_SERVICE_NAME.to_string(),
    }
}
