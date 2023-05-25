use alloc::{string::String, vec::Vec};
use bitflags::bitflags;
use core::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileType {
    RegularFile,
    Dir,
    CharacterDevice,
    BlockDevice,
    Fifo,
    Socket,
    SymbolicLink,
}

impl FileType {
    pub fn inode_file_type(self) -> u8 {
        match self {
            FileType::RegularFile => 1,
            FileType::Dir => 2,
            FileType::CharacterDevice => 3,
            FileType::BlockDevice => 4,
            FileType::Fifo => 5,
            FileType::Socket => 6,
            FileType::SymbolicLink => 7,
        }
    }

    pub fn from_inode_file_type(inode_file_type: u8) -> Option<Self> {
        match inode_file_type {
            1 => Some(FileType::RegularFile),
            2 => Some(FileType::Dir),
            3 => Some(FileType::CharacterDevice),
            4 => Some(FileType::BlockDevice),
            5 => Some(FileType::Fifo),
            6 => Some(FileType::Socket),
            7 => Some(FileType::SymbolicLink),
            _ => None,
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Mode: u32 {
        const OTHERS_EXECUTE = 0x0001; // IXOTH
        const OTHERS_WRITE = 0x0002; // IWOTH
        const OTHERS_READ = 0x0004; // IROTH
        const GROUP_EXECUTE = 0x0008; // IXGRP
        const GROUP_WRITE = 0x0010; // IWGRP
        const GROUP_READ = 0x0020; // IRGRP
        const USER_EXECUTE = 0x0040; // IXUSR
        const USER_WRITE = 0x0080; // IWUSR
        const USER_READ = 0x0100; // IRUSR
        const STICKY_BIT = 0x0200; // ISVTX
        const SETGID = 0x0400; // ISGID
        const SETUID = 0x0800; // ISUID
        const FIFO = 0x1000;
        const CHARACTER_DEVICE = 0x2000; // IFCHR
        const DIR = 0x4000;
        const BLOCK_DEVICE = 0x6000; // IFBLK
        const REGULAR_FILE = 0x8000; // IFREG
        const SYMBOLIC_LINK = 0xA000; // IFLNK
        const SOCKET = 0xC000; // IFSOCK
    }
}

impl Mode {
    pub fn from_file_type_and_permissions(permissions: u32, file_type: FileType) -> Self {
        let file_type_mode = match file_type {
            FileType::Fifo => Mode::FIFO,
            FileType::CharacterDevice => Mode::CHARACTER_DEVICE,
            FileType::Dir => Mode::DIR,
            FileType::BlockDevice => Mode::BLOCK_DEVICE,
            FileType::RegularFile => Mode::REGULAR_FILE,
            FileType::SymbolicLink => Mode::SYMBOLIC_LINK,
            FileType::Socket => Mode::SOCKET,
        };
        Mode::from_bits_retain(permissions).union(file_type_mode)
    }

    pub fn file_type(self) -> FileType {
        match self.difference(Mode::from_bits_retain(0x0fff)) {
            Mode::FIFO => FileType::Fifo,
            Mode::CHARACTER_DEVICE => FileType::CharacterDevice,
            Mode::DIR => FileType::Dir,
            Mode::BLOCK_DEVICE => FileType::BlockDevice,
            Mode::REGULAR_FILE => FileType::RegularFile,
            Mode::SYMBOLIC_LINK => FileType::SymbolicLink,
            Mode::SOCKET => FileType::Socket,
            _ => panic!("unknown mode"),
        }
    }

    pub fn permissions(self) -> u32 {
        self.bits() & 0x0fff
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FileStat {
    pub mode: Mode,
    pub uid: u16,
    pub gid: u16,
    pub links_count: u16,
    pub size: u64,
    pub access_time: u64,
    pub creation_time: u64,
    pub modification_time: u64,
}

#[derive(Clone, Debug)]
pub struct DirEntry {
    pub inode_index: u64,
    pub file_type: Option<FileType>,
    pub name: String,
}

pub trait Session {
    fn root(&self) -> u64;

    fn file_stat(&self, inode_index: u64) -> FileStat;

    fn create(&mut self, file_type: FileType, permissions: u32) -> u64;

    fn remove(&mut self, inode_index: u64);

    fn set_links_count(&mut self, inode_index: u64, links_count: u16);

    fn read_regular_file_range(&self, inode_index: u64, range: Range<u64>) -> Vec<u8>;

    fn write_regular_file_range(&mut self, inode_index: u64, range: Range<u64>, data: &[u8]);

    fn resize_regular_file(&mut self, inode_index: u64, size: u64);

    fn read_dir(&self, inode_index: u64) -> Vec<DirEntry>;

    fn write_dir(&mut self, inode_index: u64, dir_entries: &[DirEntry]);
}
