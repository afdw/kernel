use alloc::{borrow::Cow, format, rc::Rc, string::String, vec::Vec};

static mut ADDR2LINE_CONTEXT: Option<addr2line::Context<gimli::EndianReader<gimli::RunTimeEndian, Rc<[u8]>>>> = None;

pub fn init() {
    let object_file = &object::read::File::parse(super::BOOTLOADER_PROTOCOL.wait().kernel_file_data).unwrap();
    let dwarf = gimli::Dwarf::load::<_, !>(|id| {
        use object::{Object, ObjectSection};

        Ok(gimli::EndianRcSlice::new(
            Rc::from(
                object_file
                    .section_by_name(id.name())
                    .and_then(|section| section.uncompressed_data().ok())
                    .unwrap_or(Cow::Borrowed(&[])),
            ),
            if object_file.is_little_endian() {
                gimli::RunTimeEndian::Little
            } else {
                gimli::RunTimeEndian::Big
            },
        ))
    })
    .unwrap();
    let addr2line_context = addr2line::Context::from_dwarf(dwarf).unwrap();
    unsafe {
        ADDR2LINE_CONTEXT = Some(addr2line_context);
    }
}

pub fn capture_backtrace() -> Vec<usize> {
    let mut frame_program_counters = Vec::new();
    extern "C" fn trace(ctx: &mut unwinding::abi::UnwindContext<'_>, arg: *mut core::ffi::c_void) -> unwinding::abi::UnwindReasonCode {
        let frame_program_counters = unsafe { &mut *(arg as *mut Vec<usize>) };
        frame_program_counters.push(unwinding::abi::_Unwind_GetIP(ctx));
        unwinding::abi::UnwindReasonCode::NO_REASON
    }
    unwinding::abi::_Unwind_Backtrace(trace, &mut frame_program_counters as *mut Vec<usize> as *mut core::ffi::c_void);
    frame_program_counters
}

pub fn symbolize_program_counter(program_counter: usize) -> String {
    let addr2line_context = unsafe { ADDR2LINE_CONTEXT.as_ref().unwrap() };
    let mut parts = Vec::new();
    if let Some(relative_program_counter) = program_counter.checked_sub(super::BOOTLOADER_PROTOCOL.wait().memory_image_start as usize) {
        if let addr2line::LookupResult::Output(Ok(mut frame_iter)) = addr2line_context.find_frames(relative_program_counter as u64) {
            while let Ok(Some(frame)) = frame_iter.next() {
                let function = frame
                    .function
                    .map(|function_name| format!("{:#}", rustc_demangle::demangle(&String::from_utf8_lossy(function_name.name.bytes()))));
                let location = frame.location.map(|location| {
                    format!(
                        "{}:{}:{}",
                        location.file.unwrap_or("?"),
                        location.line.unwrap_or(0),
                        location.column.unwrap_or(0)
                    )
                });
                match (function, location) {
                    (Some(function), Some(location)) => parts.push(format!("{} at {}", function, location)),
                    (Some(function), None) => parts.push(function),
                    (None, Some(location)) => parts.push(format!("at {}", location)),
                    (None, None) => (),
                }
            }
        }
    }
    if parts.is_empty() {
        format!("{:?}", program_counter as *const u8)
    } else {
        parts.join(", ")
    }
}

pub fn symbolize_backtrace(frame_program_counters: &[usize]) -> String {
    let mut output = String::new();
    for (frame_index, &frame_program_counter) in frame_program_counters.iter().enumerate() {
        output += &format!("{}. {}\n", frame_index, symbolize_program_counter(frame_program_counter));
    }
    output
}
