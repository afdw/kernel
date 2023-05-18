use object::{
    coff::CoffHeader,
    read::pe::{ImageNtHeaders, ImageOptionalHeader},
    Object, ObjectSection,
};
use pdb::FallibleIterator;
use serde::{Deserialize, Serialize};
use std::{env, error::Error, ffi::OsStr, fs, fs::File, os::unix::prelude::OsStrExt};

include!("structures.rs");

// Taken from https://github.com/gimli-rs/object/blob/ffe0cee99b7fe7d717a8923dccfd6c94fc7afb9a/crates/examples/src/bin/pecopy.rs.
fn copy_file_adding_debug_info(in_data: &[u8], debug_info_data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let in_dos_header = object::pe::ImageDosHeader::parse(in_data)?;
    let mut offset = in_dos_header.nt_headers_offset().into();
    let in_rich_header = object::read::pe::RichHeaderInfo::parse(in_data, offset);
    let (in_nt_headers, in_data_directories) = object::pe::ImageNtHeaders64::parse(in_data, &mut offset)?;
    let in_file_header = in_nt_headers.file_header();
    let in_optional_header = in_nt_headers.optional_header();
    let in_sections = in_file_header.sections(in_data, offset)?;

    let mut out_data = Vec::new();
    let mut writer = object::write::pe::Writer::new(
        in_nt_headers.is_type_64(),
        in_optional_header.section_alignment(),
        in_optional_header.file_alignment(),
        &mut out_data,
    );

    // Reserve file ranges and virtual addresses.
    writer.reserve_dos_header_and_stub();
    if let Some(in_rich_header) = in_rich_header.as_ref() {
        writer.reserve(in_rich_header.length as u32 + 8, 4);
    }
    writer.reserve_nt_headers(in_data_directories.len());

    // Copy data directories that don't have special handling.
    let cert_dir = in_data_directories
        .get(object::pe::IMAGE_DIRECTORY_ENTRY_SECURITY)
        .map(object::pe::ImageDataDirectory::address_range);
    let reloc_dir = in_data_directories
        .get(object::pe::IMAGE_DIRECTORY_ENTRY_BASERELOC)
        .map(object::pe::ImageDataDirectory::address_range);
    for (i, dir) in in_data_directories.iter().enumerate() {
        if dir.virtual_address.get(object::LittleEndian) == 0
            || i == object::pe::IMAGE_DIRECTORY_ENTRY_SECURITY
            || i == object::pe::IMAGE_DIRECTORY_ENTRY_BASERELOC
        {
            continue;
        }
        writer.set_data_directory(i, dir.virtual_address.get(object::LittleEndian), dir.size.get(object::LittleEndian));
    }

    // Determine which sections to copy.
    // We ignore any existing ".reloc" section since we recreate it ourselves.
    let mut in_sections_index = Vec::new();
    for (index, in_section) in in_sections.iter().enumerate() {
        if reloc_dir == Some(in_section.pe_address_range()) {
            continue;
        }
        in_sections_index.push(index + 1);
    }

    let mut out_sections_len = in_sections_index.len();
    if reloc_dir.is_some() {
        out_sections_len += 1;
    }
    out_sections_len += 1; // debug info
    writer.reserve_section_headers(out_sections_len as u16);

    let mut in_sections_data = Vec::new();
    for index in &in_sections_index {
        let in_section = in_sections.section(*index)?;
        let range = writer.reserve_section(
            in_section.name,
            in_section.characteristics.get(object::LittleEndian),
            in_section.virtual_size.get(object::LittleEndian),
            in_section.size_of_raw_data.get(object::LittleEndian),
        );
        assert_eq!(range.virtual_address, in_section.virtual_address.get(object::LittleEndian));
        assert_eq!(range.file_offset, in_section.pointer_to_raw_data.get(object::LittleEndian));
        assert_eq!(range.file_size, in_section.size_of_raw_data.get(object::LittleEndian));
        in_sections_data.push((range.file_offset, in_section.pe_data(in_data)?));
    }

    if reloc_dir.is_some() {
        let mut blocks = in_data_directories.relocation_blocks(in_data, &in_sections)?.unwrap();
        while let Some(block) = blocks.next()? {
            for reloc in block {
                writer.add_reloc(reloc.virtual_address, reloc.typ);
            }
        }
        writer.reserve_reloc_section();
    }

    let dbg_info_file_offset = writer
        .reserve_section(
            *b"dbg_info",
            object::pe::IMAGE_SCN_LNK_INFO | object::pe::IMAGE_SCN_MEM_READ,
            debug_info_data.len().try_into()?,
            debug_info_data.len().try_into()?,
        )
        .file_offset;

    if let Some((_, size)) = cert_dir {
        // TODO: reserve individual certificates
        writer.reserve_certificate_table(size);
    }

    // Start writing.
    writer.write_dos_header_and_stub()?;
    if let Some(in_rich_header) = in_rich_header.as_ref() {
        // TODO: recalculate xor key
        writer.write_align(4);
        writer.write(&in_data[in_rich_header.offset..][..in_rich_header.length + 8]);
    }
    writer.write_nt_headers(object::write::pe::NtHeaders {
        machine: in_file_header.machine.get(object::LittleEndian),
        time_date_stamp: in_file_header.time_date_stamp.get(object::LittleEndian),
        characteristics: in_file_header.characteristics.get(object::LittleEndian),
        major_linker_version: in_optional_header.major_linker_version(),
        minor_linker_version: in_optional_header.minor_linker_version(),
        address_of_entry_point: in_optional_header.address_of_entry_point(),
        image_base: in_optional_header.image_base(),
        major_operating_system_version: in_optional_header.major_operating_system_version(),
        minor_operating_system_version: in_optional_header.minor_operating_system_version(),
        major_image_version: in_optional_header.major_image_version(),
        minor_image_version: in_optional_header.minor_image_version(),
        major_subsystem_version: in_optional_header.major_subsystem_version(),
        minor_subsystem_version: in_optional_header.minor_subsystem_version(),
        subsystem: in_optional_header.subsystem(),
        dll_characteristics: in_optional_header.dll_characteristics(),
        size_of_stack_reserve: in_optional_header.size_of_stack_reserve(),
        size_of_stack_commit: in_optional_header.size_of_stack_commit(),
        size_of_heap_reserve: in_optional_header.size_of_heap_reserve(),
        size_of_heap_commit: in_optional_header.size_of_heap_commit(),
    });
    writer.write_section_headers();
    for (offset, data) in in_sections_data {
        writer.write_section(offset, data);
    }
    writer.write_reloc_section();
    writer.write_section(dbg_info_file_offset, debug_info_data);
    if let Some((address, size)) = cert_dir {
        // TODO: write individual certificates
        writer.write_certificate_table(&in_data[address as usize..][..size as usize]);
    }

    assert_eq!(writer.reserved_len() as usize, writer.len());

    Ok(out_data)
}

