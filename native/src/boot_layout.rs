use crate::binary::{align_up, BinaryError};
use crate::boot::{BootHeader, VENDOR_HEADER_V3_SIZE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootSectionKind {
    Kernel,
    Ramdisk,
    Second,
    RecoveryDtbo,
    Dtb,
    VendorRamdisk,
    VendorDtb,
    VendorRamdiskTable,
    Bootconfig,
    Signature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootSection {
    pub kind: BootSectionKind,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    InvalidPageSize(u32),
    Binary(BinaryError),
    SectionOutOfBounds { kind: BootSectionKind, offset: u64, size: u64, source_size: u64 },
}

impl core::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidPageSize(v) => write!(f, "invalid boot page size {v}"),
            Self::Binary(e) => write!(f, "{e}"),
            Self::SectionOutOfBounds { kind, offset, size, source_size } =>
                write!(f, "{kind:?} at {offset}+{size} exceeds source size {source_size}"),
        }
    }
}

impl std::error::Error for LayoutError {}

impl From<BinaryError> for LayoutError {
    fn from(value: BinaryError) -> Self { Self::Binary(value) }
}

fn validate_section(section: BootSection, source_size: u64) -> Result<BootSection, LayoutError> {
    let end = section.offset.checked_add(section.size).ok_or(BinaryError::Overflow)?;
    if end > source_size {
        return Err(LayoutError::SectionOutOfBounds {
            kind: section.kind,
            offset: section.offset,
            size: section.size,
            source_size,
        });
    }
    Ok(section)
}

fn page_size(value: u32) -> Result<u64, LayoutError> {
    let p = u64::from(value);
    if p < 512 || !p.is_power_of_two() {
        return Err(LayoutError::InvalidPageSize(value));
    }
    Ok(p)
}

pub fn boot_sections(header: &BootHeader, source_size: u64) -> Result<Vec<BootSection>, LayoutError> {
    let page = if header.header_version >= 3 { 4096 } else { page_size(header.page_size)? };
    let mut cursor = align_up(u64::from(header.header_size), page)?;
    let mut sections = Vec::new();

    if header.kernel_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::Kernel,
            offset: cursor,
            size: u64::from(header.kernel_size),
        }, source_size)?;
        cursor = align_up(cursor + s.size, page)?;
        sections.push(s);
    }
    if header.ramdisk_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::Ramdisk,
            offset: cursor,
            size: u64::from(header.ramdisk_size),
        }, source_size)?;
        cursor = align_up(cursor + s.size, page)?;
        sections.push(s);
    }
    if header.second_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::Second,
            offset: cursor,
            size: u64::from(header.second_size),
        }, source_size)?;
        cursor = align_up(cursor + s.size, page)?;
        sections.push(s);
    }
    if header.header_version >= 1 && header.recovery_dtbo_size > 0 {
        let recorded = header.recovery_dtbo_offset;
        let end = recorded.checked_add(u64::from(header.recovery_dtbo_size));
        let offset = match end {
            Some(end) if recorded >= cursor && end <= source_size => recorded,
            _ => cursor,
        };
        let s = validate_section(BootSection {
            kind: BootSectionKind::RecoveryDtbo,
            offset,
            size: u64::from(header.recovery_dtbo_size),
        }, source_size)?;
        cursor = align_up(offset + s.size, page)?;
        sections.push(s);
    }
    if header.header_version >= 2 && header.dtb_size > 0 {
        let sequential = cursor;
        let from_end = source_size.checked_sub(u64::from(header.dtb_size)).unwrap_or(0);
        let offset = if sequential.checked_add(u64::from(header.dtb_size)).is_some_and(|end| end <= source_size) {
            sequential
        } else {
            from_end
        };
        let s = validate_section(BootSection {
            kind: BootSectionKind::Dtb,
            offset,
            size: u64::from(header.dtb_size),
        }, source_size)?;
        cursor = align_up(offset + s.size, page)?;
        sections.push(s);
    }

    if header.header_version >= 3 {
        let signature_size = u64::from(header.signature_size);
        if signature_size > source_size {
            return Err(LayoutError::SectionOutOfBounds {
                kind: BootSectionKind::Signature,
                offset: 0,
                size: signature_size,
                source_size,
            });
        }
        let signature_offset = source_size - signature_size;
        if cursor < signature_offset {
            sections.push(validate_section(BootSection {
                kind: BootSectionKind::Bootconfig,
                offset: cursor,
                size: signature_offset - cursor,
            }, source_size)?);
        }
        if signature_size > 0 {
            sections.push(validate_section(BootSection {
                kind: BootSectionKind::Signature,
                offset: signature_offset,
                size: signature_size,
            }, source_size)?);
        }
    }

    Ok(sections)
}

