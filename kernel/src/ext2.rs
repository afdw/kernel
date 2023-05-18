use acid_io::{
    byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt},
    Cursor,
};
use alloc::vec::Vec;
use bitflags::bitflags;
use core::{cmp::min, ops::Range};

use crate::sector_storage::SectorStorage;

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct FeaturesCompat: u32 {
        const DIR_PREALLOC = 0x0001;
        const IMAGIC_INODES = 0x0002;
        const HAS_JOURNAL = 0x0004;
        const EXT_ATTR = 0x0008;
        const RESIZE_INO = 0x0010;
        const DIR_INDEX = 0x0020;
    }

    #[derive(Clone, Copy, Debug)]
    struct FeaturesIncompat: u32 {
        const COMPRESSION = 0x0001;
        const FILETYPE = 0x0002;
        const RECOVER = 0x0004;
        const JOURNAL_DEV = 0x0008;
        const META_BG = 0x0010;
    }

    #[derive(Clone, Copy, Debug)]
    struct FeaturesRoCompat: u32 {
        const SPARSE_SUPER = 0x0001;
        const LARGE_FILE = 0x0002;
        const BTREE_DIR = 0x0004;
    }
}

#[derive(Clone, Copy, Debug)]
struct Superblock {
    inodes_count: u32,
    blocks_count: u64,
    reserved_blocks_count: u64, // r_blocks_count
    free_blocks_count: u64,
    free_inodes_count: u32,
    first_data_block: u64,
    log_block_size: u32,
    block_count_per_group: u64, // blocks_per_group
    inode_count_per_group: u32, // inodes_per_group
    mount_time: u64,            // mtime
    write_time: u64,            // wtime
    mount_count: u16,           // mnt_count
    max_mount_count: u16,       // max_mnt_count
    state: u16,
    errors: u16,
    minor_revision_level: u16, // minor_rev_level
    last_check_time: u64,      // lastcheck
    check_interval: u64,       // checkinterval
    creator_os: u32,
    revision_level: u32,       // rev_level
    default_reserved_uid: u16, // def_resuid
    default_reserved_gid: u16, // def_resgid
    first_inode: u32,          // first_ino
    inode_size: u64,
    block_group_index: u64, // block_group_nr
    features_compat: FeaturesCompat,
    features_incompat: FeaturesIncompat,
    features_ro_compat: FeaturesRoCompat,
}

impl Superblock {
    const INITIAL_START: u64 = 1024;
    const LENGTH: u64 = 1024;
    const MAGIC: u16 = 0xEF53;

    fn of_bytes(superblock_data: &[u8]) -> Superblock {
        let mut superblock_data_cursor = Cursor::new(superblock_data);
        let inodes_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let blocks_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let reserved_blocks_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let free_blocks_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let free_inodes_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let first_data_block = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let log_block_size = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let log_fragment_size = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let block_count_per_group = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let fragment_count_per_group = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let inode_count_per_group = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let mount_time = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let write_time = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let mount_count = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let max_mount_count = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let magic = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let state = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let errors = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let minor_revision_level = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let last_check_time = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let check_interval = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let creator_os = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let revision_level = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let default_reserved_uid = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let default_reserved_gid = superblock_data_cursor.read_u16::<LittleEndian>().unwrap();
        let first_inode = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let inode_size = superblock_data_cursor.read_u16::<LittleEndian>().unwrap() as u64;
        let block_group_index = superblock_data_cursor.read_u16::<LittleEndian>().unwrap() as u64;
        let features_compat = FeaturesCompat::from_bits_retain(superblock_data_cursor.read_u32::<LittleEndian>().unwrap());
        let features_incompat = FeaturesIncompat::from_bits_retain(superblock_data_cursor.read_u32::<LittleEndian>().unwrap());
        let features_ro_compat = FeaturesRoCompat::from_bits_retain(superblock_data_cursor.read_u32::<LittleEndian>().unwrap());
        assert_eq!(log_block_size, log_fragment_size);
        assert_eq!(block_count_per_group, fragment_count_per_group);
        assert_eq!(magic, Superblock::MAGIC);
        Superblock {
            inodes_count,
            blocks_count,
            reserved_blocks_count,
            free_blocks_count,
            free_inodes_count,
            first_data_block,
            log_block_size,
            block_count_per_group,
            inode_count_per_group,
            mount_time,
            write_time,
            mount_count,
            max_mount_count,
            state,
            errors,
            minor_revision_level,
            last_check_time,
            check_interval,
            creator_os,
            revision_level,
            default_reserved_uid,
            default_reserved_gid,
            first_inode,
            inode_size,
            block_group_index,
            features_compat,
            features_incompat,
            features_ro_compat,
        }
    }

