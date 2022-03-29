use std::thread;

use log::Record;
pub use log::{debug, error, info, trace, warn};

fn format(record: &Record) -> String {
    let cur_thread = thread::current();
    format!(
        "{}({}) {}@{}: {}",
        record.level(),
        match cur_thread.name() {
            Some(n) => format!("{}: {}", n, cur_thread.id().as_u64()),
            None => cur_thread.id().as_u64().to_string(),
        },
        record.module_path().unwrap_or("<unknown module>"),
        record
            .line()
            .map(|x| x.to_string())
            .unwrap_or_else(|| "??".to_string()),
        record.args()
    )
}

#[cfg(feature = "client")]
mod platform {
    use log::{set_max_level, LevelFilter, Log, Metadata, Record, SetLoggerError};
    use web_sys::console;

    pub struct Logger;
    static LOGGER: Logger = Logger;

    impl Log for Logger {
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let msg = super::format(record).into();
            match record.level() {
                log::Level::Error => console::error_1(&msg),
                log::Level::Warn => console::warn_1(&msg),
                log::Level::Info => console::info_1(&msg),
                log::Level::Debug => console::log_1(&msg),
                log::Level::Trace => console::debug_1(&msg),
            }
        }

        fn flush(&self) {}
    }

    pub(super) fn init() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER).map(|()| set_max_level(LevelFilter::Trace))
    }
}

#[cfg(all(not(feature = "client"), feature = "server"))]
mod platform {
    use log::{set_max_level, LevelFilter, Log, Metadata, Record, SetLoggerError};
    pub struct Logger;
    static LOGGER: Logger = Logger;

    impl Log for Logger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            eprintln!("{}", super::format(record));
        }

        fn flush(&self) {}
    }

    pub(super) fn init() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER).map(|()| set_max_level(LevelFilter::Debug))
    }
}

pub fn init() {
    if let Err(e) = platform::init() {
        let msg = format!("Error setting up logging: {}", e);
        #[cfg(feature = "server")]
        eprintln!("{}", &msg);
        #[cfg(feature = "client")]
        web_sys::console::error_1(&msg.into());
    }
}
