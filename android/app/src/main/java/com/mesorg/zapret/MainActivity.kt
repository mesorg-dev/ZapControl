package com.mesorg.zapret

import android.content.pm.PackageManager
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.ui.graphics.Color
import androidx.core.content.ContextCompat
import com.mesorg.zapret.ui.App

private val DarkColors = darkColorScheme(
    primary = Color(0xFFE0E0E0),
    background = Color.Black,
    surface = Color.Black,
    onPrimary = Color.Black,
    onBackground = Color(0xFFE0E0E0),
    onSurface = Color(0xFFE0E0E0),
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme(colorScheme = DarkColors) {
                App(
                    onRequestVpn = { onGranted ->
                        val prepare = VpnService.prepare(this@MainActivity)
                        if (prepare == null) {
                            onGranted()
                        } else {
                            vpnConsentLauncher.launch(prepare)
                            pendingVpnGrant = onGranted
                        }
                    },
                    onStopVpn = {
                        com.mesorg.zapret.vpn.ZapretVpnService.stop(this@MainActivity)
                    },
                )
            }
        }
        maybeRequestNotifications()
    }

    private var pendingVpnGrant: (() -> Unit)? = null

    private val vpnConsentLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            pendingVpnGrant?.invoke()
        }
        pendingVpnGrant = null
    }

    private fun maybeRequestNotifications() {
        if (Build.VERSION.SDK_INT >= 33) {
            val granted = ContextCompat.checkSelfPermission(
                this,
                android.Manifest.permission.POST_NOTIFICATIONS,
            ) == PackageManager.PERMISSION_GRANTED
            if (!granted) {
                notifPermissionLauncher.launch(android.Manifest.permission.POST_NOTIFICATIONS)
            }
        }
    }

    private val notifPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { /* best-effort */ }
}
