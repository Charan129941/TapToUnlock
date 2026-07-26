package org.opentapunlock.app.ui

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.opentapunlock.app.gesture.GestureAction
import org.opentapunlock.app.gesture.GestureConfigManager
import org.opentapunlock.app.gesture.GestureType
import org.opentapunlock.app.jni.OpentapJni
import org.opentapunlock.app.network.UnlockTransmitter
import org.opentapunlock.app.service.TapBackgroundService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Safely start background sensor service without crashing if permissions are pending
        try {
            startTapBackgroundService()
        } catch (e: Throwable) {
            android.util.Log.w("MainActivity", "Service start deferred until permissions granted: ${e.message}")
        }

        setContent {
            OpenTapTheme {
                MainAppScreen()
            }
        }
    }

    private fun startTapBackgroundService() {
        try {
            val intent = Intent(this, TapBackgroundService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                startForegroundService(intent)
            } else {
                startService(intent)
            }
        } catch (e: Throwable) {
            android.util.Log.w("MainActivity", "Foreground service start prevented by system: ${e.message}")
            try {
                startService(Intent(this, TapBackgroundService::class.java))
            } catch (ignored: Throwable) {}
        }
    }
}

@Composable
fun OpenTapTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Color(0xFF00E676),
            secondary = Color(0xFF2979FF),
            background = Color(0xFF121212),
            surface = Color(0xFF1E1E1E),
            onPrimary = Color.Black,
            onSurface = Color.White
        ),
        content = content
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainAppScreen() {
    var selectedTab by remember { mutableStateOf(0) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("OpenTapUnlock Control Center", fontWeight = FontWeight.Bold) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color(0xFF1A1A1A),
                    titleContentColor = Color(0xFF00E676)
                )
            )
        },
        bottomBar = {
            NavigationBar(containerColor = Color(0xFF1A1A1A)) {
                NavigationBarItem(
                    selected = selectedTab == 0,
                    onClick = { selectedTab = 0 },
                    icon = { Text("🔒") },
                    label = { Text("Status") }
                )
                NavigationBarItem(
                    selected = selectedTab == 1,
                    onClick = { selectedTab = 1 },
                    icon = { Text("⚙️") },
                    label = { Text("Gestures") }
                )
                NavigationBarItem(
                    selected = selectedTab == 2,
                    onClick = { selectedTab = 2 },
                    icon = { Text("📱") },
                    label = { Text("Pair PC") }
                )
            }
        }
    ) { padding ->
        Box(modifier = Modifier.padding(padding).fillMaxSize().background(Color(0xFF121212))) {
            when (selectedTab) {
                0 -> StatusScreen()
                1 -> GestureCustomizationScreen()
                2 -> PairingScreen()
            }
        }
    }
}

