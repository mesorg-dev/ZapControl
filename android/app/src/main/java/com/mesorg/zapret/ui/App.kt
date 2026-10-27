package com.mesorg.zapret.ui

import android.content.Context
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

@Composable
fun App() {
    val context = LocalContext.current
    var running by remember { mutableStateOf(false) }
    var robloxFix by remember { mutableStateOf(false) }
    var strategies by remember { mutableStateOf<List<CoreBridge.Strategy>>(emptyList()) }
    var selectedStrategyId by remember { mutableStateOf("general") }
    var nativeOk by remember { mutableStateOf<Boolean?>(null) }
    var logText by remember { mutableStateOf("[загрузка…]") }

    LaunchedEffect(Unit) {
        // extract assets + probe native lib off the UI thread
        val result = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            runCatching {
                val dir = AssetBootstrap.strategiesDir(context)
                val list = CoreBridge.loadStrategies(dir.absolutePath)
                list to CoreBridge.isNativeAvailable()
            }
        }
        result.fold(
            onSuccess = { (list, ok) ->
                strategies = list
                nativeOk = ok
                if (list.isNotEmpty() && list.none { it.id == selectedStrategyId }) {
                    selectedStrategyId = list.first().id
                }
                logText = buildString {
                    append(if (ok) "[core] native OK\n" else "[core] native missing\n")
                    append("[core] стратегий: ${list.size}")
                }
            },
            onFailure = { e ->
                nativeOk = false
                logText = "[core] ошибка: ${e.message}"
            },
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // Header
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            val dotColor = when {
                nativeOk == false -> Color(0xFFF44336)
                running -> Color(0xFF4CAF50)
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
                    nativeOk == false -> "Ошибка ядра"
                    running -> "Работает"
                    else -> "Остановлен"
                },
                color = Color(0xFFE0E0E0),
                fontSize = 20.sp,
            )
        }

        HorizontalDivider(color = Color(0xFF222222))

        // Strategy picker (simple cycle-through for skeleton)
        Text(
            text = "Стратегия (${strategies.size})",
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

        // Main toggle (visual only for now — VPN engine lands in M2)
        Button(
            onClick = { running = !running },
            modifier = Modifier
                .fillMaxWidth()
                .height(52.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = if (running) Color(0xFF2A2A2A) else Color(0xFFE0E0E0),
                contentColor = if (running) Color(0xFFF44336) else Color.Black,
            ),
        ) {
            Text(
                text = if (running) "Остановить" else "Запустить",
                fontSize = 16.sp,
            )
        }

        // Roblox toggle
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
                onCheckedChange = { robloxFix = it },
                colors = SwitchDefaults.colors(
                    checkedThumbColor = Color(0xFFE0E0E0),
                    checkedTrackColor = Color(0xFF444444),
                    uncheckedThumbColor = Color(0xFF666666),
                    uncheckedTrackColor = Color(0xFF222222),
                ),
            )
        }

        Spacer(modifier = Modifier.height(8.dp))

        // Log area
        Text(
            text = "Лог:\n$logText",
            color = Color(0xFF666666),
            fontSize = 12.sp,
        )
    }
}
