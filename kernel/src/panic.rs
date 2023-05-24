use alloc::{boxed::Box, format, string::String, vec::Vec};
use core::{
    any::Any,
    mem::MaybeUninit,
    panic::Location,
    sync::atomic::{AtomicUsize, Ordering},
};

const PANIC_STYLE: super::formatting::Style = super::formatting::Style {
    reset: false,
    foreground_color: Some(super::formatting::Color::Black),
    background_color: Some(super::formatting::Color::Red),
};

#[repr(transparent)]
struct RustPanic(Box<dyn Any + Send>);

#[repr(C)]
struct ExceptionWithPayload {
    exception: MaybeUninit<unwinding::abi::UnwindException>,
    payload: RustPanic,
}

unsafe impl unwinding::panicking::Exception for RustPanic {
    const CLASS: [u8; 8] = *b"MOZ\0RUST";

    fn wrap(this: Self) -> *mut unwinding::abi::UnwindException {
        Box::into_raw(Box::new(ExceptionWithPayload {
            exception: MaybeUninit::uninit(),
            payload: this,
        })) as *mut unwinding::abi::UnwindException
    }

    unsafe fn unwrap(ex: *mut unwinding::abi::UnwindException) -> Self {
        let ex = unsafe { Box::from_raw(ex as *mut ExceptionWithPayload) };
        ex.payload
    }
}

static PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn abort() -> ! {
    loop {
        unsafe {
            x86::halt();
        }
    }
}

fn do_panic(msg: Box<dyn Any + Send>) -> ! {
    if PANIC_COUNT.load(Ordering::SeqCst) >= 1 {
        super::logger::print!("{}", PANIC_STYLE);
        super::logger::println!("[PANIC] thread panicked while processing panic");
        super::logger::print!("{}", super::backtrace::symbolize_backtrace(&super::backtrace::capture_backtrace()));
        super::logger::print!("{}", super::formatting::Style::RESET);
        abort()
    }
    PANIC_COUNT.fetch_add(1, Ordering::SeqCst);
    let code = unwinding::panicking::begin_panic(RustPanic(msg));
    super::logger::print!("{}", PANIC_STYLE);
    super::logger::println!("[PANIC] failed to initiate panic, error {}", code.0);
    super::logger::print!("{}", super::formatting::Style::RESET);
    abort()
}

struct PanicHandlerPayload {
    panic_info_string: String,
    frame_program_counters: Vec<usize>,
}

#[track_caller]
pub fn panic_any<M: 'static + Any + Send>(msg: M) -> ! {
    use uefi::proto::console::text::Color;
    unsafe {
        super::SYSTEM_TABLE.as_mut().unwrap().stdout().set_color(Color::Black, Color::Red).unwrap();
    }
    super::logger::print!("{}", PANIC_STYLE);
    super::logger::println!("[PANIC] at {}", Location::caller());
    super::logger::print!("{}", super::formatting::Style::RESET);
    do_panic(Box::new(msg));
}

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    do_panic(Box::new(PanicHandlerPayload {
        panic_info_string: format!("{}", panic_info),
        frame_program_counters: super::backtrace::capture_backtrace(),
    }))
}

pub fn catch_unwind<R, F: FnOnce() -> R>(f: F) -> Result<R, Box<dyn Any + Send>> {
    unwinding::panicking::catch_unwind(f).map_err(|p: Option<RustPanic>| match p {
        None => abort(),
        Some(e) => {
            PANIC_COUNT.store(0, Ordering::SeqCst);
            e.0
        }
    })
}

pub fn catch_unwind_with_default_handler<R, F: FnOnce() -> R>(f: F) -> R {
    match catch_unwind(f) {
        Ok(value) => value,
        Err(payload) => match payload.downcast::<PanicHandlerPayload>() {
            Ok(panic_handler_payload) => {
                super::logger::print!("{}", PANIC_STYLE);
                super::logger::println!("[PANIC] {}", panic_handler_payload.panic_info_string);
                super::logger::print!("{}", super::backtrace::symbolize_backtrace(&panic_handler_payload.frame_program_counters));
                super::logger::print!("{}", super::formatting::Style::RESET);
                abort();
            }
            Err(payload) => panic_any(payload),
        },
    }
}
