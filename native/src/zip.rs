//! Range-based ZIP central-directory reader.
//!
//! This mirrors the upstream ImageForge ZIP boundary: the archive is never loaded wholesale.
//! Only the tail, central directory, and individual local headers are read.

use crate::bytesource::{ByteSource, ByteSourceError};

const LOCAL_HEADER: u32 = 0x0403_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const EOCD: u32 = 0x0605_4b50;
const ZIP64_EOCD: u32 = 0x0606_4b50;
const ZIP64_LOCATOR: u32 = 0x0706_4b50;
const ZIP64_EXTRA: u16 = 0x0001;
const ZIP64_SENTINEL: u32 = 0xffff_ffff;
const ZIP64_SENTINEL_16: u16 = 0xffff;
const END_RECORD_WINDOW: u64 = 0xffff + 22 + 76 + 0xffff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipError {
    TooShort,
    MissingEocd,
    InvalidEocd,
    InvalidZip64,
    Truncated,
    Damaged(String),
    Unsupported(String),
    Source(ByteSourceError),
}

impl From<ByteSourceError> for ZipError {
    fn from(value: ByteSourceError) -> Self { Self::Source(value) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    pub name: String,
    pub method: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub data_offset: u64,
}

#[derive(Debug, Clone, Copy)]
struct EndRecord {
    entry_count: u64,
    central_size: u64,
    central_offset: u64,
}

fn u16le(b: &[u8], o: usize) -> Result<u16, ZipError> {
    if o.checked_add(2).map_or(true, |e| e > b.len()) { return Err(ZipError::InvalidEocd); }
    Ok(u16::from_le_bytes([b[o], b[o + 1]]))
}

fn u32le(b: &[u8], o: usize) -> Result<u32, ZipError> {
    if o.checked_add(4).map_or(true, |e| e > b.len()) { return Err(ZipError::InvalidEocd); }
    Ok(u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]))
}

fn u64le(b: &[u8], o: usize) -> Result<u64, ZipError> {
    if o.checked_add(8).map_or(true, |e| e > b.len()) { return Err(ZipError::InvalidZip64); }
    Ok(u64::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7]]))
}

fn add(a: u64, b: u64) -> Result<u64, ZipError> {
    a.checked_add(b).ok_or(ZipError::Damaged("ZIP offset overflow".into()))
}

fn read_end<S: ByteSource>(source: &mut S) -> Result<EndRecord, ZipError> {
    if source.size() < 22 { return Err(ZipError::TooShort); }
    let window = END_RECORD_WINDOW.min(source.size());
    let tail = source.read(source.size() - window, window as usize)?;
    let mut at = None;
    if tail.len() >= 22 {
        for i in (0..=tail.len() - 22).rev() {
            if u32le(&tail, i).ok() == Some(EOCD) {
                at = Some(i);
                break;
            }
        }
    }
    let eocd = at.ok_or(ZipError::MissingEocd)?;
    let mut record = EndRecord {
        entry_count: u16le(&tail, eocd + 10)? as u64,
        central_size: u32le(&tail, eocd + 12)? as u64,
        central_offset: u32le(&tail, eocd + 16)? as u64,
    };
    let locator = eocd.checked_sub(20);
    if let Some(locator) = locator {
        if u32le(&tail, locator).ok() == Some(ZIP64_LOCATOR) {
            let record_offset = u64le(&tail, locator + 8)?;
            let record64 = source.read(record_offset, 56)?;
            if record64.len() < 56 || u32le(&record64, 0)? != ZIP64_EOCD {
                return Err(ZipError::InvalidZip64);
            }
            record.entry_count = u64le(&record64, 32)?;
            record.central_size = u64le(&record64, 40)?;
            record.central_offset = u64le(&record64, 48)?;
        } else if record.entry_count == ZIP64_SENTINEL_16 as u64
            || record.central_offset == ZIP64_SENTINEL as u64 {
            return Err(ZipError::InvalidZip64);
        }
    }
    Ok(record)
}

fn zip64_values(extra: &[u8], mut uncompressed: u64, mut compressed: u64, mut local_offset: u64)
    -> Result<(u64, u64, u64), ZipError>
{
    let mut p = 0usize;
    while p.checked_add(4).map_or(false, |e| e <= extra.len()) {
        let id = u16le(extra, p).map_err(|_| ZipError::InvalidZip64)?;
        let len = u16le(extra, p + 2).map_err(|_| ZipError::InvalidZip64)? as usize;
        let end = p.checked_add(4).and_then(|v| v.checked_add(len))
            .ok_or(ZipError::InvalidZip64)?;
        if end > extra.len() { return Err(ZipError::InvalidZip64); }
        if id == ZIP64_EXTRA {
            let mut f = p + 4;
            if uncompressed == ZIP64_SENTINEL as u64 {
                uncompressed = u64le(extra, f)?; f += 8;
            }
            if compressed == ZIP64_SENTINEL as u64 {
                compressed = u64le(extra, f)?; f += 8;
            }
            if local_offset == ZIP64_SENTINEL as u64 {
                local_offset = u64le(extra, f)?;
            }
        }
        p = end;
    }
    Ok((uncompressed, compressed, local_offset))
}

