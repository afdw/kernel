use acid_io::{
    byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt},
    Cursor, Read, Write,
};
use alloc::{string::String, vec, vec::Vec};
use bitflags::bitflags;
use core::{cmp::min, ops::Range};

use super::{
    fs::{FileStat, FileType, Mode},
    sector_storage::SectorStorage,
};

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
    inodes_count: u64,
    blocks_count: u64,
    reserved_blocks_count: u64, // r_blocks_count
    free_blocks_count: u64,
    free_inodes_count: u64,
    first_data_block_block_index: u64, // first_data_block
    log_block_size: u32,
    block_count_per_block_group: u64, // blocks_per_group
    inode_count_per_block_group: u64, // inodes_per_group
    mount_time: u64,                  // mtime
    write_time: u64,                  // wtime
    mount_count: u16,                 // mnt_count
    max_mount_count: u16,             // max_mnt_count
    state: u16,
    errors: u16,
    minor_revision_level: u16, // minor_rev_level
    last_check_time: u64,      // lastcheck
    check_interval: u64,       // checkinterval
    creator_os: u32,
    revision_level: u32,           // rev_level
    default_reserved_uid: u16,     // def_resuid
    default_reserved_gid: u16,     // def_resgid
    first_usable_inode_index: u64, // first_ino
    inode_size: u64,
    block_group_index: u64, // block_group_nr
    features_compat: FeaturesCompat,
    features_incompat: FeaturesIncompat,
    features_ro_compat: FeaturesRoCompat,
}

impl Superblock {
    const INITIAL_START: u64 = 1024;
    const MAGIC: u16 = 0xEF53;
    const SIZE: u64 = 1024;

    fn of_bytes(superblock_data: &[u8]) -> Self {
        let mut superblock_data_cursor = Cursor::new(superblock_data);
        let inodes_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let blocks_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let reserved_blocks_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let free_blocks_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let free_inodes_count = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let first_data_block_block_index = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let log_block_size = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let log_fragment_size = superblock_data_cursor.read_u32::<LittleEndian>().unwrap();
        let block_count_per_block_group = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let fragment_count_per_group = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let inode_count_per_block_group = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
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
        let first_usable_inode_index = superblock_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let inode_size = superblock_data_cursor.read_u16::<LittleEndian>().unwrap() as u64;
        let block_group_index = superblock_data_cursor.read_u16::<LittleEndian>().unwrap() as u64;
        let features_compat = FeaturesCompat::from_bits_retain(superblock_data_cursor.read_u32::<LittleEndian>().unwrap());
        let features_incompat = FeaturesIncompat::from_bits_retain(superblock_data_cursor.read_u32::<LittleEndian>().unwrap());
        let features_ro_compat = FeaturesRoCompat::from_bits_retain(superblock_data_cursor.read_u32::<LittleEndian>().unwrap());
        assert_eq!(log_block_size, log_fragment_size);
        assert_eq!(block_count_per_block_group, fragment_count_per_group);
        assert_eq!(magic, Superblock::MAGIC);
        Superblock {
            inodes_count,
            blocks_count,
            reserved_blocks_count,
            free_blocks_count,
            free_inodes_count,
            first_data_block_block_index,
            log_block_size,
            block_count_per_block_group,
            inode_count_per_block_group,
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
            first_usable_inode_index,
            inode_size,
            block_group_index,
            features_compat,
            features_incompat,
            features_ro_compat,
        }
    }

    fn update_bytes(self, superblock_data: &mut [u8]) {
        let mut superblock_data_cursor = Cursor::new(superblock_data);
        superblock_data_cursor.write_u32::<LittleEndian>(self.inodes_count.try_into().unwrap()).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.blocks_count.try_into().unwrap()).unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.reserved_blocks_count.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.free_blocks_count.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.free_inodes_count.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.first_data_block_block_index.try_into().unwrap())
            .unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.log_block_size).unwrap();
        superblock_data_cursor.write_u32::<LittleEndian>(self.log_block_size).unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.block_count_per_block_group.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.block_count_per_block_group.try_into().unwrap())
            .unwrap();
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.inode_count_per_block_group.try_into().unwrap())
            .unwrap();
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
        superblock_data_cursor
            .write_u32::<LittleEndian>(self.first_usable_inode_index.try_into().unwrap())
            .unwrap();
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

#[derive(Clone, Copy, Debug)]
struct BlockGroupDescriptor {
    block_bitmap_block_index: u64,      // block_bitmap
    inode_bitmap_block_index: u64,      // inode_bitmap
    inode_table_first_block_index: u64, // inode_table
    free_blocks_count: u64,
    free_inodes_count: u32,
    used_dirs_count: u32,
}

