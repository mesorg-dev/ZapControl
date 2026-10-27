package com.mesorg.zapret.ui

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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun App() {
    var running by remember { mutableStateOf(false) }
    var robloxFix by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // Header
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            val dotColor = if (running) Color(0xFF4CAF50) else Color(0xFFF44336)
            Box(
                modifier = Modifier
                    .size(12.dp)
                    .clip(CircleShape)
                    .background(dotColor),
            )
            Text(
                text = if (running) "Работает" else "Остановлен",
                color = Color(0xFFE0E0E0),
                fontSize = 20.sp,
            )
        }

        HorizontalDivider(color = Color(0xFF222222))

        // Strategy placeholder
        Text(
            text = "Стратегия",
            color = Color(0xFF888888),
            fontSize = 13.sp,
        )
        Text(
            text = "general",
            color = Color(0xFFE0E0E0),
            fontSize = 16.sp,
        )

        HorizontalDivider(color = Color(0xFF222222))

        // Main toggle
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

        Spacer(modifier = Modifier.weight(1f))

        // Log area placeholder
        Text(
            text = "Лог:\n[скелет] VPN-движок ещё не подключён",
            color = Color(0xFF666666),
            fontSize = 12.sp,
        )
    }
}