    fn update_bytes(self, superblock_data: &mut [u8]) {
        let mut superblock_data_cursor = Cursor::new(superblock_data);
        superblock_data_cursor.write_u32::<LittleEndian>(self.inodes_count).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.blocks_count.try_into().unwrap()).unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.reserved_blocks_count.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.free_blocks_count.try_into().unwrap())
            .unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.free_inodes_count).unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.first_data_block.try_into().unwrap())
            .unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.log_block_size).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.log_block_size).unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.block_count_per_group.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.block_count_per_group.try_into().unwrap())
            .unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.inode_count_per_group).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.mount_time.try_into().unwrap()).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.write_time.try_into().unwrap()).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.mount_count).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.max_mount_count).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(Superblock::MAGIC).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.state).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.errors).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.minor_revision_level).unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.last_check_time.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.check_interval.try_into().unwrap())
            .unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.creator_os).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.revision_level).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.default_reserved_uid).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.default_reserved_gid).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.first_inode).unwrap();
        superblock_data_cursor.write_u16::<LittleEndian>(self.inode_size.try_into().unwrap()).unwrap();
        superblock_data_cursor
            .write_u16::<LittleEndian>(self.block_group_index.try_into().unwrap())
            .unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.features_compat.bits()).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.features_incompat.bits()).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.features_ro_compat.bits()).unwrap();
    }

    fn block_size(self) -> u64 {
        1024 << self.log_block_size
    }
}

#[derive(Debug)]
pub struct Mounted<'ss, SS: SectorStorage> {
    sector_storage: &'ss SS,
    superblock: Superblock,
}

impl<'ss, SS: SectorStorage> Mounted<'ss, SS> {
    pub fn new(sector_storage: &'ss SS) -> Self {
        let superblock = Superblock::of_bytes(&sector_storage.read_aligned(Superblock::INITIAL_START, Superblock::LENGTH));
        Mounted { sector_storage, superblock }
    }

    fn read_block(&self, block_index: u64) -> Vec<u8> {
        self.sector_storage
            .read_aligned(block_index * self.superblock.block_size(), self.superblock.block_size())
    }

    fn write_block(&self, block_index: u64, block_data: &[u8]) {
        self.sector_storage.write_aligned(block_index * self.superblock.block_size(), block_data)
    }

    fn block_group_ranges(&self) -> impl Iterator<Item = Range<u64>> + '_ {
        (0..)
            .map(|block_group_index| self.superblock.first_data_block + block_group_index * self.superblock.block_count_per_group)
            .map(|block_group_start| Range {
                start: block_group_start,
                end: min(
                    block_group_start + self.superblock.block_count_per_group,
                    self.sector_storage.len() / self.superblock.block_size(),
                ),
            })
            .take_while(|block_group_range| !block_group_range.is_empty())
    }

    fn block_group_index_contains_superblock_copies(&self, block_group_index: u64) -> bool {
        !self.superblock.features_ro_compat.contains(FeaturesRoCompat::SPARSE_SUPER)
            || block_group_index == 0
            || [3, 5, 7]
                .into_iter()
                .any(|base: u64| base.pow(block_group_index.ilog(base)) == block_group_index)
    }

    #[allow(clippy::iter_nth_zero)]
    pub fn write_superblock_copies(&self) {
        for (block_group_index, block_group_range) in self.block_group_ranges().enumerate() {
            let block_group_index = block_group_index as u64;
            if self.block_group_index_contains_superblock_copies(block_group_index) {
                let mut superblock_data = self.read_block(block_group_range.clone().nth(0).unwrap());
                Superblock {
                    block_group_index,
                    ..self.superblock
                }
                .update_bytes(&mut superblock_data);
                self.write_block(block_group_range.clone().nth(0).unwrap(), &superblock_data);
            }
        }
    }
}