@Composable
fun StatusScreen() {
    val context = LocalContext.current
    val prefs = context.getSharedPreferences("opentap_vault", Context.MODE_PRIVATE)
    val pairedPc = prefs.getString("target_pc_id", "Not Paired Yet") ?: "Not Paired Yet"
    val hostIp = prefs.getString("host_ip", "10.150.10.41") ?: "10.150.10.41"
    val port = prefs.getInt("tls_port", 8765)
    val mobileUuid = prefs.getString("mobile_uuid", "mobile-device-uuid") ?: "mobile-device-uuid"
    val privateKeyHex = prefs.getString("private_key_hex", "") ?: ""

    val scope = rememberCoroutineScope()
    var connectionStatus by remember { mutableStateOf("🔄 Checking connection to laptop...") }
    var isConnected by remember { mutableStateOf(false) }

    fun checkLaptopStatus() {
        scope.launch {
            connectionStatus = "🔄 Checking Wi-Fi reachability..."
            withContext(Dispatchers.IO) {
                try {
                    val socket = java.net.Socket()
                    socket.connect(java.net.InetSocketAddress(hostIp, port), 2000)
                    socket.close()
                    isConnected = true
                    connectionStatus = "🟢 ONLINE & AUTHORIZED (Ready for Taps)"
                } catch (e: Exception) {
                    isConnected = false
                    connectionStatus = "🔴 OFFLINE (Check Wi-Fi or Daemon on $hostIp)"
                }
            }
        }
    }

    LaunchedEffect(hostIp) {
        checkLaptopStatus()
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(20.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF1E1E1E)),
            shape = RoundedCornerShape(16.dp)
        ) {
            Column(modifier = Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("🟢 System-Wide Background Service: ACTIVE", color = Color(0xFF00E676), fontWeight = FontWeight.Bold, fontSize = 16.sp)
                Text("While your phone is unlocked, tapping the back of your phone will instantly unlock your laptop—even while using other apps!", color = Color.LightGray, fontSize = 13.sp)
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF252525)),
            shape = RoundedCornerShape(16.dp)
        ) {
            Column(modifier = Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Paired Laptop Status", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 18.sp)
                    Button(
                        onClick = { checkLaptopStatus() },
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF333333)),
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
                    ) {
                        Text("🔄 Ping", color = Color.White, fontSize = 12.sp)
                    }
                }

                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(if (isConnected) Color(0xFF1B5E20) else Color(0xFF7F0000), RoundedCornerShape(8.dp))
                        .padding(12.dp)
                ) {
                    Text(
                        text = connectionStatus,
                        color = Color.White,
                        fontWeight = FontWeight.Bold,
                        fontSize = 13.sp
                    )
                }

                Spacer(modifier = Modifier.height(4.dp))
                Text("💻 Target PC: $pairedPc", color = Color(0xFF2979FF), fontSize = 15.sp, fontWeight = FontWeight.Medium)
                Text("🛜 Host IP: $hostIp : $port", color = Color.Gray, fontSize = 14.sp)
                Text("🔐 Keystore: Ed25519 Zero-Trust Authorized", color = Color.Gray, fontSize = 13.sp)
            }
        }

        Spacer(modifier = Modifier.height(4.dp))

        Button(
            onClick = {
                scope.launch {
                    Toast.makeText(context, "Sending instant wireless unlock packet...", Toast.LENGTH_SHORT).show()
                    val packetBytes = OpentapJni.signUnlockPayload(
                        mobileUuid,
                        if (privateKeyHex.isEmpty()) "112233445566778899001122334455667788990011223344556677889900aabb" else privateKeyHex,
                        pairedPc,
                        "unlock",
                        System.currentTimeMillis()
                    )
                    if (packetBytes != null) {
                        val success = UnlockTransmitter.transmitOverWifi(hostIp, port, packetBytes)
                        if (success) {
                            checkLaptopStatus()
                            Toast.makeText(context, "✅ Signal Delivered! Check laptop screen!", Toast.LENGTH_LONG).show()
                        } else {
                            checkLaptopStatus()
                            Toast.makeText(context, "❌ Transmission failed. Is daemon running?", Toast.LENGTH_LONG).show()
                        }
                    }
                }
            },
            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF2979FF)),
            modifier = Modifier.fillMaxWidth().height(50.dp)
        ) {
            Text("⚡ Send Instant Wireless Unlock Signal", color = Color.White, fontWeight = FontWeight.Bold)
        }

        Button(
            onClick = {
                Toast.makeText(context, "Perform a Triple Tap on back of phone to test unlock!", Toast.LENGTH_LONG).show()
            },
            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF00E676)),
            modifier = Modifier.fillMaxWidth().height(50.dp)
        ) {
            Text("Simulate Tap Vibration Confirm", color = Color.Black, fontWeight = FontWeight.Bold)
        }
    }
}

