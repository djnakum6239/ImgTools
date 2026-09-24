package com.djnakum.imgtools

import android.content.ContentResolver
import android.net.Uri
import android.os.ParcelFileDescriptor
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.FileInputStream
import java.nio.ByteBuffer
import java.nio.channels.FileChannel

/**
 * Random-access source for SAF documents.
 *
 * ImageForge's package/partition engines rely on range reads instead of loading a complete
 * multi-gigabyte image into memory. A seekable SAF descriptor gives Android the same primitive.
 */
class UriByteSource(
    private val resolver: ContentResolver,
    private val uri: Uri,
) : AutoCloseable {
    private val descriptor: ParcelFileDescriptor =
        requireNotNull(resolver.openFileDescriptor(uri, "r")) {
            "The selected document cannot be opened for reading."
        }

    private val channel: FileChannel = FileInputStream(descriptor.fileDescriptor).channel

    val size: Long
        get() = descriptor.statSize.takeIf { it >= 0 } ?: channel.size()

    suspend fun read(offset: Long, length: Int): ByteArray = withContext(Dispatchers.IO) {
        require(offset >= 0) { "offset must be non-negative" }
        require(length >= 0) { "length must be non-negative" }
        if (length == 0 || offset >= size) return@withContext ByteArray(0)

        val actual = minOf(length.toLong(), size - offset).toInt()
        val buffer = ByteBuffer.allocate(actual)
        var position = offset
        while (buffer.hasRemaining()) {
            val count = channel.read(buffer, position)
            if (count < 0) break
            if (count == 0) break
            position += count
        }
        if (buffer.position() == actual) buffer.array()
        else buffer.array().copyOf(buffer.position())
    }

    suspend fun readPrefix(maxBytes: Int = 65536): ByteArray = read(0, maxBytes)

    override fun close() {
        channel.close()
        descriptor.close()
    }
}
