use alloc::boxed::Box;
use core::{
    any::Any,
    panic::Location,
    sync::atomic::{AtomicUsize, Ordering},
};

#[repr(transparent)]
struct RustPanic(Box<dyn Any + Send>);

unsafe impl unwinding::panicking::Exception for RustPanic {
    const CLASS: [u8; 8] = *b"MOZ\0RUST";

    fn wrap(this: Self) -> *mut unwinding::abi::UnwindException {
        Box::into_raw(Box::new(this)) as *mut unwinding::abi::UnwindException
    }

    unsafe fn unwrap(ex: *mut unwinding::abi::UnwindException) -> Self {
        unsafe { *Box::from_raw(ex as *mut RustPanic) }
    }
}

static PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn abort() -> ! {
    loop {
        unsafe {
            core::arch::x86_64::_mm_pause();
        }
    }
}

fn do_panic(msg: Box<dyn Any + Send>) -> ! {
    use uefi::proto::console::text::Color;
    unsafe {
        super::SYSTEM_TABLE.as_mut().unwrap().stdout().set_color(Color::Black, Color::Red).unwrap();
    }
    if PANIC_COUNT.load(Ordering::SeqCst) >= 1 {
        super::logger::println!("[PANIC] thread panicked while processing panic");
        abort()
    }
    super::logger::print!("{}", super::backtrace::symbolize_backtrace(&super::backtrace::capture_backtrace()));
    PANIC_COUNT.fetch_add(1, Ordering::SeqCst);
    let code = unwinding::panicking::begin_panic(RustPanic(msg));
    super::logger::println!("[PANIC] failed to initiate panic, error {}", code.0);
    abort()
}

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    use uefi::proto::console::text::Color;
    unsafe {
        super::SYSTEM_TABLE.as_mut().unwrap().stdout().set_color(Color::Black, Color::Red).unwrap();
    }
    super::logger::println!("[PANIC] {}", panic_info);
    struct NoPayload;
    do_panic(Box::new(NoPayload))
}

#[track_caller]
#[allow(dead_code)]
pub fn panic_any<M: 'static + Any + Send>(msg: M) -> ! {
    use uefi::proto::console::text::Color;
    unsafe {
        super::SYSTEM_TABLE.as_mut().unwrap().stdout().set_color(Color::Black, Color::Red).unwrap();
    }
    super::logger::println!("[PANIC] at {}", Location::caller());
    do_panic(Box::new(msg));
}

#[allow(dead_code)]
pub fn begin_panic(payload: Box<dyn Any + Send>) -> unwinding::abi::UnwindReasonCode {
    unwinding::panicking::begin_panic(RustPanic(payload))
}

#[allow(dead_code)]
pub fn catch_unwind<R, F: FnOnce() -> R>(f: F) -> Result<R, Box<dyn Any + Send>> {
    unwinding::panicking::catch_unwind(f).map_err(|p: Option<RustPanic>| match p {
        None => abort(),
        Some(e) => {
            PANIC_COUNT.store(0, Ordering::SeqCst);
            e.0
        }
    })
}
