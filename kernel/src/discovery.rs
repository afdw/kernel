use acpi::AcpiTable;
use alloc::vec::Vec;

use super::sector_storage::SectorStorage;

#[derive(Clone, Copy)]
struct AcpiHandler;

impl acpi::AcpiHandler for AcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> acpi::PhysicalMapping<Self, T> {
        acpi::PhysicalMapping::new(physical_address, core::ptr::NonNull::new(physical_address as _).unwrap(), size, size, *self)
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {}
}

fn find_mmconfig_base(acpi_tables: &acpi::AcpiTables<AcpiHandler>) -> Option<*mut u8> {
    let mcfg: acpi::PhysicalMapping<AcpiHandler, acpi::mcfg::Mcfg> = unsafe { acpi_tables.get_sdt::<acpi::mcfg::Mcfg>(acpi::sdt::Signature::MCFG) }.ok()??;
    let length = mcfg.header().length as usize - core::mem::size_of::<acpi::mcfg::Mcfg>();
    let num_entries = length / core::mem::size_of::<acpi::mcfg::McfgEntry>();
    let mcfg_entries = unsafe {
        let pointer = (mcfg.virtual_start().as_ref() as *const acpi::mcfg::Mcfg as *const u8).add(core::mem::size_of::<acpi::mcfg::Mcfg>())
            as *const acpi::mcfg::McfgEntry;
        core::slice::from_raw_parts(pointer, num_entries)
    };
    #[repr(C, packed)]
    pub struct McfgEntry {
        base_address: u64,
        pci_segment_group: u16,
        bus_number_start: u8,
        bus_number_end: u8,
        _reserved: u32,
    }
    let mcfg_entries: &[McfgEntry] = unsafe { core::mem::transmute(mcfg_entries) };
    Some(mcfg_entries.iter().find(|mcfg_entry| mcfg_entry.pci_segment_group == 0)?.base_address as _)
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Display {
    Gop(super::gop::Display),
    VirtioGpu(super::virtio_gpu::Display),
}

impl super::display::Display for Display {
    fn reinitialize_if_needed(&self) {
        match self {
            Display::Gop(display) => display.reinitialize_if_needed(),
            Display::VirtioGpu(display) => display.reinitialize_if_needed(),
        }
    }

    fn resolution(&self) -> (usize, usize) {
        match self {
            Display::Gop(display) => display.resolution(),
            Display::VirtioGpu(display) => display.resolution(),
        }
    }

    fn update(&self, pixel_data: &[u32]) {
        match self {
            Display::Gop(display) => display.update(pixel_data),
            Display::VirtioGpu(display) => display.update(pixel_data),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum DiskSectorStorage {
    Pata(super::pata::DiskSectorStorage),
    VirtioBlk(super::virtio_blk::DiskSectorStorage),
}

impl SectorStorage for DiskSectorStorage {
    fn sector_count(&self) -> u64 {
        match self {
            DiskSectorStorage::Pata(disk_sector_storage) => disk_sector_storage.sector_count(),
            DiskSectorStorage::VirtioBlk(disk_sector_storage) => disk_sector_storage.sector_count(),
        }
    }

    fn read_sector(&self, sector_index: u64) -> [u8; super::sector_storage::SECTOR_SIZE as usize] {
        match self {
            DiskSectorStorage::Pata(disk_sector_storage) => disk_sector_storage.read_sector(sector_index),
            DiskSectorStorage::VirtioBlk(disk_sector_storage) => disk_sector_storage.read_sector(sector_index),
        }
    }

    fn write_sector(&self, sector_index: u64, sector_data: [u8; super::sector_storage::SECTOR_SIZE as usize]) {
        match self {
            DiskSectorStorage::Pata(disk_sector_storage) => disk_sector_storage.write_sector(sector_index, sector_data),
            DiskSectorStorage::VirtioBlk(disk_sector_storage) => disk_sector_storage.write_sector(sector_index, sector_data),
        }
    }
}

#[derive(Debug, Default)]
pub struct DiscoveryResult {
    pub displays: Vec<Display>,
    pub disk_sector_storages: Vec<DiskSectorStorage>,
}

pub fn discover() -> DiscoveryResult {
    let mut discovery_result = DiscoveryResult::default();
    log::debug!("UEFI GOP");
    if let Some(display) = super::gop::Display::new() {
        log::info!("--> display of resolution {:?}", super::display::Display::resolution(&display));
        discovery_result.displays.push(Display::Gop(display));
    }
    for device in [
        super::pata::Device::PrimaryMaster,
        super::pata::Device::PrimarySlave,
        super::pata::Device::SecondaryMaster,
        super::pata::Device::SecondarySlave,
    ] {
        log::debug!("PATA device {:?}", device);
        if let Some(disk_sector_storage) = super::pata::DiskSectorStorage::new(device) {
            log::info!("--> disk with {} sectors", disk_sector_storage.sector_count());
            discovery_result.disk_sector_storages.push(DiskSectorStorage::Pata(disk_sector_storage));
        }
    }
    let mut rsdp_address = None;
    let config_table = unsafe { super::SYSTEM_TABLE.as_mut().unwrap() }.config_table();
    for config_table_entry in config_table {
        if config_table_entry.guid == uefi::table::cfg::ACPI2_GUID {
            rsdp_address = Some(config_table_entry.address);
        }
    }
    if rsdp_address.is_none() {
        for config_table_entry in config_table {
            if config_table_entry.guid == uefi::table::cfg::ACPI_GUID {
                rsdp_address = Some(config_table_entry.address);
            }
        }
    }
    if let Some(rsdp_address) = rsdp_address {
        let acpi_tables = unsafe { acpi::AcpiTables::from_rsdp(AcpiHandler, rsdp_address as _) }.unwrap();
        if let Some(mmconfig_base) = find_mmconfig_base(&acpi_tables) {
            log::info!("Found PCIe mmconfig base: {:?}", mmconfig_base);
            let mut pci_root = unsafe { virtio_drivers::transport::pci::bus::PciRoot::new(mmconfig_base, virtio_drivers::transport::pci::bus::Cam::Ecam) };
            for bus in 0..=255 {
                for (device_function, device_function_info) in pci_root.enumerate_bus(bus) {
                    log::info!(
                        "Found PCIe device {}:{}.{}: {}",
                        device_function.bus,
                        device_function.device,
                        device_function.function,
                        device_function_info
                    );
                    if let Some(display) = super::virtio_gpu::Display::new(&mut pci_root, device_function, device_function_info.clone()) {
                        log::info!("--> display of resolution {:?}", super::display::Display::resolution(&display));
                        discovery_result.displays.push(Display::VirtioGpu(display));
                    }
                    if let Some(disk_sector_storage) = super::virtio_blk::DiskSectorStorage::new(&mut pci_root, device_function, device_function_info) {
                        log::info!("--> disk with {} sectors", disk_sector_storage.sector_count());
                        discovery_result.disk_sector_storages.push(DiskSectorStorage::VirtioBlk(disk_sector_storage));
                    }
                }
            }
        }
    }
    discovery_result
}
