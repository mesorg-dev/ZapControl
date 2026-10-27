package com.mesorg.zapret.vpn

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/** Process-wide VPN state observed by Compose UI. */
object VpnState {
    var running by mutableStateOf(false)
    var robloxApplied by mutableStateOf(false)
    var lastError by mutableStateOf<String?>(null)
}
