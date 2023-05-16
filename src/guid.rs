use core::fmt::{Debug, Display, Formatter};

#[derive(PartialEq, Eq)]
pub struct Guid(uefi::data_types::Guid);

impl Debug for Guid {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Guid {
    pub fn from_bytes(bytes: [u8; 16]) -> Guid {
        Guid(uefi::data_types::Guid::from_bytes(bytes))
    }
}

pub const ZERO: Guid = Guid(uefi::data_types::Guid::from_values(0, 0, 0, 0, 0));
pub const TYPE_ID_LINUX: Guid = Guid(uefi::data_types::Guid::from_values(0x0FC63DAF, 0x8483, 0x4772, 0x8E79, 0x3D69D8477DE4));
