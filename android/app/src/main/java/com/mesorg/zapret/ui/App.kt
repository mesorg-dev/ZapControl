package com.mesorg.zapret.ui

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.mesorg.zapret.core.AssetBootstrap
import com.mesorg.zapret.core.CoreBridge
import com.mesorg.zapret.vpn.VpnState
import com.mesorg.zapret.vpn.ZapretVpnService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun App(
    onRequestVpn: (onGranted: () -> Unit) -> Unit,
    onStopVpn: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var robloxFix by remember { mutableStateOf(VpnState.robloxApplied) }
    var strategies by remember { mutableStateOf<List<CoreBridge.Strategy>>(emptyList()) }
    var selectedStrategyId by remember { mutableStateOf("general") }
    var nativeOk by remember { mutableStateOf<Boolean?>(null) }
    var logText by remember { mutableStateOf("[загрузка…]") }

    LaunchedEffect(Unit) {
        val result = withContext(Dispatchers.IO) {
            runCatching {
                val dir = AssetBootstrap.strategiesDir(context)
                val list = CoreBridge.loadStrategies(dir.absolutePath)
                val savedHosts = java.io.File(context.filesDir, "roblox-hosts.json")
                val hasSaved = savedHosts.exists() && savedHosts.readText().isNotBlank()
                list to (CoreBridge.isNativeAvailable() to hasSaved)
            }
        }
        result.fold(
            onSuccess = { (list, flags) ->
                val (ok, hasSaved) = flags
                strategies = list
                nativeOk = ok
                robloxFix = hasSaved || VpnState.robloxApplied
                if (list.isNotEmpty() && list.none { it.id == selectedStrategyId }) {
                    selectedStrategyId = list.first().id
                }
                logText = buildString {
                    append(if (ok) "[core] native OK\n" else "[core] native missing\n")
                    append("[core] стратегий: ${list.size}\n")
                    append("[m2] DPI-обход: TODO (userspace TCP), работает DNS-фикс")
                }
            },
            onFailure = { e ->
                nativeOk = false
                logText = "[core] ошибка: ${e.message}"
            },
        )
    }

    LaunchedEffect(VpnState.lastError) {
        VpnState.lastError?.let { logText = "[vpn] $it" }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // Header status
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            val dotColor = when {
                VpnState.lastError != null -> Color(0xFFF44336)
                VpnState.running -> Color(0xFF4CAF50)
                nativeOk == false -> Color(0xFFF44336)
                else -> Color(0xFFFFC107)
            }
            Box(
                modifier = Modifier
                    .size(12.dp)
                    .clip(CircleShape)
                    .background(dotColor),
            )
            Text(
                text = when {
                    VpnState.lastError != null -> "Ошибка"
                    VpnState.running -> "VPN работает"
                    else -> "Остановлен"
                },
                color = Color(0xFFE0E0E0),
                fontSize = 20.sp,
            )
        }

        HorizontalDivider(color = Color(0xFF222222))

        // Strategy picker (active list comes from Rust core)
        Text(
            text = "Стратегия (${strategies.size}) — Windows-режим",
            color = Color(0xFF888888),
            fontSize = 13.sp,
        )
        Button(
            onClick = {
                if (strategies.isEmpty()) return@Button
                val idx = strategies.indexOfFirst { it.id == selectedStrategyId }
                val next = (idx + 1).mod(strategies.size)
                selectedStrategyId = strategies[next].id
            },
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.buttonColors(
                containerColor = Color(0xFF1A1A1A),
                contentColor = Color(0xFFE0E0E0),
            ),
        ) {
            val current = strategies.find { it.id == selectedStrategyId }
            Text(
                text = current?.let { "${it.name} (${it.id})" } ?: selectedStrategyId,
                fontSize = 15.sp,
            )
        }

        HorizontalDivider(color = Color(0xFF222222))

        // Main toggle: start/stop the VPN (DNS engine for now)
        Button(
            onClick = {
                if (VpnState.running) {
                    onStopVpn()
                    logText = "[vpn] остановлен"
                } else {
                    onRequestVpn {
                        ZapretVpnService.start(context)
                        logText = "[vpn] запущен (dns-only:10.0.0.3/32)\n" +
                            if (robloxFix) "[vpn] roblox-hosts: активны" else "[vpn] roblox-hosts: выключены"
                    }
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(52.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = if (VpnState.running) Color(0xFF2A2A2A) else Color(0xFFE0E0E0),
                contentColor = if (VpnState.running) Color(0xFFF44336) else Color.Black,
            ),
        ) {
            Text(
                text = if (VpnState.running) "Остановить" else "Запустить",
                fontSize = 16.sp,
            )
        }

        // Roblox toggle: fetch JSON -> persist -> live reload
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "Roblox-фикс (DNS)",
                color = Color(0xFFE0E0E0),
                fontSize = 15.sp,
            )
            Switch(
                checked = robloxFix,
                onCheckedChange = { enabled ->
                    robloxFix = enabled
                    scope.launch {
                        if (enabled) {
                            logText = "[roblox] тяну JSON…"
                            val r = withContext(Dispatchers.IO) {
                                runCatching {
                                    val url = CoreBridge.robloxJsonUrl()
                                    val block = CoreBridge.fetchRobloxHosts(url)
                                    block
                                }
                            }
                            r.fold(
                                onSuccess = { block ->
                                    withContext(Dispatchers.IO) {
                                        ZapretVpnService.persistHosts(context, block)
                                    }
                                    logText = "[roblox] применён (${block.lineCount} строк)\n" +
                                        "[vpn] DNS-подмена активна"
                                },
                                onFailure = { e ->
                                    robloxFix = false
                                    VpnState.robloxApplied = false
                                    logText = "[roblox] ошибка: ${e.message}\n" +
                                        "[hint] задай URL JSON (settings) — сейчас заглушка CHANGE_ME"
                                    Toast.makeText(
                                        context,
                                        "Roblox JSON: ${e.message}",
                                        Toast.LENGTH_LONG,
                                    ).show()
                                },
                            )
                        } else {
                            withContext(Dispatchers.IO) {
                                java.io.File(context.filesDir, "roblox-hosts.json").writeText("")
                                VpnState.robloxApplied = false
                                if (VpnState.running) {
                                    ZapretVpnService.reloadHosts(context)
                                }
                            }
                            logText = "[roblox] выключен"
                        }
                    }
                },
                colors = SwitchDefaults.colors(
                    checkedThumbColor = Color(0xFFE0E0E0),
                    checkedTrackColor = Color(0xFF444444),
                    uncheckedThumbColor = Color(0xFF666666),
                    uncheckedTrackColor = Color(0xFF222222),
                ),
            )
        }

        Spacer(modifier = Modifier.height(8.dp))

        Text(
            text = "Лог:\n$logText",
            color = Color(0xFF666666),
            fontSize = 12.sp,
        )
    }
}
