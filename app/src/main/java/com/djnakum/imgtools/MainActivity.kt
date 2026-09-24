package com.djnakum.imgtools

import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch

private object NativeEngine {
    init { System.loadLibrary("imgtools") }
    external fun version(): Int
    external fun crc32(data: ByteArray): Long
    external fun detectFormat(data: ByteArray): Int
}

private data class Detection(val label: String, val detail: String)

private fun detectionFor(code: Int): Detection = when (code) {
    1 -> Detection("Android boot image", "ANDROID! header")
    2 -> Detection("Android vendor_boot image", "VNDRBOOT header")
    3 -> Detection("Android sparse image", "libsparse header")
    4 -> Detection("Android OTA payload", "CrAU header")
    5 -> Detection("ZIP package", "PK archive header")
    6 -> Detection("ext4 filesystem", "ext4 superblock")
    7 -> Detection("EROFS filesystem", "EROFS superblock")
    8 -> Detection("F2FS filesystem", "F2FS superblock")
    9 -> Detection("Device tree blob", "DTB header")
    10 -> Detection("ELF object", "ELF header")
    11 -> Detection("Logical partition image", "liblp geometry")
    12 -> Detection("CPIO archive", "CPIO header")
    else -> Detection("Unknown format", "No supported ImageForge magic matched")
}

data class Tool(val title: String, val description: String)

class MainActivity : ComponentActivity() {
    private val detection = MutableStateFlow<Detection?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { ImgToolsApp() }
    }

    private fun chooseResult(uri: Uri?) {
        if (uri == null) return
        try {
            contentResolver.takePersistableUriPermission(
                uri,
                android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (_: SecurityException) {
            // Some providers do not offer persistable grants; the current grant remains usable.
        }
        lifecycleScope.launch {
            val result = runCatching {
                UriByteSource(contentResolver, uri).use { source ->
                    detectionFor(NativeEngine.detectFormat(source.readPrefix()))
                }
            }
            detection.value = result.getOrElse {
                Detection("Could not inspect file", it.message ?: "Unknown read error")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ImgToolsApp() {
    val activity = androidx.compose.ui.platform.LocalContext.current as MainActivity
    var selected by remember { mutableStateOf<Uri?>(null) }
    val detection by activity.detection.collectAsStateWithLifecycle()
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) {
        selected = it
        activity.chooseResult(it)
    }
    val tools = listOf(
        Tool("Inspect image", "Identify boot, init_boot, vendor_boot, sparse and other supported formats."),
        Tool("Extract package", "Extract supported OTA/package entries locally."),
        Tool("Unpack partitions", "Inspect sparse and super containers."),
        Tool("Patch image", "Local patch workflow for supported boot and ramdisk images."),
        Tool("Boot logo", "Inspect and rebuild supported logo containers."),
        Tool("Boot animation", "Inspect and rebuild bootanimation packages."),
        Tool("Compare images", "Compare local files and report differences.")
    )
    MaterialTheme {
        Scaffold(topBar = { TopAppBar(title = { Text("ImgTools") }) }) { padding ->
            LazyColumn(Modifier.padding(padding).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                item {
                    Text("Offline Android image toolkit", style = MaterialTheme.typography.headlineSmall)
                    Spacer(Modifier.height(6.dp))
                    Text("Processing is designed to stay on this device.")
                    Spacer(Modifier.height(12.dp))
                    Button(onClick = { picker.launch(arrayOf("*/*")) }) { Text("Choose image / package") }
                    selected?.let { Text("Selected: $it") }
                    detection?.let {
                        Spacer(Modifier.height(8.dp))
                        ElevatedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(12.dp)) {
                                Text(it.label, style = MaterialTheme.typography.titleMedium)
                                Text(it.detail, style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
                items(tools) { tool ->
                    ElevatedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(16.dp)) {
                            Text(tool.title, style = MaterialTheme.typography.titleMedium)
                            Spacer(Modifier.height(4.dp)); Text(tool.description)
                            Spacer(Modifier.height(8.dp))
                            OutlinedButton(onClick = { picker.launch(arrayOf("*/*")) }) { Text("Open local file") }
                        }
                    }
                }
                item { Text("Native engine v${NativeEngine.version()}", style = MaterialTheme.typography.labelMedium) }
            }
        }
    }
}