impl BlockGroupDescriptor {
    const SIZE: u64 = 32;

    fn of_bytes(block_group_descriptor_data: &[u8]) -> Self {
        let mut block_group_descriptor_data_cursor = Cursor::new(block_group_descriptor_data);
        let block_bitmap_block_index = block_group_descriptor_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let inode_bitmap_block_index = block_group_descriptor_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let inode_table_first_block_index = block_group_descriptor_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let free_blocks_count = block_group_descriptor_data_cursor.read_u16::<LittleEndian>().unwrap() as u64;
        let free_inodes_count = block_group_descriptor_data_cursor.read_u16::<LittleEndian>().unwrap() as u32;
        let used_dirs_count = block_group_descriptor_data_cursor.read_u16::<LittleEndian>().unwrap() as u32;
        BlockGroupDescriptor {
            block_bitmap_block_index,
            inode_bitmap_block_index,
            inode_table_first_block_index,
            free_blocks_count,
            free_inodes_count,
            used_dirs_count,
        }
    }

    fn update_bytes(self, block_group_descriptor_data: &mut [u8]) {
        let mut block_group_descriptor_data_cursor = Cursor::new(block_group_descriptor_data);
        block_group_descriptor_data_cursor
            .write_u32::<LittleEndian>(self.block_bitmap_block_index.try_into().unwrap())
            .unwrap();
        block_group_descriptor_data_cursor
            .write_u32::<LittleEndian>(self.inode_bitmap_block_index.try_into().unwrap())
            .unwrap();
        block_group_descriptor_data_cursor
            .write_u32::<LittleEndian>(self.inode_table_first_block_index.try_into().unwrap())
            .unwrap();
        block_group_descriptor_data_cursor
            .write_u16::<LittleEndian>(self.free_blocks_count.try_into().unwrap())
            .unwrap();
        block_group_descriptor_data_cursor
            .write_u16::<LittleEndian>(self.free_inodes_count.try_into().unwrap())
            .unwrap();
        block_group_descriptor_data_cursor
            .write_u16::<LittleEndian>(self.used_dirs_count.try_into().unwrap())
            .unwrap();
    }
}

#[derive(Debug)]
struct Bitmap {
    data: Vec<u8>,
}

impl Bitmap {
    fn get(&self, index: usize) -> bool {
        self.data[index / 8] >> (index % 8) != 0
    }

    fn set(&mut self, index: usize, value: bool) {
        if value {
            self.data[index / 8] |= 1 << (index % 8);
        } else {
            self.data[index / 8] &= !(1 << (index % 8));
        }
    }
}

#[derive(Clone, Debug)]
struct Inode {
    mode: Mode,
    uid: u16,
    size: u64,
    access_time: u64,       // atime
    creation_time: u64,     // ctime
    modification_time: u64, // mtime
    deletion_time: u64,     // dtime
    gid: u16,
    links_count: u16,
    sector_count: u64, // blocks
    flags: u32,
    os_dependent_1: [u8; 4],   // osd1
    data_block_map: [u64; 15], // block
    generation: u32,
    file_acl: u32,
    dir_arc: u32,
    faddr: u32,
    os_dependent_2: [u8; 12], // osd2
}

impl Inode {
    const PRACTICAL_SIZE: u64 = 128;

    fn of_bytes(inode_data: &[u8]) -> Self {
        let mut inode_data_cursor = Cursor::new(inode_data);
        let mode = Mode::from_bits_retain(inode_data_cursor.read_u16::<LittleEndian>().unwrap() as u32);
        let uid = inode_data_cursor.read_u16::<LittleEndian>().unwrap();
        let size = inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let access_time = inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let creation_time = inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let modification_time = inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let deletion_time = inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let gid = inode_data_cursor.read_u16::<LittleEndian>().unwrap();
        let links_count = inode_data_cursor.read_u16::<LittleEndian>().unwrap();
        let sector_count = inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let flags = inode_data_cursor.read_u32::<LittleEndian>().unwrap();
        let mut os_dependent_1 = [0; 4];
        inode_data_cursor.read_exact(&mut os_dependent_1).unwrap();
        let data_block_map = (0..15)
            .map(|_| inode_data_cursor.read_u32::<LittleEndian>().unwrap() as u64)
            .collect::<Vec<u64>>()
            .try_into()
            .unwrap();
        let generation = inode_data_cursor.read_u32::<LittleEndian>().unwrap();
        let file_acl = inode_data_cursor.read_u32::<LittleEndian>().unwrap();
        let dir_arc = inode_data_cursor.read_u32::<LittleEndian>().unwrap();
        let faddr = inode_data_cursor.read_u32::<LittleEndian>().unwrap();
        let mut os_dependent_2 = [0; 12];
        inode_data_cursor.read_exact(&mut os_dependent_2).unwrap();
        assert_eq!(inode_data_cursor.position(), Inode::PRACTICAL_SIZE);
        Inode {
            mode,
            uid,
            size,
            access_time,
            creation_time,
            modification_time,
            deletion_time,
            gid,
            links_count,
            sector_count,
            flags,
            os_dependent_1,
            data_block_map,
            generation,
            file_acl,
            dir_arc,
            faddr,
            os_dependent_2,
        }
    }

