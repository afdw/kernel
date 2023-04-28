use core::fmt::Write;

pub fn _print(args: core::fmt::Arguments) {
    unsafe {
        crate::SYSTEM_TABLE.as_mut().unwrap().stdout().write_fmt(args).unwrap();
    }
}

macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::logger::_print(core::format_args!($($arg)*));
    }};
}

pub(crate) use print;

macro_rules! println {
    () => {
        $crate::logger::print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::logger::_print(core::format_args_nl!($($arg)*));
    }};
}

pub(crate) use println;

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

pub(crate) use dbg;

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    use uefi::proto::console::text::Color;
    unsafe {
        crate::SYSTEM_TABLE.as_mut().unwrap().stdout().set_color(Color::Black, Color::Red).unwrap();
    }
    println!("[PANIC]: {}", panic_info);
    loop {}
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        use uefi::proto::console::text::Color;
        unsafe {
            crate::SYSTEM_TABLE
                .as_mut()
                .unwrap()
                .stdout()
                .set_color(
                    match record.level() {
                        log::Level::Error => Color::Red,
                        log::Level::Warn => Color::Yellow,
                        log::Level::Info => Color::White,
                        log::Level::Debug => Color::LightGray,
                        log::Level::Trace => Color::DarkGray,
                    },
                    Color::Black,
                )
                .unwrap();
        }
        println!(
            "[{} {}:{}] {}",
            record.level(),
            record.file().unwrap_or("<unknown file>"),
            record.line().unwrap_or(0),
            record.args()
        );
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(log::STATIC_MAX_LEVEL);
}