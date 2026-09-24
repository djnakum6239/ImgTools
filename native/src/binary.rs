use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryError {
    OutOfBounds { offset: usize, width: usize, len: usize },
    Overflow,
    InvalidAlignment,
}

impl fmt::Display for BinaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds { offset, width, len } =>
                write!(f, "range {offset}..{} is outside {len} bytes", offset.saturating_add(*width)),
            Self::Overflow => write!(f, "integer overflow"),
            Self::InvalidAlignment => write!(f, "alignment must be non-zero"),
        }
    }
}

impl std::error::Error for BinaryError {}

pub fn checked_range(len: usize, offset: usize, width: usize) -> Result<std::ops::Range<usize>, BinaryError> {
    let end = offset.checked_add(width).ok_or(BinaryError::Overflow)?;
    if end > len {
        return Err(BinaryError::OutOfBounds { offset, width, len });
    }
    Ok(offset..end)
}

pub fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, BinaryError> {
    let r = checked_range(bytes.len(), offset, 2)?;
    Ok(u16::from_le_bytes(bytes[r].try_into().unwrap()))
}

pub fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, BinaryError> {
    let r = checked_range(bytes.len(), offset, 4)?;
    Ok(u32::from_le_bytes(bytes[r].try_into().unwrap()))
}

pub fn read_u64_le(bytes: &[u8], offset: usize) -> Result<u64, BinaryError> {
    let r = checked_range(bytes.len(), offset, 8)?;
    Ok(u64::from_le_bytes(bytes[r].try_into().unwrap()))
}

pub fn read_u16_be(bytes: &[u8], offset: usize) -> Result<u16, BinaryError> {
    let r = checked_range(bytes.len(), offset, 2)?;
    Ok(u16::from_be_bytes(bytes[r].try_into().unwrap()))
}

pub fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, BinaryError> {
    let r = checked_range(bytes.len(), offset, 4)?;
    Ok(u32::from_be_bytes(bytes[r].try_into().unwrap()))
}

pub fn read_u64_be(bytes: &[u8], offset: usize) -> Result<u64, BinaryError> {
    let r = checked_range(bytes.len(), offset, 8)?;
    Ok(u64::from_be_bytes(bytes[r].try_into().unwrap()))
}

pub fn align_up(value: u64, alignment: u64) -> Result<u64, BinaryError> {
    if alignment == 0 {
        return Err(BinaryError::InvalidAlignment);
    }
    let rem = value % alignment;
    if rem == 0 {
        Ok(value)
    } else {
        value.checked_add(alignment - rem).ok_or(BinaryError::Overflow)
    }
}

pub fn write_u32_le(out: &mut [u8], offset: usize, value: u32) -> Result<(), BinaryError> {
    let r = checked_range(out.len(), offset, 4)?;
    out[r].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

pub fn write_u64_le(out: &mut [u8], offset: usize, value: u64) -> Result<(), BinaryError> {
    let r = checked_range(out.len(), offset, 8)?;
    out[r].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_endian_values() {
        let b = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        assert_eq!(read_u16_le(&b, 0).unwrap(), 0x0201);
        assert_eq!(read_u32_le(&b, 0).unwrap(), 0x04030201);
        assert_eq!(read_u64_le(&b, 0).unwrap(), 0x0807060504030201);
        assert_eq!(read_u32_be(&b, 0).unwrap(), 0x01020304);
    }

    #[test]
    fn rejects_out_of_range_and_overflow() {
        assert!(matches!(checked_range(4, 3, 2), Err(BinaryError::OutOfBounds { .. })));
        assert!(matches!(checked_range(usize::MAX, usize::MAX, 1), Err(BinaryError::Overflow)));
    }

    #[test]
    fn aligns_without_overflow() {
        assert_eq!(align_up(4097, 4096).unwrap(), 8192);
        assert_eq!(align_up(8192, 4096).unwrap(), 8192);
        assert!(matches!(align_up(1, 0), Err(BinaryError::InvalidAlignment)));
    }
}
