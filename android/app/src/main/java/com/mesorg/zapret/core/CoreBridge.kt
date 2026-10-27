package com.mesorg.zapret.core

/**
 * Placeholder for JNI/UniFFI bridge to zapret-core-ffi (Rust cdylib).
 *
 * When M2 lands:
 * 1. Build crates/zapret-core-ffi for aarch64-linux-android via cargo-ndk.
 * 2. Generate Kotlin bindings:
 *      cargo run -p zapret-core-ffi --bin uniffi-bindgen \
 *        generate --language kotlin --out-dir app/src/main/java \
 *        --source crates/zapret-core-ffi/src/lib.rs
 * 3. System.loadLibrary("zapret_core_ffi") and call generated functions.
 *
 * Exported FFI surface (JSON-in/JSON-out):
 *  - loadStrategiesJson(dir): String
 *  - loadTargetsJson(path): String
 *  - fetchRobloxJson(url): suspend -> String
 *  - buildRobloxHosts(fileJson): String
 *  - pickBestStrategy(summaryJson): String?
 *  - loadSettingsJson(path): String
 *  - saveSettingsJson(path, json): Unit
 */
object CoreBridge {
    // Intentionally empty for the skeleton.
}