pub fn vendor_sections(
    header_version: u32,
    header_size: u32,
    page_size_value: u32,
    vendor_ramdisk_size: u32,
    dtb_size: u32,
    table_size: u32,
    bootconfig_size: u32,
    source_size: u64,
) -> Result<Vec<BootSection>, LayoutError> {
    let page = page_size(page_size_value)?;
    let minimum_header = if header_version >= 4 { VENDOR_HEADER_V3_SIZE as u64 + 16 } else { VENDOR_HEADER_V3_SIZE as u64 };
    if u64::from(header_size) < minimum_header {
        return Err(LayoutError::SectionOutOfBounds {
            kind: BootSectionKind::VendorRamdisk,
            offset: u64::from(header_size),
            size: 0,
            source_size,
        });
    }
    let mut cursor = align_up(u64::from(header_size), page)?;
    let mut sections = Vec::new();

    if vendor_ramdisk_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::VendorRamdisk,
            offset: cursor,
            size: u64::from(vendor_ramdisk_size),
        }, source_size)?;
        cursor = align_up(cursor + s.size, page)?;
        sections.push(s);
    }
    if dtb_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::VendorDtb,
            offset: cursor,
            size: u64::from(dtb_size),
        }, source_size)?;
        cursor = align_up(cursor + s.size, page)?;
        sections.push(s);
    }
    if table_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::VendorRamdiskTable,
            offset: cursor,
            size: u64::from(table_size),
        }, source_size)?;
        cursor = align_up(cursor + s.size, 4)?;
        sections.push(s);
    }
    if bootconfig_size > 0 {
        let s = validate_section(BootSection {
            kind: BootSectionKind::Bootconfig,
            offset: cursor,
            size: u64::from(bootconfig_size),
        }, source_size)?;
        sections.push(s);
    }

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot::{BOOT_MAGIC, HEADER_V4_SIZE};

    fn v4(kernel: u32, ramdisk: u32) -> Vec<u8> {
        let mut b = vec![0u8; 4096 * 3];
        b[..8].copy_from_slice(BOOT_MAGIC);
        b[40..44].copy_from_slice(&4u32.to_le_bytes());
        b[20..24].copy_from_slice(&(HEADER_V4_SIZE as u32).to_le_bytes());
        b[8..12].copy_from_slice(&kernel.to_le_bytes());
        b[12..16].copy_from_slice(&ramdisk.to_le_bytes());
        b
    }

    #[test]
    fn lays_out_modern_kernel_and_ramdisk() {
        let b = v4(100, 200);
        let h = crate::boot::parse_boot_header(&b).unwrap();
        let sections = boot_sections(&h, b.len() as u64).unwrap();
        assert_eq!(sections[0].kind, BootSectionKind::Kernel);
        assert_eq!(sections[0].offset, 4096);
        assert_eq!(sections[1].offset, 8192);
    }

    #[test]
    fn detects_truncated_section() {
        let b = v4(5000, 0);
        let h = crate::boot::parse_boot_header(&b).unwrap();
        let source_size = 4096u64 + 4999;
        assert!(matches!(
            boot_sections(&h, source_size),
            Err(LayoutError::SectionOutOfBounds { kind: BootSectionKind::Kernel, .. })
        ));
    }
}
