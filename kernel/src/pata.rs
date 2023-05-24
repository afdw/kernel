use super::sector_storage::{SectorStorage, SECTOR_SIZE};

const PORT_BASE_CONTROL_PRIMARY: u16 = 0x3F6;
const PORT_BASE_CONTROL_SECONDARY: u16 = 0x376;

const PORT_BASE_IO_PRIMARY: u16 = 0x1F0;
const PORT_BASE_IO_SECONDARY: u16 = 0x170;

const PORT_OFFSET_CONTROL: u16 = 0x0;

const PORT_OFFSET_DATA: u16 = 0x0;
const PORT_OFFSET_SECTOR_COUNT: u16 = 0x2;
const PORT_OFFSET_SECTOR_NUMBER: u16 = 0x3;
const PORT_OFFSET_CYLINDER_LOW: u16 = 0x4;
const PORT_OFFSET_CYLINDER_HIGH: u16 = 0x5;
const PORT_OFFSET_DRIVE_HEAD: u16 = 0x6;
const PORT_OFFSET_COMMAND: u16 = 0x7;
const PORT_OFFSET_STATUS: u16 = 0x7;

const CONTROL_BIT_NIEN: u8 = 1 << 3;

const STATUS_BIT_ERR: u8 = 1 << 0;
const STATUS_BIT_DRQ: u8 = 1 << 3;
const STATUS_BIT_BSY: u8 = 1 << 7;

const COMMAND_READ_SECTORS_EXT: u8 = 0x24;
const COMMAND_WRITE_SECTORS_EXT: u8 = 0x34;
const COMMAND_FLUSH_CACHE: u8 = 0xE7;
const COMMAND_IDENTIFY: u8 = 0xEC;

#[derive(Copy, Clone, Debug)]
pub enum Device {
    PrimaryMaster,
    PrimarySlave,
    SecondaryMaster,
    SecondarySlave,
}

impl Device {
    fn port_base_control(self) -> u16 {
        match self {
            Device::PrimaryMaster => PORT_BASE_CONTROL_PRIMARY,
            Device::PrimarySlave => PORT_BASE_CONTROL_PRIMARY,
            Device::SecondaryMaster => PORT_BASE_CONTROL_SECONDARY,
            Device::SecondarySlave => PORT_BASE_CONTROL_SECONDARY,
        }
    }

    fn port_base_io(self) -> u16 {
        match self {
            Device::PrimaryMaster => PORT_BASE_IO_PRIMARY,
            Device::PrimarySlave => PORT_BASE_IO_PRIMARY,
            Device::SecondaryMaster => PORT_BASE_IO_SECONDARY,
            Device::SecondarySlave => PORT_BASE_IO_SECONDARY,
        }
    }

    fn device_bit(self) -> u8 {
        match self {
            Device::PrimaryMaster => 0,
            Device::PrimarySlave => 1 << 4,
            Device::SecondaryMaster => 0,
            Device::SecondarySlave => 1 << 4,
        }
    }
}

pub fn identify(device: Device) -> Option<u64> {
    unsafe {
        let status = x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
        if status == 0xFF {
            return None; // disconnected
        }
        x86::io::outb(device.port_base_control() + PORT_OFFSET_CONTROL, CONTROL_BIT_NIEN); // do not send IRQs
        x86::io::outb(device.port_base_io() + PORT_OFFSET_DRIVE_HEAD, 0xA0 | device.device_bit());
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_COUNT, 0);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_NUMBER, 0);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_LOW, 0);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_HIGH, 0);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_COMMAND, COMMAND_IDENTIFY);
        for _ in 0..15 {
            // wait for the device to be selected (to read the status register of the correct one)
            x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
        }
        loop {
            let status = x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
            if status & STATUS_BIT_BSY != 0 {
                continue;
            }
            if status & STATUS_BIT_ERR != 0 || status & STATUS_BIT_DRQ == 0 {
                return None; // probably not an ATA drive
            }
            break;
        }
        let mut identify_data: [u16; SECTOR_SIZE as usize / 2] = [0; SECTOR_SIZE as usize / 2];
        for word in identify_data.iter_mut() {
            *word = x86::io::inw(device.port_base_io() + PORT_OFFSET_DATA);
        }
        assert!(identify_data[83] & (1 << 10) != 0); // LBA48 mode is supported
        return Some(u64::from_le_bytes(bytemuck::cast_slice(&identify_data[100..104]).try_into().unwrap()));
    }
}