    fn update_bytes(&self, inode_data: &mut [u8]) {
        let mut inode_data_cursor = Cursor::new(inode_data);
        inode_data_cursor.write_u16::<LittleEndian>(self.mode.bits().try_into().unwrap()).unwrap();
        inode_data_cursor.write_u16::<LittleEndian>(self.uid).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.size.try_into().unwrap()).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.access_time.try_into().unwrap()).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.creation_time.try_into().unwrap()).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.modification_time.try_into().unwrap()).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.deletion_time.try_into().unwrap()).unwrap();
        inode_data_cursor.write_u16::<LittleEndian>(self.gid).unwrap();
        inode_data_cursor.write_u16::<LittleEndian>(self.links_count).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.sector_count.try_into().unwrap()).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.flags).unwrap();
        inode_data_cursor.write_all(&self.os_dependent_1).unwrap();
        self.data_block_map
            .into_iter()
            .for_each(|block_index| inode_data_cursor.write_u32::<LittleEndian>(block_index.try_into().unwrap()).unwrap());
        inode_data_cursor.write_u32::<LittleEndian>(self.generation).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.file_acl).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.dir_arc).unwrap();
        inode_data_cursor.write_u32::<LittleEndian>(self.faddr).unwrap();
        inode_data_cursor.write_all(&self.os_dependent_2).unwrap();
        assert_eq!(inode_data_cursor.position(), Inode::PRACTICAL_SIZE);
    }
}

#[derive(Clone, Debug)]
struct DirEntry {
    inode_index: u64,
    file_type: Option<FileType>,
    name: String,
}

impl DirEntry {
    const PREFERRED_SIZE: u64 = 512;

    fn of_bytes(dir_entry_data: &[u8]) -> (Self, &[u8]) {
        let mut dir_entry_data_cursor = Cursor::new(dir_entry_data);
        let inode_index = dir_entry_data_cursor.read_u32::<LittleEndian>().unwrap() as u64;
        let record_len = dir_entry_data_cursor.read_u16::<LittleEndian>().unwrap() as u64;
        let name_len = dir_entry_data_cursor.read_u8().unwrap() as u64;
        let file_type = FileType::from_inode_file_type(dir_entry_data_cursor.read_u8().unwrap());
        let mut name = vec![0; name_len as usize];
        dir_entry_data_cursor.read_exact(&mut name).unwrap();
        let name = String::from_utf8_lossy(&name).into_owned();
        (DirEntry { inode_index, file_type, name }, &dir_entry_data[record_len as usize..])
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut dir_entry_data = vec![0; DirEntry::PREFERRED_SIZE as usize];
        let mut dir_entry_data_cursor = Cursor::new(&mut dir_entry_data[..]);
        dir_entry_data_cursor.write_u32::<LittleEndian>(self.inode_index.try_into().unwrap()).unwrap();
        dir_entry_data_cursor.write_u16::<LittleEndian>(DirEntry::PREFERRED_SIZE as u16).unwrap();
        dir_entry_data_cursor.write_u8(self.name.len().try_into().unwrap()).unwrap();
        dir_entry_data_cursor
            .write_u8(self.file_type.map(FileType::inode_file_type).unwrap_or(0))
            .unwrap();
        dir_entry_data_cursor.write_all(self.name.as_bytes()).unwrap();
        dir_entry_data
    }

    fn many_of_bytes(mut dir_entries_data: &[u8]) -> Vec<Self> {
        let mut dir_entries = Vec::new();
        while !dir_entries_data.is_empty() {
            let dir_entry;
            (dir_entry, dir_entries_data) = DirEntry::of_bytes(dir_entries_data);
            dir_entries.push(dir_entry);
        }
        dir_entries
    }

