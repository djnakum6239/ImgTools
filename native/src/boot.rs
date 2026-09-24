use core::fmt;

pub const BOOT_MAGIC: &[u8; 8] = b"ANDROID!";
pub const VENDOR_BOOT_MAGIC: &[u8; 8] = b"VNDRBOOT";
pub const HEADER_V0_SIZE: usize = 1632;
pub const HEADER_V1_SIZE: usize = 1648;
pub const HEADER_V2_SIZE: usize = 1660;
pub const HEADER_V3_SIZE: usize = 1580;
pub const HEADER_V4_SIZE: usize = 1584;
pub const VENDOR_HEADER_V3_SIZE: usize = 2112;
pub const VENDOR_HEADER_V4_SIZE: usize = 2128;
pub const MAX_HEADER_VERSION: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootParseError {
    TooSmall { actual: usize, minimum: usize },
    BadMagic([u8; 8]),
    UnsupportedVersion(u32),
    Truncated { required: usize, actual: usize },
}

impl fmt::Display for BootParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall { actual, minimum } => write!(f, "file is {actual} bytes, minimum is {minimum}"),
            Self::BadMagic(magic) => write!(f, "unsupported boot magic: {:?}", magic),
            Self::UnsupportedVersion(version) => write!(f, "boot header version {version} is newer than supported version {MAX_HEADER_VERSION}"),
            Self::Truncated { required, actual } => write!(f, "header requires {required} bytes, file has {actual}"),
        }
    }
}

