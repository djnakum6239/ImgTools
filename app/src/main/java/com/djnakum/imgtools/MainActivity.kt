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

private object NativeEngine {
    init { System.loadLibrary("imgtools") }
    external fun version(): Int
    external fun crc32(data: ByteArray): Long
}

data class Tool(val title: String, val description: String)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { ImgToolsApp() }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ImgToolsApp() {
    var selected by remember { mutableStateOf<Uri?>(null) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { selected = it }
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
                }
                items(tools) { tool ->
                    ElevatedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(16.dp)) {
                            Text(tool.title, style = MaterialTheme.typography.titleMedium)
                            Spacer(Modifier.height(4.dp)); Text(tool.description)
                            Spacer(Modifier.height(8.dp)); OutlinedButton(onClick = { picker.launch(arrayOf("*/*")) }) { Text("Open local file") }
                        }
                    }
                }
                item { Text("Native engine v${NativeEngine.version()}", style = MaterialTheme.typography.labelMedium) }
            }
        }
    }
}
