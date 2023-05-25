use super::display::Display;
use core::fmt::Write;

static mut CONSOLE: Option<super::console::Console> = None;

pub fn print_args(args: core::fmt::Arguments) {
    super::serial::Serial.write_fmt(args).unwrap();
    match unsafe { super::DISPLAY.take() } {
        Some(display) => {
            let console = unsafe { CONSOLE.as_mut().unwrap() };
            console.write_fmt(args).unwrap();
            display.reinitialize_if_needed();
            console.render(&display);
            unsafe { super::DISPLAY = Some(display) };
        }
        None => (),
    }
}

pub fn update() {
    match unsafe { super::DISPLAY.take() } {
        Some(display) => {
            let console = unsafe { CONSOLE.as_mut().unwrap() };
            display.reinitialize_if_needed();
            console.render(&display);
            unsafe { super::DISPLAY = Some(display) };
        }
        None => (),
    }
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
    unsafe { CONSOLE = Some(super::console::Console::new()) };
    log::set_logger(&Logger).unwrap();
    log::set_max_level(log::STATIC_MAX_LEVEL);
}
