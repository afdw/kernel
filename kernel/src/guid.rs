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

pub const ZERO: Guid = Guid(uefi::data_types::Guid::ZERO);
pub const TYPE_ID_LINUX: Guid = Guid(uefi::data_types::Guid::parse_or_panic("0FC63DAF-8483-4772-8E79-3D69D8477DE4"));
