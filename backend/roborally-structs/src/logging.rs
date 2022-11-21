use std::thread;

use log::Record;
pub use log::{error, info, warn};

fn format(record: &Record) -> String {
    let cur_thread = thread::current();
    format!(
        "[{:0<18}] {}({}) {}@{}: {}",
        platform::get_time(),
        record.level(),
        cur_thread.name().map_or_else(
            || cur_thread.id().as_u64().to_string(),
            |n| format!("{n}:{:0>2}", cur_thread.id().as_u64())
        ),
        record.module_path().unwrap_or("<unknown module>"),
        record
            .line()
            .map_or_else(|| "??".to_owned(), |x| x.to_string()),
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
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= log::Level::Info
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

    pub fn init() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER).map(|()| set_max_level(LevelFilter::Trace))
    }

    pub fn get_time() -> f64 {
        js_sys::Date::now() / 1000.0
    }
}

#[cfg(all(not(feature = "client"), feature = "server"))]
mod platform {
    use log::{set_max_level, LevelFilter, Log, Metadata, Record, SetLoggerError};
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
            eprintln!("{}", super::format(record));
        }

        fn flush(&self) {}
    }

    pub fn init() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER).map(|()| set_max_level(LevelFilter::Debug))
    }

    pub fn get_time() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as f64
            / 1_000_000_000.0
    }
}

pub fn init() {
    if let Err(e) = platform::init() {
        let msg = format!("Error setting up logging: {e}");
        #[cfg(feature = "server")]
        eprintln!("{}", &msg);
        #[cfg(feature = "client")]
        web_sys::console::error_1(&msg.into());
    }
}