impl std::error::Error for BootParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootHeader {
    pub header_version: u32,
    pub header_size: u32,
    pub page_size: u32,
    pub kernel_size: u32,
    pub kernel_addr: u32,
    pub ramdisk_size: u32,
    pub ramdisk_addr: u32,
    pub second_size: u32,
    pub second_addr: u32,
    pub tags_addr: u32,
    pub os_version_raw: u32,
    pub recovery_dtbo_size: u32,
    pub recovery_dtbo_offset: u64,
    pub dtb_size: u32,
    pub dtb_addr: u64,
    pub signature_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorBootHeader {
    pub header_version: u32,
    pub header_size: u32,
    pub page_size: u32,
    pub kernel_addr: u32,
    pub ramdisk_addr: u32,
    pub vendor_ramdisk_size: u32,
    pub tags_addr: u32,
    pub dtb_size: u32,
    pub dtb_addr: u64,
    pub ramdisk_table_size: u32,
    pub ramdisk_table_entry_num: u32,
    pub ramdisk_table_entry_size: u32,
    pub bootconfig_size: u32,
}

fn u32le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn u64le(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

pub fn boot_header_size(version: u32) -> usize {
    match version {
        0 => HEADER_V0_SIZE,
        1 => HEADER_V1_SIZE,
        2 => HEADER_V2_SIZE,
        3 => HEADER_V3_SIZE,
        _ => HEADER_V4_SIZE,
    }
}

pub fn parse_boot_header(bytes: &[u8]) -> Result<BootHeader, BootParseError> {
    if bytes.len() < HEADER_V0_SIZE {
        return Err(BootParseError::TooSmall { actual: bytes.len(), minimum: HEADER_V0_SIZE });
    }
    let mut magic = [0u8; 8];
    magic.copy_from_slice(&bytes[..8]);
    if &magic != BOOT_MAGIC {
        return Err(BootParseError::BadMagic(magic));
    }

    let version = u32le(bytes, 40);
    if version > MAX_HEADER_VERSION {
        return Err(BootParseError::UnsupportedVersion(version));
    }
    let required = boot_header_size(version);
    if bytes.len() < required {
        return Err(BootParseError::Truncated { required, actual: bytes.len() });
    }

    if version >= 3 {
        return Ok(BootHeader {
            header_version: version,
            header_size: u32le(bytes, 20).max(required as u32),
            page_size: 4096,
            kernel_size: u32le(bytes, 8),
            kernel_addr: 0,
            ramdisk_size: u32le(bytes, 12),
            ramdisk_addr: 0,
            second_size: 0,
            second_addr: 0,
            tags_addr: 0,
            os_version_raw: u32le(bytes, 16),
            recovery_dtbo_size: 0,
            recovery_dtbo_offset: 0,
            dtb_size: 0,
            dtb_addr: 0,
            signature_size: if version >= 4 { u32le(bytes, 1580) } else { 0 },
        });
    }

    Ok(BootHeader {
        header_version: version,
        header_size: required as u32,
        page_size: u32le(bytes, 36),
        kernel_size: u32le(bytes, 8),
        kernel_addr: u32le(bytes, 12),
        ramdisk_size: u32le(bytes, 16),
        ramdisk_addr: u32le(bytes, 20),
        second_size: u32le(bytes, 24),
        second_addr: u32le(bytes, 28),
        tags_addr: u32le(bytes, 32),
        os_version_raw: u32le(bytes, 44),
        recovery_dtbo_size: if version >= 1 { u32le(bytes, 1632) } else { 0 },
        recovery_dtbo_offset: if version >= 1 { u64le(bytes, 1636) } else { 0 },
        dtb_size: if version >= 2 { u32le(bytes, 1648) } else { 0 },
        dtb_addr: if version >= 2 { u64le(bytes, 1652) } else { 0 },
        signature_size: 0,
    })
}

pub fn parse_vendor_boot_header(bytes: &[u8]) -> Result<VendorBootHeader, BootParseError> {
    if bytes.len() < VENDOR_HEADER_V3_SIZE {
        return Err(BootParseError::TooSmall { actual: bytes.len(), minimum: VENDOR_HEADER_V3_SIZE });
    }
    let mut magic = [0u8; 8];
    magic.copy_from_slice(&bytes[..8]);
    if &magic != VENDOR_BOOT_MAGIC {
        return Err(BootParseError::BadMagic(magic));
    }

    let version = u32le(bytes, 8);
    if version < 3 || version > MAX_HEADER_VERSION {
        return Err(BootParseError::UnsupportedVersion(version));
    }
    let required = if version >= 4 { VENDOR_HEADER_V4_SIZE } else { VENDOR_HEADER_V3_SIZE };
    if bytes.len() < required {
        return Err(BootParseError::Truncated { required, actual: bytes.len() });
    }

    Ok(VendorBootHeader {
        header_version: version,
        header_size: { let encoded = u32le(bytes, 2096); if encoded == 0 { required as u32 } else { encoded } },
        page_size: u32le(bytes, 12),
        kernel_addr: u32le(bytes, 16),
        ramdisk_addr: u32le(bytes, 20),
        vendor_ramdisk_size: u32le(bytes, 24),
        tags_addr: u32le(bytes, 2076),
        dtb_size: u32le(bytes, 2100),
        dtb_addr: u64le(bytes, 2104),
        ramdisk_table_size: if version >= 4 { u32le(bytes, 2112) } else { 0 },
        ramdisk_table_entry_num: if version >= 4 { u32le(bytes, 2116) } else { 0 },
        ramdisk_table_entry_size: if version >= 4 { u32le(bytes, 2120) } else { 0 },
        bootconfig_size: if version >= 4 { u32le(bytes, 2124) } else { 0 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_v4() -> Vec<u8> {
        let mut b = vec![0u8; HEADER_V4_SIZE];
        b[..8].copy_from_slice(BOOT_MAGIC);
        b[40..44].copy_from_slice(&4u32.to_le_bytes());
        b[20..24].copy_from_slice(&(HEADER_V4_SIZE as u32).to_le_bytes());
        b
    }

    #[test]
    fn parses_modern_boot_header() {
        let bytes = base_v4();
        let header = parse_boot_header(&bytes).unwrap();
        assert_eq!(header.header_version, 4);
        assert_eq!(header.header_size, HEADER_V4_SIZE as u32);
        assert_eq!(header.page_size, 4096);
    }

    #[test]
    fn rejects_bad_magic_and_new_versions() {
        let mut bytes = base_v4();
        bytes[..8].copy_from_slice(b"NOPE!!!!");
        assert!(matches!(parse_boot_header(&bytes), Err(BootParseError::BadMagic(_))));

        bytes[..8].copy_from_slice(BOOT_MAGIC);
        bytes[40..44].copy_from_slice(&5u32.to_le_bytes());
        assert_eq!(parse_boot_header(&bytes), Err(BootParseError::UnsupportedVersion(5)));
    }

    #[test]
    fn identifies_init_boot_shape() {
        let mut bytes = base_v4();
        bytes[8..12].copy_from_slice(&0u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&4096u32.to_le_bytes());
        let header = parse_boot_header(&bytes).unwrap();
        assert_eq!(header.kernel_size, 0);
        assert_eq!(header.ramdisk_size, 4096);
    }
}