fn main() -> Result<(), Box<dyn Error>> {
    let input_file_path = env::args().nth(1).ok_or("not enough arguments")?;
    let output_file_path = env::args().nth(2).ok_or("not enough arguments")?;
    let input_file_data = fs::read(input_file_path)?;
    let mut debug_info = DebugInfo::default();
    let input_object = object::File::parse(&input_file_data[..])?;
    let text_data = input_object.section_by_name(".text").ok_or("no text section")?.data()?;
    let mut decoder = iced_x86::Decoder::new(64, text_data, 0);
    let pdb_file_path = input_object.pdb_info()?.ok_or("no PDB info")?.path();
    let mut pdb = pdb::PDB::open(File::open(OsStr::from_bytes(pdb_file_path))?)?;
    let address_map = pdb.address_map()?;
    let frame_table = pdb.frame_table()?;
    let mut frames = frame_table.iter();
    while let Some(frame) = frames.next()? {
        println!("{:#?}", frame);
    }
    let string_table = pdb.string_table()?;
    let debug_information = pdb.debug_information()?;
    let mut modules = debug_information.modules()?;
    while let Some(module) = modules.next()? {
        if let Some(module_info) = pdb.module_info(&module)? {
            let mut symbols = module_info.symbols()?;
            while let Some(symbol) = symbols.next()? {
                match symbol.parse() {
                    Ok(symbol_data) => match symbol_data {
                        pdb::SymbolData::Procedure(procedure_symbol) => {
                            decoder.set_position(procedure_symbol.offset.offset as usize)?;
                            let first_instruction = decoder.decode();
                            if first_instruction.code() != iced_x86::Code::Sub_rm64_imm8
                                && first_instruction.code() != iced_x86::Code::Sub_rm64_imm32
                                && first_instruction.code() != iced_x86::Code::Push_r64
                            {
                                dbg!(first_instruction.code());
                                use iced_x86::Formatter;
                                decoder.set_position(procedure_symbol.offset.offset as usize)?;
                                let mut formatter = iced_x86::NasmFormatter::new();
                                while decoder.position() < (procedure_symbol.offset.offset + procedure_symbol.len) as usize {
                                    let instruction = decoder.decode();
                                    let mut output = String::new();
                                    formatter.format(&instruction, &mut output);
                                    dbg!(output);
                                }
                                dbg!("---");
                            }
                            let mut frame_size = 0;
                            decoder.set_position(procedure_symbol.offset.offset as usize)?;
                            while decoder.position() < (procedure_symbol.offset.offset + procedure_symbol.len) as usize {
                                let instruction = decoder.decode();
                                match instruction.code() {
                                    iced_x86::Code::Push_r64 => frame_size += 8,
                                    iced_x86::Code::Sub_rm64_imm8 | iced_x86::Code::Sub_rm64_imm32 => {
                                        assert_eq!(instruction.op0_register(), iced_x86::Register::RSP);
                                        frame_size += instruction.immediate(1);
                                        break;
                                    }
                                    iced_x86::Code::Retnq => break,
                                    _ => break,
                                }
                            }
                            debug_info.functions.push(Function {
                                name: procedure_symbol.name.to_string().into_owned(),
                                relative_start: procedure_symbol.offset.offset as u64,
                                start: procedure_symbol
                                    .offset
                                    .to_rva(&address_map)
                                    .ok_or("unable to resolve an actual Relative Virtual Address in the executable's address space")?
                                    .0 as u64,
                                len: procedure_symbol.len as u64,
                                frame_size,
                            })
                        }
                        _ => (),
                    },
                    Err(pdb::Error::UnimplementedSymbolKind(..)) => (),
                    Err(err) => return Err(err.into()),
                }
            }
            let line_program = module_info.line_program()?;
            let mut lines = line_program.lines();
            while let Some(line_info) = lines.next()? {
                let file_info = line_program.get_file_info(line_info.file_index)?;
                debug_info.regions.push(Region {
                    start: line_info
                        .offset
                        .to_rva(&address_map)
                        .ok_or("unable to resolve an actual Relative Virtual Address in the executable's address space")?
                        .0 as u64,
                    len: line_info.length.ok_or("no length")? as u64,
                    file: file_info.name.to_string_lossy(&string_table)?.into_owned(),
                    line: line_info.line_start as u64,
                });
            }
        }
    }
    let debug_info_data = serde_json::to_vec(&debug_info)?;
    let output_file_data = copy_file_adding_debug_info(&input_file_data, &debug_info_data)?;
    fs::write(output_file_path, output_file_data)?;
    println!("{}", String::from_utf8(debug_info_data.clone())?);
    Ok(())
}
