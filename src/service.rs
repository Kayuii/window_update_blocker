use own_logger::*;
use std::{
    path::PathBuf,
    time::{Duration, Instant}, ffi::{OsString, OsStr},
};
use windows_service::{
    service::{
        ServiceAccess, ServiceConfig, ServiceControlAccept,
        ServiceExitCode, ServiceInfo, ServiceState, ServiceStatus, ServiceType,
    },
    // service_control_handler::{self, ServiceControlHandlerResult},
    // service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;

use anyhow::{anyhow, Result};
pub const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

pub trait ServiceStatusEx {
    fn running() -> ServiceStatus;
    fn stopped() -> ServiceStatus;
    fn stopped_with_error(code: u32) -> ServiceStatus;
}

impl ServiceStatusEx for ServiceStatus {
    fn running() -> ServiceStatus {
        ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        }
    }

    fn stopped() -> ServiceStatus {
        ServiceStatus {
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            ..Self::running()
        }
    }

    fn stopped_with_error(code: u32) -> ServiceStatus {
        ServiceStatus {
            exit_code: ServiceExitCode::ServiceSpecific(code),
            ..Self::stopped()
        }
    }
}

pub struct WindowsService {
    // log: log::Logger,
    pub name: OsString,
    pub display_name: OsString,
    pub executable_path: PathBuf,
    pub arguments: Vec<OsString>,
}

/// Checks if a service is installed on Windows via name.
fn service_exist(service_name: &str) -> bool {
    let manager_access = ServiceManagerAccess::CONNECT;
    if let Ok(service_manager) = ServiceManager::local_computer(None::<&str>, manager_access) {
        if let Ok(_service) =
            service_manager.open_service(service_name, ServiceAccess::QUERY_CONFIG)
        {
            return true;
        }
    }
    false
}

pub fn install(service: WindowsService, service_info: ServiceInfo) -> anyhow::Result<()> {
    if service_exist(service.name.to_str().unwrap()) {
        return Err(anyhow!("Service already exists"));
    }
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(anyhow!("Failed to connect to service manager")),
    };

    if let Err(_) = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG) {
        return Err(anyhow!("Failed to create service"));
    }

    // let _ = service.set_description(OsString::from(service_description));
    // grant_start_access_rights(&service.name)
    info!("service installed");
    Ok(())
}

/// Update the permissions so that any user can start the specified service
// pub fn grant_start_access_rights(service_name: &str) -> Result<bool, BootstrapError> {
//     let custom_manager = match dacl::CustomServiceManager::new() {
//         Ok(sm) => sm,
//         Err(_e) => return Err(BootstrapError::ServiceConnectionFailure),
//     };
//     match custom_manager.change_service_dacl(service_name) {
//         Ok(_) => {
//             log::info!("Successfully updated service");
//             Ok(true)
//         }
//         Err(_) => Ok(false),
//     }
// }

pub fn uninstall(service_name: &str) -> Result<()> {
    if !service_exist(service_name) {
        return Err(anyhow!("Service is no exist"));
    }
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(anyhow!("Failed to connect to service manager")),
    };

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = match service_manager.open_service(service_name, service_access) {
        Ok(s) => s,
        Err(_e) => return Err(anyhow!("Failed to open service")),
    };
    let service_status = match service.query_status() {
        Ok(s) => s,
        Err(_e) => return Err(anyhow!("Failed to query service status")),
    };
    if service_status.current_state != ServiceState::Stopped {
        if let Ok(_s) = service.stop() {
            info!("Stopped {}", service_name);
        }
    }
    if let Ok(_s) = service.delete() {
        info!("Deleteing {}", service_name);
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(service_name, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                warn!("service deleted");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

pub fn stop(service_name: &str) -> Result<()> {
    if !service_exist(service_name) {
        return Err(anyhow!("Service is no exist"));
    }
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(anyhow!("Failed to connect to service manager")),
    };

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP;
    let service = match service_manager.open_service(service_name, service_access) {
        Ok(s) => s,
        Err(_e) => return Err(anyhow!("Failed to open service")),
    };
    let service_status = match service.query_status() {
        Ok(s) => s,
        Err(_e) => return Err(anyhow!("Failed to query service status")),
    };
    if service_status.current_state != ServiceState::Stopped {
        if let Ok(_s) = service.stop() {
            info!("Stopped {}", service_name);
        }
        // Wait for service to stop
        std::thread::sleep(Duration::from_secs(5));
    }
    Ok(())
}

pub fn start(service_name: &str) -> Result<()> {
    if !service_exist(service_name) {
        return Err(anyhow!("Service is no exist"));
    }
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(anyhow!("Failed to connect to service manager")),
    };
    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
    let service = match service_manager.open_service(service_name, service_access) {
        Ok(s) => s,
        Err(_e) => return Err(anyhow!("Failed to open service")),
    };
    // let service_status = match service.query_status() {
    //     Ok(s) => s,
    //     Err(_e) => return Err(anyhow!("Failed to query service status")),
    // };
    // if service_status.current_state != ServiceState::Stopped {
    //     if let Ok(_s) = service.stop() {
    //         info!("Stopped {}", service_name);
    //     }
    // }
    // match service.start(&[OsStr::new("Started from Rust!")]) {
    match service.start(Vec::<&str>::new().as_slice()) {
        Ok(_o) => Ok(()),
        Err(_e) => Err(anyhow!("Failed to query service status")),
    }
}

pub fn get_config(service_name: &str) -> anyhow::Result<ServiceConfig> {
    if !service_exist(service_name) {
        return Err(anyhow!("Service is no exist"));
    }
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(anyhow!("Failed to connect to service manager")),
    };

    let service_access = ServiceAccess::QUERY_CONFIG;
    let service = match service_manager.open_service(service_name, service_access) {
        Ok(s) => s,
        Err(_e) => return Err(anyhow!("Failed to open service")),
    };
    let service_config = service.query_config()?;
    // info!("{:#?}", service_config);
    Ok(service_config)
}

pub fn change_config(service_name: &str, service_info: ServiceInfo) -> anyhow::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access =
        ServiceAccess::QUERY_STATUS | ServiceAccess::CHANGE_CONFIG | ServiceAccess::STOP;
    let service = service_manager.open_service(service_name, service_access)?;

    info!("change {} service", service_name);
    info!("stopping {} service", service_name);

    let service_status = service.query_status()?;
    if service_status.current_state == ServiceState::Running {
        // stop service
        service.stop().ok();
        // wait for service to stop
        loop {
            let _state = service.query_status()?;
            // info!("_state:{:#?}", _state);
            if _state.current_state == ServiceState::Stopped {
                info!("{} service stopped", service_name);
                break;
            }

            std::thread::sleep(Duration::from_millis(250));
        }
    }

    info!("patching {} service", service_name);
    // service_info.executable_path = service_binary_path;
    service.change_config(&service_info)?;

    info!("successfully patched {} service", service_name);

    // info!("starting {} service", service_name);
    // service.start(Vec::<&str>::new().as_slice())?;
    // info!("started {} service", service_name);

    Ok(())
}
