mod imageforge_core;
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
