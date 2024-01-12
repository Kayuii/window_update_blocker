// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// The standard gol macro.
#[macro_export]
macro_rules! log {
    // log!(target: "my_target", Level::INFO, "a {} event", "log");
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => {
        $crate::gol::event!(target: $target, $lvl, $($arg)+)
    };

    // log!(Level::INFO, "a log event")
    ($lvl:expr, $($arg:tt)+) => {
        $crate::gol::event!($lvl, $($arg)+)
    };
}

/// Logs a message at the error level.
#[macro_export]
macro_rules! error {
    // error!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ({
        $crate::log!(target: $target, $crate::gol::Level::ERROR, $($arg)+)
    });

    // error!(e; target: "my_target", "a {} event", "log")
    ($e:expr; target: $target:expr, $($arg:tt)+) => ({
        use $crate::ext_error::ext::ErrorExt;
        use std::error::Error;
        match ($e.source(), $e.location_opt()) {
            (Some(source), Some(location)) => {
                $crate::log!(
                    target: $target,
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    err.source = source,
                    err.location = %location,
                    $($arg)+
                )
            },
            (Some(source), None) => {
                $crate::log!(
                    target: $target,
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    err.source = source,
                    $($arg)+
                )
            },
            (None, Some(location)) => {
                $crate::log!(
                    target: $target,
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    err.location = %location,
                    $($arg)+
                )
            },
            (None, None) => {
                $crate::log!(
                    target: $target,
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    $($arg)+
                )
            }
        }
    });

    // error!(e; "a {} event", "log")
    ($e:expr; $($arg:tt)+) => ({
        use std::error::Error;
        use $crate::ext_error::ext::ErrorExt;
        match ($e.source(), $e.location_opt()) {
            (Some(source), Some(location)) => {
                $crate::log!(
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    err.source = source,
                    err.location = %location,
                    $($arg)+
                )
            },
            (Some(source), None) => {
                $crate::log!(
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    err.source = source,
                    $($arg)+
                )
            },
            (None, Some(location)) => {
                $crate::log!(
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    err.location = %location,
                    $($arg)+
                )
            },
            (None, None) => {
                $crate::log!(
                    $crate::gol::Level::ERROR,
                    err.msg = %$e,
                    err.code = %$e.status_code(),
                    $($arg)+
                )
            }
        }
    });

    // error!("a {} event", "log")
    ($($arg:tt)+) => ({
        $crate::log!($crate::gol::Level::ERROR, $($arg)+)
    });
}

/// Logs a message at the warn level.
#[macro_export]
macro_rules! warn {
    // warn!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        $crate::log!(target: $target, $crate::gol::Level::WARN, $($arg)+)
    };

    // warn!("a {} event", "log")
    ($($arg:tt)+) => {
        $crate::log!($crate::gol::Level::WARN, $($arg)+)
    };
}

/// Logs a message at the info level.
#[macro_export]
macro_rules! info {
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        $crate::log!(target: $target, $crate::gol::Level::INFO, $($arg)+)
    };

    // info!("a {} event", "log")
    ($($arg:tt)+) => {
        $crate::log!($crate::gol::Level::INFO, $($arg)+)
    };
}

/// Logs a message at the debug level.
#[macro_export]
macro_rules! debug {
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        $crate::log!(target: $target, $crate::gol::Level::DEBUG, $($arg)+)
    };

    // debug!("a {} event", "log")
    ($($arg:tt)+) => {
        $crate::log!($crate::gol::Level::DEBUG, $($arg)+)
    };
}

/// Logs a message at the trace level.
#[macro_export]
macro_rules! trace {
    // trace!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        $crate::log!(target: $target, $crate::gol::Level::TRACE, $($arg)+)
    };

    // trace!("a {} event", "log")
    ($($arg:tt)+) => {
        $crate::log!($crate::gol::Level::TRACE, $($arg)+)
    };
}
