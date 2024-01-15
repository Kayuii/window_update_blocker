use clap::Parser;
use tokio::process::Command;
use tokio_cron_scheduler::{JobScheduler, Job};
use std::{ffi::OsString, env};
use anyhow::anyhow;
use own_logger::*;
use tokio_util::sync::CancellationToken;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceErrorControl, 
        ServiceInfo, ServiceStartType,  ServiceStatus, 
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};
use window_update_blocker::{os::windows::is_elevated, 
    serv_install, serv_uninstall, serv_start, serv_stop,
};
use window_update_blocker::{Logging, ServiceStatusEx, WindowsService, SERVICE_TYPE};

const SERVICE_NAME: &str = "WindowsUpdateBlocker.rs";
const SERVICE_DESCRIPTION: &str = "Blocker for Windows Update";
const SERVICE_DISPLAY: &str = "Blocker for Windows Update";
const SERVICE_ARGUMENTS: &[&'static str] = &["run"];

#[derive(Parser, Debug)]
#[command(name = "window_update_blocker")]
#[command(author = "gecko <577738@qq.com>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "window update blocker", long_about = None)]
pub struct Args {
    #[clap(flatten)]
    output: Logging,

    #[clap(subcommand)]
    cmd: Option<Cmd>,
}

impl Args {
    fn execute(self) -> Result<(), anyhow::Error> {
        let Args { cmd, output } = self;

        output.initialize_logging();
        own_logger::set_panic_hook();

        match cmd {
            #[cfg(windows)]
            Some(Cmd::Install) => {
                if !is_elevated() {
                    return Err(anyhow::Error::msg("the program isn’t running as elevated"));
                }
                match uninstall() {
                    Ok(_) => {
                        install()?;
                    }
                    Err(e) => {
                        error!("Service uninstall error: {}", e);
                    }
                }

                Ok(())
            }
            #[cfg(windows)]
            Some(Cmd::Uninstall) => uninstall(),
            #[cfg(windows)]
            Some(Cmd::Start) => start(),
            #[cfg(windows)]
            Some(Cmd::Stop) => stop(),
            #[cfg(windows)]
            Some(Cmd::Run) => run(),

            None => Ok({
                // std::process::exit(0);
            }),
        }
    }
}

#[derive(Parser, Debug)]
/// The options for the wasmer Command Line Interface
enum Cmd {
    #[cfg(windows)]
    Install,
    #[cfg(windows)]
    Uninstall,
    #[cfg(windows)]
    Start,
    #[cfg(windows)]
    Stop,
    #[cfg(windows)]
    Run,
}

fn main() {
    match Args::try_parse() {
        Ok(args) => {
            let _ = args.execute();
        }
        Err(e) => {
            let _ = matches!(
                e.kind(),
                clap::error::ErrorKind::InvalidSubcommand | clap::error::ErrorKind::UnknownArgument
            );
            e.exit();
        }
    }
}

pub fn install() -> anyhow::Result<()> {
    let agent_serv = WindowsService {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY),
        executable_path: env::current_exe().unwrap_or_default(),
        arguments: SERVICE_ARGUMENTS
            .into_iter()
            .map(|x| OsString::from(x))
            .rev()
            .collect(),
    };

    let service_info = ServiceInfo {
        name: agent_serv.name.clone(),
        display_name: agent_serv.display_name.clone(),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: agent_serv.executable_path.clone(),
        launch_arguments: agent_serv.arguments.clone(),
        dependencies: vec![],
        account_name: None,
        // account_name: Some(OsString::from(r#"NT AUTHORITY\NetworkService"#)),
        account_password: None,
    };
    let _ = serv_install(agent_serv, service_info);
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    let _ = serv_uninstall(SERVICE_NAME);
    Ok(())
}
pub fn start() -> anyhow::Result<()> {
    let _ = serv_start(SERVICE_NAME);
    Ok(())
}
pub fn stop() -> anyhow::Result<()> {
    let _ = serv_stop(SERVICE_NAME);
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    define_windows_service!(ffi_service_main, my_service_main);
    Ok(service_dispatcher::start(SERVICE_NAME, ffi_service_main)?)
}

pub fn my_service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service(_arguments) {
        error!("error: {}", e);
    }
}

pub fn run_service(arguments: Vec<OsString>) -> anyhow::Result<()> {
    info!("service start {:?}", arguments);
    // Create a cancellation token to be able to cancell server
    let control_token = CancellationToken::new();
    let server_token = control_token.child_token();

    // Define system service event handler that will be receiving service events.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                info!("service stop event received");
                control_token.cancel();
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler.
    // The returned status handle should be used to report service status changes to the system.
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Tell the system that service is running
    status_handle.set_service_status(ServiceStatus::running())?;
    info!("service running");

    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build() {
            Ok(rt) => {
                match std::thread::spawn(move || {
                    rt.block_on(async { 
                        serv_executor(server_token).await
                    })
                }).join() {
                    Ok(_) => {
                        info!("server thread stoped");
                        // Tell the system that service has stopped.
                        status_handle.set_service_status(ServiceStatus::stopped())?;
                                            info!("service stoped");
                        Ok(())
                    },
                    Err(e) => {
                        error!("server panic: {:#?}", e);
                        status_handle.set_service_status(ServiceStatus::stopped_with_error(1))?;
                        return Err(anyhow!("server panic"));
                    },
                }

            },
            Err(e) => {
                error!("server error: {:#?}", e);
                status_handle.set_service_status(ServiceStatus::stopped_with_error(2))?;
                return Err(anyhow!("server error"));
            },
        }
}

async fn serv_executor(token: CancellationToken) -> anyhow::Result<()> {
    let mut sched = JobScheduler::new().await.unwrap();

    sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            info!("sched shutdown done");
        })
    }));

    sched
    .add(
        Job::new_async("0 */5 * * * *", |_uuid, _l| {
            Box::pin(async move {
                // let _ = serv_kill_update::kill();


                let cmds = &[
                    r#"Disable-ScheduledTask -TaskName "\Microsoft\Windows\WindowsUpdate\Scheduled Start" | Out-Null")"#,
                    r#"if (!(Test-Path "HKLM:\Software\Microsoft\WindowsUpdate\UX\Settings")) { New-Item -Path "HKLM:\Software\Microsoft\WindowsUpdate\UX\Settings" -Force | Out-Null }"#,
                    r#"Set-ItemProperty -Path "HKLM:\Software\Microsoft\WindowsUpdate\UX\Settings" -Name "UxOption" -Type DWord -Value 1"#,
                    r#"Stop-Process -Name "MoUsoCoreWorker", "TiWorker" -Force -PassThru -ErrorAction SilentlyContinue | Out-Null"#,
                    r#"Set-ItemProperty -Path "HKLM:\SYSTEM\ControlSet001\Services\WaaSMedicSvc" -Name Start -Value 4"#,
                ];

                cmds.iter().for_each(|cmd| {
                    tokio::spawn(async move {
                        match Command::new("powershell")
                            .arg("-Command")
                            .arg(cmd)
                            .status()
                            .await {
                            Ok(_) => {
                                info!("{}", cmd);
                            },
                            Err(err) => {
                                error!("psh {}", err);
                            }
                        }
                    });
                });
                

            })
        })
        .unwrap(),
    )
    .await
    .expect("Should be able to add a job");

sched.start().await.unwrap();


loop {
    tokio::select! {
      _ = token.cancelled() => {
        // let _ = sched.shutdown().await;
        if let Err(err) = sched.shutdown().await {
            error!("Cancelled {:?}", err);
        }
        // info!("Cancelled");
        break;
      },
    }
}

    Ok(())
}