use core::fmt::Write;

pub fn print_args(args: core::fmt::Arguments) {
    super::serial::Serial.write_fmt(args).unwrap()
}

#[allow(unused_macros)]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::logger::print_args(core::format_args!($($arg)*));
    }};
}

#[allow(unused_imports)]
pub(crate) use print;

#[allow(unused_macros)]
macro_rules! println {
    () => {
        $crate::logger::print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::logger::print_args(core::format_args_nl!($($arg)*));
    }};
}

#[allow(unused_imports)]
pub(crate) use println;

#[allow(unused_macros)]
macro_rules! dbg {
    () => {
        log::debug!("")
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                log::debug!("{} = {:#?}", core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}

#[allow(unused_imports)]
pub(crate) use dbg;

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if record.file().is_some() && record.file().unwrap().contains(".cargo/registry") {
            return;
        }
        print!(
            "{}",
            super::formatting::Style {
                reset: false,
                foreground_color: match record.level() {
                    log::Level::Error => Some(super::formatting::Color::Red),
                    log::Level::Warn => Some(super::formatting::Color::Yellow),
                    log::Level::Info => Some(super::formatting::Color::BrightWhite),
                    log::Level::Debug => Some(super::formatting::Color::White),
                    log::Level::Trace => Some(super::formatting::Color::BrightBlack),
                },
                background_color: None,
            }
        );
        println!(
            "[{} {}:{}] {}",
            record.level(),
            record.file().unwrap_or("<unknown file>"),
            record.line().unwrap_or(0),
            record.args()
        );
        print!("{}", super::formatting::Style::RESET);
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(log::STATIC_MAX_LEVEL);
}
