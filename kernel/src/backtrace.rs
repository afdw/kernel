use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

static ROOT: AtomicU64 = AtomicU64::new(0);

fn probe() {}

pub fn capture_backtrace(limit: Option<usize>) -> Vec<u64> {
    let debug_info = unsafe { super::DEBUG_INFO.as_ref().unwrap() };
    let text_relative_start = probe as usize as u64
        - debug_info
            .functions
            .iter()
            .find(|function| function.name == "kernel::backtrace::probe")
            .unwrap()
            .relative_start;
    let mut frame_program_counter = x86::bits64::registers::rip();
    let mut frame_pointer = x86::bits64::registers::rsp();
    let mut frame_program_counters = Vec::new();
    let mut frame_index = 0;
    while limit.is_none() || frame_index < limit.unwrap() {
        super::logger::dbg!(frame_program_counter as *const u64);
        let function = debug_info
            .functions
            .iter()
            .find(|function| {
                frame_program_counter >= text_relative_start + function.relative_start
                    && frame_program_counter < text_relative_start + function.relative_start + function.len
            })
            .unwrap();
        super::logger::dbg!(function);
        frame_pointer += function.frame_size;
        frame_pointer += 8;
        frame_program_counter = unsafe { (frame_pointer as *const u64).read() };
        super::logger::dbg!(unsafe { ((frame_pointer + 8) as *const u64).read() } as *const u64);
        frame_program_counters.push(frame_program_counter);
        frame_index += 1
    }
    frame_program_counters
}

#[inline(always)]
pub fn capture_root() {
    ROOT.store(*capture_backtrace(Some(1)).last().unwrap(), Ordering::SeqCst);
}
