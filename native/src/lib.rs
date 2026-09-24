mod imageforge_core;
mod boot;
mod bytesource;
mod binary;
use jni::objects::{JByteArray, JClass};
use jni::sys::{jint, jlong};
use jni::JNIEnv;

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_djnakum_imgtools_NativeEngine_version(_env: JNIEnv, _class: JClass) -> jint {
    imageforge_core::imageforge_version() as jint
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_djnakum_imgtools_NativeEngine_crc32(mut env: JNIEnv, _class: JClass, data: JByteArray) -> jlong {
    let bytes = match env.convert_byte_array(data) { Ok(v) => v, Err(_) => return -1 };
    if bytes.is_empty() { return 0; }
    unsafe { imageforge_core::imageforge_crc32(bytes.as_ptr() as *const u8, bytes.len()) as jlong }
}

/// Format identifiers intentionally mirror the upstream workspace detector's content/container split.
pub mod detect {
    pub const UNKNOWN: i32 = 0;
    pub const BOOT: i32 = 1;
    pub const INIT_BOOT: i32 = 13;
    pub const VENDOR_BOOT: i32 = 2;
    pub const SPARSE: i32 = 3;
    pub const OTA_PAYLOAD: i32 = 4;
    pub const ZIP: i32 = 5;
    pub const EXT4: i32 = 6;
    pub const EROFS: i32 = 7;
    pub const F2FS: i32 = 8;
    pub const DTB: i32 = 9;
    pub const ELF: i32 = 10;
    pub const SUPER: i32 = 11;
    pub const CPIO: i32 = 12;

    fn magic_at(bytes: &[u8], offset: usize, magic: &[u8]) -> bool {
        bytes.len() >= offset.saturating_add(magic.len())
            && &bytes[offset..offset + magic.len()] == magic
    }

    pub fn detect(bytes: &[u8]) -> i32 {
        if magic_at(bytes, 0, b"ANDROID!") {
            if let Ok(header) = crate::boot::parse_boot_header(bytes) {
                if header.header_version >= 4 && header.kernel_size == 0 && header.ramdisk_size > 0 {
                    return INIT_BOOT;
                }
            }
            return BOOT;
        }
        if magic_at(bytes, 0, b"VNDRBOOT") { return VENDOR_BOOT; }
        if magic_at(bytes, 0, &[0x3a, 0xff, 0x26, 0xed]) { return SPARSE; }
        if magic_at(bytes, 0, b"CrAU") { return OTA_PAYLOAD; }
        if magic_at(bytes, 0, &[0x50, 0x4b, 0x03, 0x04])
            || magic_at(bytes, 0, &[0x50, 0x4b, 0x05, 0x06]) { return ZIP; }
        if magic_at(bytes, 0, b"070701")
            || magic_at(bytes, 0, b"070702")
            || magic_at(bytes, 0, b"070707") { return CPIO; }
        if magic_at(bytes, 0x438, &[0x53, 0xef]) { return EXT4; }
        if magic_at(bytes, 1024, &[0xe2, 0xe1, 0xf5, 0xe0]) { return EROFS; }
        if magic_at(bytes, 1024, &[0x10, 0x20, 0xf5, 0xf2]) { return F2FS; }
        if magic_at(bytes, 0, &[0xd0, 0x0d, 0xfe, 0xed]) { return DTB; }
        if magic_at(bytes, 0, b"\x7fELF") { return ELF; }
        // AOSP LP_METADATA_GEOMETRY_MAGIC = 0x616c4467, stored little-endian as "gDla".
        if magic_at(bytes, 0, &[0x67, 0x44, 0x6c, 0x61])
            || magic_at(bytes, 4096, &[0x67, 0x44, 0x6c, 0x61]) {
            return SUPER;
        }
        UNKNOWN
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_djnakum_imgtools_NativeEngine_detectFormat(
    mut env: JNIEnv,
    _class: JClass,
    data: JByteArray,
) -> jint {
    let bytes = match env.convert_byte_array(data) {
        Ok(v) => v,
        Err(_) => return detect::UNKNOWN,
    };
    detect::detect(&bytes)
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn detects_boot_and_vendor_headers() {
        assert_eq!(detect::detect(b"ANDROID!"), detect::BOOT);
        assert_eq!(detect::detect(b"VNDRBOOT"), detect::VENDOR_BOOT);
    }

    #[test]
    fn detects_common_container_magics() {
        assert_eq!(detect::detect(&[0x3a, 0xff, 0x26, 0xed]), detect::SPARSE);
        assert_eq!(detect::detect(b"CrAU"), detect::OTA_PAYLOAD);
        assert_eq!(detect::detect(&[0x50, 0x4b, 0x03, 0x04]), detect::ZIP);
        assert_eq!(detect::detect(b"070701"), detect::CPIO);
    }
}
