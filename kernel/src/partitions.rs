use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use super::{
    guid::Guid,
    sector_storage::{SectorStorage, SECTOR_SIZE},
};

#[derive(PartialEq, Eq, Debug)]
pub struct Partition {
    pub type_id: Guid,
    pub id: Guid,
    pub starting_sector: u64,
    pub ending_sector: u64, // inclusive
    pub flags: u64,
    pub name: Option<String>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct PartitionTable {
    pub id: Guid,
    pub partitions: Vec<Partition>,
}

pub fn read_partition_table<SS: SectorStorage>(sector_storage: &SS) -> Option<PartitionTable> {
    assert!(sector_storage.sector_count() > 1);
    let partition_table_header_data = sector_storage.read_sector(1);
    if &partition_table_header_data[0..8] != b"EFI PART" {
        return None;
    }
    let partition_entries_starting_sector = u64::from_le_bytes(bytemuck::cast_slice(&partition_table_header_data[72..80]).try_into().unwrap());
    let partition_count = u32::from_le_bytes(bytemuck::cast_slice(&partition_table_header_data[80..84]).try_into().unwrap()) as u64;
    let partition_entry_size = u32::from_le_bytes(bytemuck::cast_slice(&partition_table_header_data[84..88]).try_into().unwrap()) as u64;
    assert!(SECTOR_SIZE % partition_entry_size == 0);
    Some(PartitionTable {
        id: Guid::from_bytes(bytemuck::cast_slice(&partition_table_header_data[56..72]).try_into().unwrap()),
        partitions: (0..partition_count)
            .map(|partition_index| {
                let partition_entry_offset = partition_index * partition_entry_size;
                let partition_entry_sector_index = partition_entries_starting_sector + partition_entry_offset / SECTOR_SIZE;
                let partition_entry_sector_offset = partition_entry_offset % SECTOR_SIZE;
                assert!(partition_entry_sector_index < sector_storage.sector_count());
                let partition_entry_sector_data = sector_storage.read_sector(partition_entry_sector_index);
                let partition_entry_data =
                    &partition_entry_sector_data[partition_entry_sector_offset as usize..(partition_entry_sector_offset + partition_entry_size) as usize];
                let mut name: &[u16] = bytemuck::cast_slice(&partition_entry_data[56..]);
                while let Some((0, chopped_name)) = name.split_last() {
                    if let Some(0) = chopped_name.last() {
                        name = chopped_name;
                    } else {
                        break;
                    }
                }
                let partition = Partition {
                    type_id: Guid::from_bytes(bytemuck::cast_slice(&partition_entry_data[0..16]).try_into().unwrap()),
                    id: Guid::from_bytes(bytemuck::cast_slice(&partition_entry_data[16..32]).try_into().unwrap()),
                    starting_sector: u64::from_le_bytes(bytemuck::cast_slice(&partition_entry_data[32..40]).try_into().unwrap()),
                    ending_sector: u64::from_le_bytes(bytemuck::cast_slice(&partition_entry_data[40..48]).try_into().unwrap()),
                    flags: u64::from_le_bytes(bytemuck::cast_slice(&partition_entry_data[48..56]).try_into().unwrap()),
                    name: uefi::data_types::CStr16::from_u16_with_nul(name).map(ToString::to_string).ok(),
                };
                assert!(partition.starting_sector <= partition.ending_sector && partition.ending_sector < sector_storage.sector_count());
                partition
            })
            .filter(|partition| partition.id != super::guid::ZERO)
            .collect(),
    })
}

impl<SS: SectorStorage> SectorStorage for (SS, Partition) {
    fn sector_count(&self) -> u64 {
        self.1.ending_sector - self.1.starting_sector + 1
    }

    fn read_sector(&self, sector_index: u64) -> [u8; SECTOR_SIZE as usize] {
        let (sector_storage, partition) = self;
        assert!(sector_index < self.sector_count());
        sector_storage.read_sector(partition.starting_sector + sector_index)
    }

    fn write_sector(&self, sector_index: u64, sector_data: [u8; SECTOR_SIZE as usize]) {
        let (sector_storage, partition) = self;
        assert!(sector_index < self.sector_count());
        sector_storage.write_sector(partition.starting_sector + sector_index, sector_data);
    }
}