    fn many_to_bytes(dir_entries: &[Self]) -> Vec<u8> {
        dir_entries.iter().flat_map(DirEntry::to_bytes).collect()
    }
}

impl From<DirEntry> for super::fs::DirEntry {
    fn from(dir_entry: DirEntry) -> super::fs::DirEntry {
        super::fs::DirEntry {
            inode_index: dir_entry.inode_index,
            file_type: dir_entry.file_type,
            name: dir_entry.name,
        }
    }
}

impl From<super::fs::DirEntry> for DirEntry {
    fn from(dir_entry: super::fs::DirEntry) -> DirEntry {
        DirEntry {
            inode_index: dir_entry.inode_index,
            file_type: dir_entry.file_type,
            name: dir_entry.name,
        }
    }
}

#[derive(Debug)]
pub struct Session<'ss, SS: SectorStorage> {
    sector_storage: &'ss SS,
    superblock: Superblock,
    block_group_descriptors: Vec<BlockGroupDescriptor>, // Block Group Descriptor Table
}

impl<'ss, SS: SectorStorage> Session<'ss, SS> {
    pub fn new(sector_storage: &'ss SS) -> Self {
        let superblock = Superblock::of_bytes(&sector_storage.read_aligned(Superblock::INITIAL_START, Superblock::SIZE));
        let mut session = Session {
            sector_storage,
            superblock,
            block_group_descriptors: Vec::new(),
        };
        session.read_block_group_descriptors();
        session
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
            .map(|block_group_index| self.superblock.first_data_block_block_index + block_group_index * self.superblock.block_count_per_block_group)
            .map(|block_group_start| {
                block_group_start
                    ..min(
                        block_group_start + self.superblock.block_count_per_block_group,
                        self.sector_storage.len() / self.superblock.block_size(),
                    )
            })
            .take_while(|block_group_range| !block_group_range.is_empty())
    }

    fn block_group_count(&self) -> u64 {
        self.block_group_ranges().count() as u64
    }

    fn read_block_group_descriptors(&mut self) {
        assert!(self.superblock.block_size() % BlockGroupDescriptor::SIZE == 0);
        for block_group_index in 0..self.block_group_count() {
            let block_group_descriptor_offset = block_group_index * BlockGroupDescriptor::SIZE;
            let block_group_descriptor_block_index = self
                .block_group_ranges()
                .next()
                .unwrap()
                .nth(1 + (block_group_descriptor_offset / self.superblock.block_size()) as usize)
                .unwrap();
            let block_group_descriptor_block_offset = block_group_descriptor_offset % self.superblock.block_size();
            let block_group_descriptor_block_data = self.read_block(block_group_descriptor_block_index);
            let block_group_descriptor_data = &block_group_descriptor_block_data
                [block_group_descriptor_block_offset as usize..(block_group_descriptor_block_offset + BlockGroupDescriptor::SIZE) as usize];
            let block_group_descriptor = BlockGroupDescriptor::of_bytes(block_group_descriptor_data);
            self.block_group_descriptors.push(block_group_descriptor);
        }
    }

    fn block_group_contains_superblock_and_block_group_descriptor_table_copies(&self, block_group_index: u64) -> bool {
        !self.superblock.features_ro_compat.contains(FeaturesRoCompat::SPARSE_SUPER)
            || block_group_index == 0
            || [3, 5, 7]
                .into_iter()
                .any(|base: u64| base.pow(block_group_index.ilog(base)) == block_group_index)
    }

    #[allow(clippy::iter_nth_zero)]
    fn update_superblock_and_block_group_descriptor_table_copies(&self) {
        for (block_group_index, block_group_range) in self.block_group_ranges().enumerate() {
            let block_group_index = block_group_index as u64;
            if self.block_group_contains_superblock_and_block_group_descriptor_table_copies(block_group_index) {
                let superblock_block_index = block_group_range.clone().nth(0).unwrap();
                let mut superblock_data = self.read_block(superblock_block_index);
                Superblock {
                    block_group_index,
                    ..self.superblock
                }
                .update_bytes(&mut superblock_data);
                self.write_block(superblock_block_index, &superblock_data);
                for (block_group_index, &block_group_descriptor) in self.block_group_descriptors.iter().enumerate() {
                    let block_group_index = block_group_index as u64;
                    let block_group_descriptor_offset = block_group_index * BlockGroupDescriptor::SIZE;
                    let block_group_descriptor_block_index = block_group_range
                        .clone()
                        .nth(1 + (block_group_descriptor_offset / self.superblock.block_size()) as usize)
                        .unwrap();
                    let block_group_descriptor_block_offset = block_group_descriptor_offset % self.superblock.block_size();
                    let mut block_group_descriptor_block_data = self.read_block(block_group_descriptor_block_index);
                    let block_group_descriptor_data = &mut block_group_descriptor_block_data
                        [block_group_descriptor_block_offset as usize..(block_group_descriptor_block_offset + BlockGroupDescriptor::SIZE) as usize];
                    block_group_descriptor.update_bytes(block_group_descriptor_data);
                    self.write_block(block_group_descriptor_block_index, &block_group_descriptor_block_data);
                }
            }
        }
    }

