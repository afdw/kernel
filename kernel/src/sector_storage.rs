use alloc::vec::Vec;

pub const SECTOR_SIZE: u64 = 512;

pub trait SectorStorage {
    fn sector_count(&self) -> u64;

    fn read_sector(&self, sector_index: u64) -> [u8; SECTOR_SIZE as usize];

    fn write_sector(&self, sector_index: u64, sector_data: [u8; SECTOR_SIZE as usize]);

    fn len(&self) -> u64 {
        self.sector_count() * SECTOR_SIZE
    }

    fn read_aligned(&self, start: u64, len: u64) -> Vec<u8> {
        assert!(start % SECTOR_SIZE == 0 && len % SECTOR_SIZE == 0);
        (0..len / SECTOR_SIZE).flat_map(|index| self.read_sector(start / SECTOR_SIZE + index)).collect()
    }

    fn write_aligned(&self, start: u64, data: &[u8]) {
        assert!(start % SECTOR_SIZE == 0 && data.len() as u64 % SECTOR_SIZE == 0);
        for (index, sector_data) in data.array_chunks().enumerate() {
            self.write_sector(start / SECTOR_SIZE + index as u64, *sector_data);
        }
    }
}

impl<SS: SectorStorage> SectorStorage for &SS {
    fn sector_count(&self) -> u64 {
        (*self).sector_count()
    }

    fn read_sector(&self, sector_index: u64) -> [u8; SECTOR_SIZE as usize] {
        (*self).read_sector(sector_index)
    }

    fn write_sector(&self, sector_index: u64, sector_data: [u8; SECTOR_SIZE as usize]) {
        (*self).write_sector(sector_index, sector_data)
    }
}
