//! Range-based byte sources mirroring ImageForge's ByteSource contract.
//!
//! The Android side supplies SAF ranges; native parsers should consume those ranges rather than
//! assuming an entire image is resident in memory.

pub trait ByteSource {
    fn size(&self) -> u64;
    fn read(&mut self, offset: u64, length: usize) -> Result<Vec<u8>, ByteSourceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteSourceError {
    OutOfRange,
    Io(String),
}

pub struct MemorySource {
    bytes: Vec<u8>,
}

impl MemorySource {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl ByteSource for MemorySource {
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read(&mut self, offset: u64, length: usize) -> Result<Vec<u8>, ByteSourceError> {
        let start = usize::try_from(offset).map_err(|_| ByteSourceError::OutOfRange)?;
        if start > self.bytes.len() {
            return Ok(Vec::new());
        }
        let end = start.saturating_add(length).min(self.bytes.len());
        Ok(self.bytes[start..end].to_vec())
    }
}

/// A bounded view of another source. This is used for nested ranges such as ZIP payloads and
/// logical partitions inside a super image.
pub struct SubSource<S> {
    parent: S,
    offset: u64,
    size: u64,
}

impl<S> SubSource<S> {
    pub fn new(parent: S, offset: u64, size: u64) -> Self {
        Self { parent, offset, size }
    }

    pub fn into_inner(self) -> S {
        self.parent
    }
}

impl<S: ByteSource> ByteSource for SubSource<S> {
    fn size(&self) -> u64 {
        self.size
    }

    fn read(&mut self, offset: u64, length: usize) -> Result<Vec<u8>, ByteSourceError> {
        let start = offset.min(self.size);
        let end = start.saturating_add(length as u64).min(self.size);
        if end == start {
            return Ok(Vec::new());
        }
        self.parent.read(self.offset + start, (end - start) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_source_clamps_reads() {
        let mut source = MemorySource::new(b"abcdef".to_vec());
        assert_eq!(source.read(2, 3).unwrap(), b"cde");
        assert_eq!(source.read(5, 20).unwrap(), b"f");
        assert_eq!(source.read(99, 2).unwrap(), b"");
    }

    #[test]
    fn sub_source_cannot_escape_window() {
        let mut source = SubSource::new(MemorySource::new(b"0123456789".to_vec()), 3, 4);
        assert_eq!(source.read(0, 10).unwrap(), b"3456");
        assert_eq!(source.read(3, 3).unwrap(), b"6");
    }
}
