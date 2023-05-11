use super::guid::Guid;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

#[derive(PartialEq, Eq, Debug)]
pub struct Partition {
    type_id: Guid,
    id: Guid,
    starting_sector: u64,
    ending_sector: u64,
    flags: u64,
    name: Option<String>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct PartitionTable {
    id: Guid,
    partitions: Vec<Partition>,
}

pub fn read_partition_table(device: super::disk::Device) -> Option<PartitionTable> {
    let sector_count = super::disk::identify(device).unwrap();
    assert!(sector_count > 1);
    let partition_table_header_data = super::disk::read_sector(device, 1);
    if &partition_table_header_data[0..8] != b"EFI PART" {
        return None;
    }
    let partition_entries_starting_sector = u64::from_le_bytes(bytemuck::cast_slice(&partition_table_header_data[72..80]).try_into().unwrap());
    let partition_count = u32::from_le_bytes(bytemuck::cast_slice(&partition_table_header_data[80..84]).try_into().unwrap()) as u64;
    let partition_entry_size = u32::from_le_bytes(bytemuck::cast_slice(&partition_table_header_data[84..88]).try_into().unwrap()) as u64;
    assert!((super::disk::SECTOR_SIZE as u64) % partition_entry_size == 0);
    Some(PartitionTable {
        id: Guid::from_bytes(bytemuck::cast_slice(&partition_table_header_data[56..72]).try_into().unwrap()),
        partitions: (0..partition_count)
            .map(|partition_index| {
                let partition_entry_offset = partition_index * partition_entry_size;
                let partition_entry_sector_index = partition_entries_starting_sector + partition_entry_offset / (super::disk::SECTOR_SIZE as u64);
                let partition_entry_sector_offset = partition_entry_offset % (super::disk::SECTOR_SIZE as u64);
                assert!(partition_entry_sector_index < sector_count);
                let partition_entry_sector_data = super::disk::read_sector(device, partition_entry_sector_index);
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
                assert!(partition.starting_sector <= partition.ending_sector && partition.ending_sector < sector_count);
                partition
            })
            .filter(|partition| partition.id != super::guid::ZERO)
            .collect(),
    })
}