pub fn read_sector(device: Device, sector_index: u64) -> [u8; SECTOR_SIZE as usize] {
    let sector_index_bytes: [u8; 8] = sector_index.to_le_bytes();
    assert!(sector_index_bytes[6] == 0 && sector_index_bytes[7] == 0);
    unsafe {
        x86::io::outb(device.port_base_io() + PORT_OFFSET_DRIVE_HEAD, 0x40 | device.device_bit());
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_COUNT, 0);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_NUMBER, sector_index_bytes[3]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_LOW, sector_index_bytes[4]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_HIGH, sector_index_bytes[5]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_COUNT, 1);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_NUMBER, sector_index_bytes[0]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_LOW, sector_index_bytes[1]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_HIGH, sector_index_bytes[2]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_COMMAND, COMMAND_READ_SECTORS_EXT);
        for _ in 0..15 {
            x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
        }
        loop {
            let status = x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
            if status & STATUS_BIT_BSY != 0 {
                continue;
            }
            assert!(status & STATUS_BIT_ERR == 0 && status & STATUS_BIT_DRQ != 0);
            break;
        }
        let mut sector_data: [u16; SECTOR_SIZE as usize / 2] = [0; SECTOR_SIZE as usize / 2];
        for word in sector_data.iter_mut() {
            *word = x86::io::inw(device.port_base_io() + PORT_OFFSET_DATA);
        }
        return bytemuck::cast_slice(&sector_data).try_into().unwrap();
    }
}

pub fn write_sector(device: Device, sector_index: u64, sector_data: [u8; SECTOR_SIZE as usize]) {
    let sector_index_bytes: [u8; 8] = sector_index.to_le_bytes();
    assert!(sector_index_bytes[6] == 0 && sector_index_bytes[7] == 0);
    let sector_data: [u16; SECTOR_SIZE as usize / 2] = bytemuck::cast_slice(&sector_data).try_into().unwrap();
    unsafe {
        x86::io::outb(device.port_base_io() + PORT_OFFSET_DRIVE_HEAD, 0x40 | device.device_bit());
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_COUNT, 0);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_NUMBER, sector_index_bytes[3]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_LOW, sector_index_bytes[4]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_HIGH, sector_index_bytes[5]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_COUNT, 1);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_SECTOR_NUMBER, sector_index_bytes[0]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_LOW, sector_index_bytes[1]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_CYLINDER_HIGH, sector_index_bytes[2]);
        x86::io::outb(device.port_base_io() + PORT_OFFSET_COMMAND, COMMAND_WRITE_SECTORS_EXT);
        for _ in 0..15 {
            x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
        }
        loop {
            let status = x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
            if status & STATUS_BIT_BSY != 0 {
                continue;
            }
            assert!(status & STATUS_BIT_ERR == 0 && status & STATUS_BIT_DRQ != 0);
            break;
        }
        for word in sector_data {
            x86::io::outw(device.port_base_io() + PORT_OFFSET_DATA, word);
        }
        x86::io::outb(device.port_base_io() + PORT_OFFSET_DRIVE_HEAD, device.device_bit());
        x86::io::outb(device.port_base_io() + PORT_OFFSET_COMMAND, COMMAND_FLUSH_CACHE);
        loop {
            let status = x86::io::inb(device.port_base_io() + PORT_OFFSET_STATUS);
            if status & STATUS_BIT_BSY != 0 {
                continue;
            }
            break;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DiskSectorStorage {
    device: Device,
    sector_count: u64,
}

impl DiskSectorStorage {
    pub fn new(device: Device) -> Option<Self> {
        identify(device).map(|sector_count| DiskSectorStorage { device, sector_count })
    }
}

impl SectorStorage for DiskSectorStorage {
    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read_sector(&self, sector_index: u64) -> [u8; SECTOR_SIZE as usize] {
        assert!(sector_index < self.sector_count);
        read_sector(self.device, sector_index)
    }

    fn write_sector(&self, sector_index: u64, sector_data: [u8; SECTOR_SIZE as usize]) {
        assert!(sector_index < self.sector_count);
        write_sector(self.device, sector_index, sector_data)
    }
}