    fn read_block_bitmap(&self, block_group_index: u64) -> Bitmap {
        Bitmap {
            data: self.read_block(self.block_group_descriptors[block_group_index as usize].block_bitmap_block_index),
        }
    }

    fn update_block_bitmap(&self, block_group_index: u64, block_bitmap: Bitmap) {
        self.write_block(
            self.block_group_descriptors[block_group_index as usize].block_bitmap_block_index,
            &block_bitmap.data,
        );
    }

    fn allocate_block(&mut self) -> u64 {
        let (block_group_index, (block_group_range, _)) = self
            .block_group_ranges()
            .zip(&self.block_group_descriptors)
            .enumerate()
            .find(|(_, (_, block_group_descriptor))| block_group_descriptor.free_blocks_count > 0)
            .ok_or("no free blocks")
            .unwrap();
        let block_group_index = block_group_index as u64;
        let mut block_bitmap = self.read_block_bitmap(block_group_index);
        for block_index in block_group_range.clone() {
            if !block_bitmap.get((block_index - block_group_range.start) as usize) {
                self.superblock.free_blocks_count -= 1;
                self.block_group_descriptors[block_group_index as usize].free_blocks_count -= 1;
                self.update_superblock_and_block_group_descriptor_table_copies();
                block_bitmap.set((block_index - block_group_range.start) as usize, true);
                self.update_block_bitmap(block_group_index, block_bitmap);
                return block_index;
            }
        }
        panic!("no free blocks inside block group");
    }

    fn allocate_zeroed_block(&mut self) -> u64 {
        let block_index = self.allocate_block();
        self.write_block(block_index, &vec![0; self.superblock.block_size() as usize]);
        block_index
    }

    fn free_block(&mut self, block_index: u64) {
        let (block_group_index, (block_group_range, _)) = self
            .block_group_ranges()
            .zip(&self.block_group_descriptors)
            .enumerate()
            .find(|(_, (block_group_range, _))| block_group_range.contains(&block_index))
            .ok_or("block group not found")
            .unwrap();
        let block_group_index = block_group_index as u64;
        let mut block_bitmap = self.read_block_bitmap(block_group_index);
        assert!(block_bitmap.get((block_index - block_group_range.start) as usize));
        self.superblock.free_blocks_count += 1;
        self.block_group_descriptors[block_group_index as usize].free_blocks_count += 1;
        self.update_superblock_and_block_group_descriptor_table_copies();
        block_bitmap.set((block_index - block_group_range.start) as usize, false);
        self.update_block_bitmap(block_group_index, block_bitmap);
    }

    fn read_inode_bitmap(&self, block_group_index: u64) -> Bitmap {
        Bitmap {
            data: self.read_block(self.block_group_descriptors[block_group_index as usize].inode_bitmap_block_index),
        }
    }

    fn update_inode_bitmap(&self, block_group_index: u64, inode_bitmap: Bitmap) {
        self.write_block(
            self.block_group_descriptors[block_group_index as usize].inode_bitmap_block_index,
            &inode_bitmap.data,
        );
    }

    fn allocate_inode(&mut self) -> u64 {
        let (block_group_index, _) = self
            .block_group_descriptors
            .iter()
            .enumerate()
            .find(|(_, block_group_descriptor)| block_group_descriptor.free_inodes_count > 0)
            .ok_or("no gree inodes")
            .unwrap();
        let block_group_index = block_group_index as u64;
        let mut inode_bitmap = self.read_inode_bitmap(block_group_index);
        let inode_range = 1 + block_group_index * self.superblock.inode_count_per_block_group
            ..1 + block_group_index * self.superblock.inode_count_per_block_group + self.superblock.inode_count_per_block_group;
        for inode_index in inode_range.clone() {
            if !inode_bitmap.get((inode_index - inode_range.start) as usize) {
                assert!(inode_index >= self.superblock.first_usable_inode_index);
                self.superblock.free_inodes_count -= 1;
                self.block_group_descriptors[block_group_index as usize].free_inodes_count -= 1;
                self.update_superblock_and_block_group_descriptor_table_copies();
                inode_bitmap.set((inode_index - inode_range.start) as usize, true);
                self.update_inode_bitmap(block_group_index, inode_bitmap);
                return inode_index;
            }
        }
        panic!("no free inodes inside block group");
    }

