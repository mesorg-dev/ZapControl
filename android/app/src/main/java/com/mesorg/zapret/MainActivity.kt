package com.mesorg.zapret

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.ui.graphics.Color
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
                App()
            }
        }
    }
}
