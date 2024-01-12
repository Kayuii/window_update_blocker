pub mod os;
mod logging;
mod service;
mod kill_update;

pub use logging::Logging;
pub use service::{
    SERVICE_TYPE, 
    ServiceStatusEx,
    WindowsService,
    install as serv_install,
    uninstall as serv_uninstall,
    start as serv_start,
    stop as serv_stop,
    get_config as serv_get_config,
    change_config as serv_change_config,
};
pub use kill_update::kill;