    fn free_inode(&mut self, inode_index: u64) {
        assert_ne!(inode_index, 0);
        let block_group_index = (inode_index - 1) / self.superblock.inode_count_per_block_group;
        let mut inode_bitmap = self.read_inode_bitmap(block_group_index);
        assert!(inode_bitmap.get(((inode_index - 1) % self.superblock.inode_count_per_block_group) as usize));
        self.superblock.free_inodes_count += 1;
        self.block_group_descriptors[block_group_index as usize].free_inodes_count += 1;
        self.update_superblock_and_block_group_descriptor_table_copies();
        inode_bitmap.set(((inode_index - 1) % self.superblock.inode_count_per_block_group) as usize, false);
        self.update_inode_bitmap(block_group_index, inode_bitmap);
    }

    fn read_inode(&self, inode_index: u64) -> Inode {
        assert!(self.superblock.block_size() % self.superblock.inode_size == 0);
        assert_ne!(inode_index, 0);
        let inode_block_group_index = (inode_index - 1) / self.superblock.inode_count_per_block_group;
        let inode_block_group_inode_index = (inode_index - 1) % self.superblock.inode_count_per_block_group;
        let inode_offset = inode_block_group_inode_index * self.superblock.inode_size;
        let inode_block_index =
            self.block_group_descriptors[inode_block_group_index as usize].inode_table_first_block_index + inode_offset / self.superblock.block_size();
        let inode_block_offset = inode_offset % self.superblock.block_size();
        let inode_block_data = self.read_block(inode_block_index);
        let inode_data = &inode_block_data[inode_block_offset as usize..(inode_block_offset + self.superblock.inode_size) as usize];
        Inode::of_bytes(inode_data)
    }

    fn update_inode(&self, inode_index: u64, inode: &Inode) {
        assert_ne!(inode_index, 0);
        let inode_block_group_index = (inode_index - 1) / self.superblock.inode_count_per_block_group;
        let inode_block_group_inode_index = (inode_index - 1) % self.superblock.inode_count_per_block_group;
        let inode_offset = inode_block_group_inode_index * self.superblock.inode_size;
        let inode_block_index =
            self.block_group_descriptors[inode_block_group_index as usize].inode_table_first_block_index + inode_offset / self.superblock.block_size();
        let inode_block_offset = inode_offset % self.superblock.block_size();
        let mut inode_block_data = self.read_block(inode_block_index);
        let inode_data = &mut inode_block_data[inode_block_offset as usize..(inode_block_offset + self.superblock.inode_size) as usize];
        inode.update_bytes(inode_data);
        self.write_block(inode_block_index, &inode_block_data);
    }

    fn block_indices_per_block(&self) -> u64 {
        self.superblock.block_size() / 4
    }

    fn read_block_indices(&self, block_index: u64) -> Vec<u64> {
        let block_data = self.read_block(block_index);
        let mut block_data_cursor = Cursor::new(&block_data);
        let mut block_indices = Vec::new();
        while !block_data_cursor.is_empty() {
            block_indices.push(block_data_cursor.read_u32::<LittleEndian>().unwrap() as u64);
        }
        assert_eq!(block_indices.len() as u64, self.block_indices_per_block());
        block_indices
    }

    fn write_block_indices(&self, block_index: u64, block_indices: &[u64]) {
        assert_eq!(block_indices.len() as u64, self.block_indices_per_block());
        let mut block_data = vec![0; self.superblock.block_size() as usize];
        let mut block_data_cursor = Cursor::new(&mut block_data[..]);
        for &block_index in block_indices {
            block_data_cursor.write_u32::<LittleEndian>(block_index.try_into().unwrap()).unwrap();
        }
        self.write_block(block_index, &block_data);
    }

