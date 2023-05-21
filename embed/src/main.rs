use object::{Object, ObjectSection};
use pdb::FallibleIterator;
use std::{env, error::Error, ffi::OsStr, fs, fs::File, os::unix::prelude::OsStrExt};

include!("common.rs");

fn main() -> Result<(), Box<dyn Error>> {
    let bootloader_file_path = env::args().nth(1).ok_or("not enough arguments")?;
    let kernel_file_path = env::args().nth(2).ok_or("not enough arguments")?;
    let bundle_file_path = env::args().nth(3).ok_or("not enough arguments")?;
    let bootloader_file_data = fs::read(bootloader_file_path)?;
    let kernel_file_data = fs::read(kernel_file_path)?;
    let mut bundle_file_data = bootloader_file_data.clone();
    let object = object::File::parse(&bootloader_file_data[..])?;
    let pdb_file_path = object.pdb_info()?.ok_or("no PDB info")?.path();
    let mut pdb = pdb::PDB::open(File::open(OsStr::from_bytes(pdb_file_path))?)?;
    let sections = pdb.sections()?.ok_or("no sections")?;
    let debug_information = pdb.debug_information()?;
    let mut modules = debug_information.modules()?;
    while let Some(module) = modules.next()? {
        if let Some(module_info) = pdb.module_info(&module)? {
            let mut symbols = module_info.symbols()?;
            while let Some(symbol) = symbols.next()? {
                match symbol.parse() {
                    Ok(pdb::SymbolData::Data(data_symbol)) => {
                        if data_symbol.name == "bootloader::KERNEL_FILE_DATA".into() {
                            assert!(data_symbol.offset.section != 0);
                            let section_name = sections[data_symbol.offset.section as usize - 1].name();
                            let section = object.section_by_name(section_name).ok_or("section not found")?;
                            let (section_start, section_length) = section.file_range().ok_or("no file range")?;
                            let section_in_bundle = &mut bundle_file_data[section_start as usize..(section_start + section_length) as usize];
                            let kernel_file_data_in_bindle =
                                &mut section_in_bundle[data_symbol.offset.offset as usize..data_symbol.offset.offset as usize + KERNEL_FILE_DATA_MAX_SIZE];
                            assert_eq!(kernel_file_data_in_bindle, &[KERNEL_FILE_DATA_FILLER; KERNEL_FILE_DATA_MAX_SIZE]);
                            assert!(kernel_file_data.len() <= KERNEL_FILE_DATA_MAX_SIZE);
                            kernel_file_data_in_bindle[..kernel_file_data.len()].copy_from_slice(&kernel_file_data);
                        }
                    }
                    Ok(_) => (),
                    Err(pdb::Error::UnimplementedSymbolKind(..)) => (),
                    Err(err) => return Err(err.into()),
                }
            }
        }
    }
    fs::write(bundle_file_path, bundle_file_data)?;
    Ok(())
}