pub fn list<S: ByteSource>(source: &mut S) -> Result<Vec<ZipEntry>, ZipError> {
    let end = read_end(source)?;
    let central = source.read(end.central_offset, usize::try_from(end.central_size)
        .map_err(|_| ZipError::Unsupported("central directory is too large".into()))?)?;
    if central.len() as u64 != end.central_size { return Err(ZipError::Truncated); }

    let mut entries = Vec::with_capacity(usize::try_from(end.entry_count).unwrap_or(0));
    let mut cursor = 0usize;
    for index in 0..end.entry_count {
        if cursor.checked_add(46).map_or(true, |e| e > central.len())
            || u32le(&central, cursor).ok() != Some(CENTRAL_HEADER) {
            return Err(ZipError::Damaged(format!("central directory entry {} is damaged", index + 1)));
        }
        let method = u16le(&central, cursor + 10)?;
        let crc32 = u32le(&central, cursor + 16)?;
        let compressed = u32le(&central, cursor + 20)? as u64;
        let uncompressed = u32le(&central, cursor + 24)? as u64;
        let name_len = u16le(&central, cursor + 28)? as usize;
        let extra_len = u16le(&central, cursor + 30)? as usize;
        let comment_len = u16le(&central, cursor + 32)? as usize;
        let local_offset = u32le(&central, cursor + 42)? as u64;
        let name_start = cursor + 46;
        let name_end = name_start.checked_add(name_len).ok_or(ZipError::Damaged("name overflow".into()))?;
        let extra_end = name_end.checked_add(extra_len).ok_or(ZipError::Damaged("extra overflow".into()))?;
        let next = extra_end.checked_add(comment_len).ok_or(ZipError::Damaged("comment overflow".into()))?;
        if next > central.len() { return Err(ZipError::Truncated); }
        let name = String::from_utf8_lossy(&central[name_start..name_end]).into_owned();
        let (uncompressed, compressed, local_offset) =
            zip64_values(&central[name_end..extra_end], uncompressed, compressed, local_offset)?;

        let local = source.read(local_offset, 30)?;
        if local.len() < 30 || u32le(&local, 0).ok() != Some(LOCAL_HEADER) {
            return Err(ZipError::Damaged(format!("local header for {} is invalid", name)));
        }
        let local_name_len = u16le(&local, 26)? as u64;
        let local_extra_len = u16le(&local, 28)? as u64;
        let data_offset = add(add(local_offset, 30)?, add(local_name_len, local_extra_len)?)?;
        entries.push(ZipEntry { name, method, crc32, compressed_size: compressed, uncompressed_size: uncompressed, data_offset });
        cursor = next;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesource::{ByteSource, MemorySource};

    fn zip_one(name: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&LOCAL_HEADER.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        let local_offset = 0u32;
        let central_offset = out.len() as u32;
        out.extend_from_slice(&CENTRAL_HEADER.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&local_offset.to_le_bytes());
        out.extend_from_slice(name);
        let central_size = (out.len() as u32) - central_offset;
        out.extend_from_slice(&EOCD.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&central_size.to_le_bytes());
        out.extend_from_slice(&central_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn lists_stored_entry_without_reading_archive_whole() {
        let bytes = zip_one(b"payload.bin", b"hello");
        let mut source = MemorySource::new(bytes);
        let entries = list(&mut source).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "payload.bin");
        assert_eq!(entries[0].compressed_size, 5);
        assert_eq!(source.read(entries[0].data_offset, 5).unwrap(), b"hello");
    }

    #[test]
    fn rejects_non_zip() {
        let mut source = MemorySource::new(b"not a zip".to_vec());
        assert_eq!(list(&mut source), Err(ZipError::TooShort));
    }

    #[test]
    fn rejects_damaged_central_directory() {
        let mut bytes = zip_one(b"a", b"b");
        let eocd = bytes.len() - 22;
        let central_offset = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        bytes[central_offset] = 0;
        let mut source = MemorySource::new(bytes);
        assert!(matches!(list(&mut source), Err(ZipError::Damaged(_))));
    }
}