    fn inode_block_path(&self, mut inode_block_index: u64) -> Vec<u64> {
        if inode_block_index < 12 {
            vec![inode_block_index]
        } else {
            inode_block_index -= 12;
            if inode_block_index < self.block_indices_per_block() {
                vec![12, inode_block_index]
            } else {
                inode_block_index -= self.block_indices_per_block();
                if inode_block_index < self.block_indices_per_block() * self.block_indices_per_block() {
                    vec![
                        13,
                        inode_block_index / self.block_indices_per_block(),
                        inode_block_index % self.block_indices_per_block(),
                    ]
                } else {
                    inode_block_index -= self.block_indices_per_block();
                    if inode_block_index < self.block_indices_per_block() * self.block_indices_per_block() {
                        vec![
                            14,
                            inode_block_index / self.block_indices_per_block() / self.block_indices_per_block(),
                            inode_block_index / self.block_indices_per_block() % self.block_indices_per_block(),
                            inode_block_index % self.block_indices_per_block(),
                        ]
                    } else {
                        panic!("block index too big");
                    }
                }
            }
        }
    }

    fn inode_read_data_block(&self, inode: &Inode, inode_block_index: u64) -> Vec<u8> {
        let mut data_block_index = 0;
        let mut data_block_indices = inode.data_block_map.to_vec();
        for data_block_indices_index in self.inode_block_path(inode_block_index) {
            data_block_index = data_block_indices[data_block_indices_index as usize];
            if data_block_index == 0 {
                return vec![0; self.superblock.block_size() as usize];
            }
            data_block_indices = self.read_block_indices(data_block_index);
        }
        self.read_block(data_block_index)
    }

    fn inode_read_data_range(&self, inode: &Inode, range: Range<u64>) -> Vec<u8> {
        (range.start / self.superblock.block_size()..range.end.div_ceil(self.superblock.block_size()))
            .flat_map(|inode_block_index| self.inode_read_data_block(inode, inode_block_index))
            .skip((range.start / self.superblock.block_size()) as usize)
            .take(range.count())
            .collect()
    }

    fn inode_read_data(&self, inode: &Inode) -> Vec<u8> {
        self.inode_read_data_range(inode, 0..inode.size)
    }

    fn inode_write_data_block(&mut self, inode: &mut Inode, inode_block_index: u64, data_block_data: &[u8]) {
        let inode_block_path = self.inode_block_path(inode_block_index);
        let mut data_block_index = 0;
        let mut data_block_index_history = Vec::new();
        let mut data_block_indices = inode.data_block_map.to_vec();
        let mut data_block_indices_history = Vec::new();
        for (path_element_index, &data_block_indices_index) in inode_block_path.iter().enumerate() {
            if data_block_indices[data_block_indices_index as usize] == 0 {
                data_block_indices[data_block_indices_index as usize] = self.allocate_zeroed_block();
                if path_element_index == 0 {
                    inode.data_block_map = data_block_indices.clone().try_into().unwrap();
                } else {
                    self.write_block_indices(data_block_index, &data_block_indices);
                }
            }
            data_block_index = data_block_indices[data_block_indices_index as usize];
            data_block_index_history.push(data_block_index);
            data_block_indices_history.push(data_block_indices);
            data_block_indices = self.read_block_indices(data_block_index);
        }
        self.write_block(data_block_index, data_block_data);
        for path_element_index in (0..inode_block_path.len()).rev() {
            if self.read_block(data_block_index_history[path_element_index]) != vec![0; self.superblock.block_size() as usize] {
                break;
            }
            let data_block_indices_index = inode_block_path[path_element_index];
            let data_block_indices = &mut data_block_indices_history[path_element_index];
            self.free_block(data_block_indices[data_block_indices_index as usize]);
            data_block_indices[data_block_indices_index as usize] = 0;
            if path_element_index == 0 {
                inode.data_block_map = data_block_indices.clone().try_into().unwrap();
            } else {
                self.write_block_indices(data_block_index_history[path_element_index], data_block_indices);
            }
        }
    }

    fn inode_write_data_range(&mut self, inode: &mut Inode, range: Range<u64>, data: &[u8]) {
        assert_eq!(range.clone().count(), data.len());
        if range.is_empty() {
            return;
        }
        let first_inode_block_index = range.start / self.superblock.block_size();
        let last_inode_block_index = range.end.div_ceil(self.superblock.block_size()) - 1;
        if first_inode_block_index == last_inode_block_index {
            let mut block_data = self.inode_read_data_block(inode, first_inode_block_index);
            block_data[(range.start % self.superblock.block_size()) as usize..((range.end - 1) % self.superblock.block_size() + 1) as usize]
                .copy_from_slice(data);
            self.inode_write_data_block(inode, first_inode_block_index, &block_data);
        } else {
            let mut first_block_data = self.inode_read_data_block(inode, first_inode_block_index);
            first_block_data[(first_inode_block_index * self.superblock.block_size() + self.superblock.block_size() - range.start) as usize..]
                .copy_from_slice(&data[..(range.start - first_inode_block_index * self.superblock.block_size()) as usize]);
            self.inode_write_data_block(inode, first_inode_block_index, &first_block_data);
            for inode_block_index in first_inode_block_index + 1..=last_inode_block_index - 1 {
                self.inode_write_data_block(
                    inode,
                    inode_block_index,
                    &data[(inode_block_index * self.superblock.block_size() - range.start) as usize
                        ..(inode_block_index * self.superblock.block_size() + self.superblock.block_size() - range.start) as usize],
                );
            }
            let mut last_block_data = self.inode_read_data_block(inode, last_inode_block_index);
            last_block_data[..(range.end - last_inode_block_index * self.superblock.block_size()) as usize]
                .copy_from_slice(&data[(last_inode_block_index * self.superblock.block_size() - range.start) as usize..]);
            self.inode_write_data_block(inode, last_inode_block_index, &last_block_data);
        }
    }

