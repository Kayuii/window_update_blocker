use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, Instant},
};

use own_logger::*;
use splitty::split_unquoted_whitespace;
use windows_service::service::{ServiceInfo, ServiceStartType};

use crate::{serv_change_config, serv_get_config, serv_stop}; 

pub const BLOCK_WINDOWS_UPDATES: &[&'static str] =
    &["wuauserv", "WaaSMedicSvc", "UsoSvc", "bits", "DoSvc", "PeerDistSvc", "appidsvc"];

pub fn kill() -> anyhow::Result<()> {
    BLOCK_WINDOWS_UPDATES.iter().for_each(|name| {
        info!("service::kill_update_service: killing {}", name);
        // std::process::Command::new("taskkill").args(&["/IM", name]).spawn().ok();
        let _ = serv_stop(name);
        let serv_config = serv_get_config(name);
        match serv_config {
            Ok(config) => {
                if config.start_type == ServiceStartType::Disabled {
                    let path = config.executable_path.to_str().unwrap();

                    let split_path: Vec<&str> = split_unquoted_whitespace(path)
                        .unwrap_quotes(true)
                        .collect();
                    
                    let buffer: Vec<OsString> =
                        split_path.iter().map(|x| OsString::from(x)).collect();

                    let new_config = ServiceInfo {
                        name: OsString::from(name),
                        display_name: config.display_name,
                        service_type: config.service_type,
                        start_type: ServiceStartType::Disabled,
                        error_control: config.error_control,
                        executable_path: PathBuf::from(buffer[0].clone()),
                        launch_arguments: buffer[1..].to_vec(),
                        dependencies: config.dependencies,
                        account_name: config.account_name,
                        account_password: None,
                    };

                    let _ = serv_change_config(name, new_config);
                } else {
                    info!("{} is disabled", name);
                }
            }
            Err(_) => {}
        }
    });

    Ok(())
}