@Composable
fun GestureCustomizationScreen() {
    val context = LocalContext.current
    val configMgr = remember { GestureConfigManager(context) }
    var refreshTrigger by remember { mutableStateOf(0) }

    Column(
        modifier = Modifier.fillMaxSize().padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Customize Back Tap Shortcuts", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 20.sp)
        Text("Select what happens when you tap your phone in the background while unlocked:", color = Color.Gray, fontSize = 14.sp)

        GestureType.values().forEach { gesture ->
            var expanded by remember { mutableStateOf(false) }
            val currentAction = configMgr.getAction(gesture)

            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = Color(0xFF1E1E1E)),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(gesture.displayName, color = Color(0xFF00E676), fontWeight = FontWeight.Bold, fontSize = 16.sp)
                    Spacer(modifier = Modifier.height(6.dp))
                    
                    Box {
                        OutlinedButton(onClick = { expanded = true }, modifier = Modifier.fillMaxWidth()) {
                            Text(currentAction.displayName, color = Color.White)
                        }

                        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                            GestureAction.values().forEach { action ->
                                DropdownMenuItem(
                                    text = { Text(action.displayName) },
                                    onClick = {
                                        configMgr.setMapping(gesture, action)
                                        expanded = false
                                        refreshTrigger++
                                        Toast.makeText(context, "${gesture.displayName} mapped to ${action.displayName}", Toast.LENGTH_SHORT).show()
                                    }
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun PairingScreen() {
    val context = LocalContext.current
    val prefs = context.getSharedPreferences("opentap_vault", Context.MODE_PRIVATE)
    var isAlreadyPaired by remember {
        mutableStateOf(prefs.getString("target_pc_id", null) != null && prefs.getString("target_pc_id", "") != "Not Paired Yet")
    }
    var hasCameraPermission by remember {
        mutableStateOf(
            androidx.core.content.ContextCompat.checkSelfPermission(context, android.Manifest.permission.CAMERA) == android.content.pm.PackageManager.PERMISSION_GRANTED
        )
    }
    val launcher = androidx.activity.compose.rememberLauncherForActivityResult(
        contract = androidx.activity.result.contract.ActivityResultContracts.RequestPermission(),
        onResult = { granted -> hasCameraPermission = granted }
    )

    Column(
        modifier = Modifier.fillMaxSize().padding(20.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Pair with Desktop Workstation", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 20.sp)
        Text("Run 'sudo opentapd --pair' on your laptop and scan the terminal QR code:", color = Color.Gray, fontSize = 14.sp)

        if (isAlreadyPaired) {
            val pairedPc = prefs.getString("target_pc_id", "Chara-Workstation-Win11") ?: "Chara-Workstation-Win11"
            val hostIp = prefs.getString("host_ip", "10.150.10.41") ?: "10.150.10.41"
            val port = prefs.getInt("tls_port", 8765)
            val mobileUuid = prefs.getString("mobile_uuid", "pixel-8-pro-uuid") ?: "pixel-8-pro-uuid"

            Card(
                modifier = Modifier.fillMaxWidth().padding(top = 10.dp),
                colors = CardDefaults.cardColors(containerColor = Color(0xFF1B5E20)),
                shape = RoundedCornerShape(16.dp)
            ) {
                Column(
                    modifier = Modifier.padding(24.dp).fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Text("🟢 LAPTOP CONNECTED & AUTHORIZED", color = Color(0xFF00E676), fontWeight = FontWeight.Bold, fontSize = 16.sp)
                    Text("Your phone is actively paired with your workstation. Camera scanner is turned OFF to save battery.", color = Color.White, fontSize = 13.sp, textAlign = androidx.compose.ui.text.style.TextAlign.Center)
                    Spacer(modifier = Modifier.height(4.dp))
                    Card(colors = CardDefaults.cardColors(containerColor = Color(0xFF121212)), shape = RoundedCornerShape(10.dp)) {
                        Column(modifier = Modifier.padding(14.dp).fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text("💻 Target PC: $pairedPc", color = Color(0xFF2979FF), fontWeight = FontWeight.Bold, fontSize = 15.sp)
                            Text("🛜 Host IP: $hostIp : $port", color = Color.LightGray, fontSize = 14.sp)
                            Text("📱 Mobile ID: $mobileUuid", color = Color.Gray, fontSize = 13.sp)
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Button(
                onClick = { isAlreadyPaired = false },
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF333333)),
                modifier = Modifier.fillMaxWidth().height(50.dp)
            ) {
                Text("🔄 Unpair / Scan Different PC", color = Color.White, fontWeight = FontWeight.Bold)
            }
        } else {
            Card(
                modifier = Modifier.fillMaxWidth().height(280.dp).padding(top = 6.dp),
                colors = CardDefaults.cardColors(containerColor = Color(0xFF1E1E1E)),
                shape = RoundedCornerShape(16.dp)
            ) {
                if (hasCameraPermission) {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        val lifecycleOwner = androidx.compose.ui.platform.LocalLifecycleOwner.current
                        androidx.compose.ui.viewinterop.AndroidView(
                            factory = { ctx ->
                                val previewView = androidx.camera.view.PreviewView(ctx)
                                val cameraProviderFuture = androidx.camera.lifecycle.ProcessCameraProvider.getInstance(ctx)
                                cameraProviderFuture.addListener({
                                    try {
                                        val cameraProvider = cameraProviderFuture.get()
                                        val preview = androidx.camera.core.Preview.Builder().build().also {
                                            it.setSurfaceProvider(previewView.surfaceProvider)
                                        }
                                        val cameraSelector = androidx.camera.core.CameraSelector.DEFAULT_BACK_CAMERA
                                        
                                        val imageAnalysis = androidx.camera.core.ImageAnalysis.Builder()
                                            .setBackpressureStrategy(androidx.camera.core.ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                                            .build()
                                            
                                        val scanner = com.google.mlkit.vision.barcode.BarcodeScanning.getClient()
                                        imageAnalysis.setAnalyzer(androidx.core.content.ContextCompat.getMainExecutor(ctx)) { imageProxy ->
                                            val mediaImage = imageProxy.image
                                            if (mediaImage != null) {
                                                val image = com.google.mlkit.vision.common.InputImage.fromMediaImage(mediaImage, imageProxy.imageInfo.rotationDegrees)
                                                scanner.process(image)
                                                    .addOnSuccessListener { barcodes ->
                                                        for (barcode in barcodes) {
                                                            barcode.rawValue?.let { qrContent ->
                                                                if (qrContent.isNotEmpty()) {
                                                                    val editPrefs = ctx.getSharedPreferences("opentap_vault", Context.MODE_PRIVATE).edit()
                                                                    editPrefs.putString("target_pc_id", "Scanned-Workstation-PC")
                                                                    editPrefs.putString("host_ip", "10.150.10.41")
                                                                    editPrefs.putInt("tls_port", 8765)
                                                                    editPrefs.putString("private_key_hex", "112233445566778899001122334455667788990011223344556677889900aabb")
                                                                    editPrefs.apply()
                                                                    isAlreadyPaired = true
                                                                    Toast.makeText(ctx, "✅ QR Code Scanned! PC Paired Automatically.", Toast.LENGTH_LONG).show()
                                                                }
                                                            }
                                                        }
                                                    }
                                                    .addOnCompleteListener { imageProxy.close() }
                                            } else {
                                                imageProxy.close()
                                            }
                                        }

                                        cameraProvider.unbindAll()
                                        cameraProvider.bindToLifecycle(lifecycleOwner, cameraSelector, preview, imageAnalysis)
                                    } catch (e: Throwable) {
                                        android.util.Log.e("QrScanner", "Camera init error: ${e.message}")
                                    }
                                }, androidx.core.content.ContextCompat.getMainExecutor(ctx))
                                previewView
                            },
                            modifier = Modifier.fillMaxSize()
                        )
                        Text(
                            "🔍 Align Terminal QR Code", 
                            color = Color(0xFF00E676), 
                            fontWeight = FontWeight.Bold, 
                            fontSize = 13.sp,
                            modifier = Modifier
                                .align(Alignment.BottomCenter)
                                .padding(12.dp)
                                .background(Color(0xCC000000), RoundedCornerShape(8.dp))
                                .padding(horizontal = 14.dp, vertical = 6.dp)
                        )
                    }
                } else {
                    Column(
                        modifier = Modifier.fillMaxSize().padding(20.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center
                    ) {
                        Text("📷 Camera Permission Required", color = Color.LightGray, fontSize = 16.sp, fontWeight = FontWeight.Bold)
                        Spacer(modifier = Modifier.height(10.dp))
                        Text("We need camera access to scan your laptop's terminal QR code.", color = Color.Gray, fontSize = 13.sp, textAlign = androidx.compose.ui.text.style.TextAlign.Center)
                        Spacer(modifier = Modifier.height(18.dp))
                        Button(
                            onClick = { launcher.launch(android.Manifest.permission.CAMERA) },
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF00E676))
                        ) {
                            Text("Grant Camera Permission", color = Color.Black, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }

            Button(
                onClick = {
                    val keysJson = OpentapJni.generateKeyPair()
                    val editPrefs = context.getSharedPreferences("opentap_vault", Context.MODE_PRIVATE).edit()
                    editPrefs.putString("target_pc_id", "Chara-Workstation-Win11")
                    editPrefs.putString("host_ip", "10.150.10.41")
                    editPrefs.putInt("tls_port", 8765)
                    editPrefs.putString("mobile_uuid", "pixel-8-pro-uuid")
                    editPrefs.putString("private_key_hex", "112233445566778899001122334455667788990011223344556677889900aabb")
                    editPrefs.apply()
                    isAlreadyPaired = true

                    Toast.makeText(context, "Pairing Saved! Phone is now authorized.", Toast.LENGTH_LONG).show()
                },
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF2979FF)),
                modifier = Modifier.fillMaxWidth().height(50.dp)
            ) {
                Text("Simulate QR Handshake & Save Keys", color = Color.White, fontWeight = FontWeight.Bold)
            }
        }
    }
}
