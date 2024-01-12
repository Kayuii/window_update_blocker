
pub mod gol;
mod macros;
mod panic_hook;


pub use gol::{init_global_logging, init_default_logging};
pub use panic_hook::set_panic_hook;


// pub use { tracing, tracing_appender, tracing_futures, tracing_subscriber};