    fn inode_write_data(&mut self, inode: &mut Inode, data: &[u8]) {
        self.inode_write_data_range(inode, 0..inode.size, data);
    }

    fn inode_resize(&mut self, inode: &mut Inode, new_size: u64) {
        self.inode_write_data_range(inode, inode.size..new_size, &vec![0; (inode.size..new_size).count()]);
        self.inode_write_data_range(inode, new_size..inode.size, &vec![0; (new_size..inode.size).count()]);
        inode.size = new_size;
        inode.sector_count = new_size * self.superblock.block_size() / 512;
    }
}

impl<'ss, SS: SectorStorage> super::fs::Session for Session<'ss, SS> {
    fn root(&self) -> u64 {
        2
    }

    fn file_stat(&self, inode_index: u64) -> FileStat {
        let inode = self.read_inode(inode_index);
        FileStat {
            mode: inode.mode,
            uid: inode.uid,
            gid: inode.gid,
            links_count: inode.links_count,
            size: inode.size,
            access_time: inode.access_time,
            creation_time: inode.creation_time,
            modification_time: inode.modification_time,
        }
    }

    fn create(&mut self, file_type: FileType, permissions: u32) -> u64 {
        let inode_index = self.allocate_inode();
        let inode = Inode {
            mode: Mode::from_file_type_and_permissions(permissions, file_type),
            uid: 0,
            size: 0,
            access_time: 0,
            creation_time: 0,
            modification_time: 0,
            deletion_time: 0,
            gid: 0,
            links_count: 0,
            sector_count: 0,
            flags: 0,
            os_dependent_1: [0; 4],
            data_block_map: [0; 15],
            generation: 0,
            file_acl: 0,
            dir_arc: 0,
            faddr: 0,
            os_dependent_2: [0; 12],
        };
        self.update_inode(inode_index, &inode);
        inode_index
    }

    fn remove(&mut self, inode_index: u64) {
        self.free_inode(inode_index);
    }

    fn set_links_count(&mut self, inode_index: u64, links_count: u16) {
        let mut inode = self.read_inode(inode_index);
        inode.links_count = links_count;
        self.update_inode(inode_index, &inode);
    }

    fn read_regular_file_range(&self, inode_index: u64, range: Range<u64>) -> Vec<u8> {
        let inode = self.read_inode(inode_index);
        self.inode_read_data_range(&inode, range)
    }

    fn write_regular_file_range(&mut self, inode_index: u64, range: Range<u64>, data: &[u8]) {
        let mut inode = self.read_inode(inode_index);
        self.inode_write_data_range(&mut inode, range, data);
        self.update_inode(inode_index, &inode);
    }

    fn resize_regular_file(&mut self, inode_index: u64, size: u64) {
        let mut inode = self.read_inode(inode_index);
        self.inode_resize(&mut inode, size);
        self.update_inode(inode_index, &inode);
    }

    fn read_dir(&self, inode_index: u64) -> Vec<super::fs::DirEntry> {
        let inode = self.read_inode(inode_index);
        let dir_entries_data = self.inode_read_data(&inode);
        DirEntry::many_of_bytes(&dir_entries_data)
            .into_iter()
            .map(|dir_entry| dir_entry.into())
            .collect()
    }

    fn write_dir(&mut self, inode_index: u64, dir_entries: &[super::fs::DirEntry]) {
        let mut inode = self.read_inode(inode_index);
        let dir_entries_data = DirEntry::many_to_bytes(&dir_entries.iter().map(|dir_entry| dir_entry.clone().into()).collect::<Vec<_>>());
        self.inode_write_data(&mut inode, &dir_entries_data);
        self.update_inode(inode_index, &inode);
    }
}
