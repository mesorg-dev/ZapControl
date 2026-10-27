package com.mesorg.zapret.core

import uniffi.zapret_core_ffi.FfiException
import uniffi.zapret_core_ffi.buildRobloxHosts
import uniffi.zapret_core_ffi.fetchRobloxJson
import uniffi.zapret_core_ffi.loadSettingsJson
import uniffi.zapret_core_ffi.loadStrategiesJson
import uniffi.zapret_core_ffi.loadTargetsJson
import uniffi.zapret_core_ffi.pickBestStrategy
import uniffi.zapret_core_ffi.saveSettingsJson

/**
 * Thin Kotlin-side wrapper over zapret-core-ffi (Rust cdylib via UniFFI/JNA).
 *
 * The native library `libzapret_core_ffi.so` must be present in jniLibs
 * (built by cargo-ndk in CI). Loading is lazy on first FFI call.
 */
object CoreBridge {

    data class Strategy(val id: String, val name: String)

    data class Target(
        val name: String,
        val url: String?,
        val ping: String?,
    )

    /** Load strategy list from a directory of *.txt templates. */
    fun loadStrategies(dir: String): List<Strategy> {
        val json = loadStrategiesJson(dir)
        return parseStrategyArray(json)
    }

    /** Load test targets (targets.txt path, or defaults when missing). */
    fun loadTargets(path: String): List<Target> {
        val json = loadTargetsJson(path)
        return parseTargetArray(json)
    }

    /** Fetch roblox-ips.json, return body of DNS/hosts entries or throw FfiException. */
    suspend fun fetchRobloxHosts(url: String): String {
        val fileJson = fetchRobloxJson(url)
        return buildRobloxHosts(fileJson)
    }

    /** Best strategy id from a TestSummary JSON, or null. */
    fun pickBest(summaryJson: String): String? = pickBestStrategy(summaryJson)

    fun loadSettings(path: String): String = loadSettingsJson(path)

    fun saveSettings(path: String, json: String) = saveSettingsJson(path, json)

    /** Exposed so UI can catch UnsatisfiedLinkError early if .so is missing. */
    fun isNativeAvailable(): Boolean = try {
        loadStrategiesJson("")
        true
    } catch (_: UnsatisfiedLinkError) {
        false
    } catch (_: NoClassDefFoundError) {
        false
    }

    // --- minimal JSON parsing (no kotlinx-serialization dependency) ---

    private fun parseStrategyArray(json: String): List<Strategy> {
        // [{"id":"general","name":"General","argsTemplate":"..."}]
        if (json.isBlank() || json == "[]") return emptyList()
        val out = mutableListOf<Strategy>()
        // split on top-level objects crudely but safely for our controlled format
        val objs = json.removePrefix("[").removeSuffix("]").split("},{")
        for (o in objs) {
            val s = o.trim().removePrefix("{").removeSuffix("}")
            val id = extractString(s, "id") ?: continue
            val name = extractString(s, "name") ?: id
            out.add(Strategy(id, name))
        }
        return out
    }

    private fun parseTargetArray(json: String): List<Target> {
        if (json.isBlank() || json == "[]") return emptyList()
        val out = mutableListOf<Target>()
        val objs = json.removePrefix("[").removeSuffix("]").split("},{")
        for (o in objs) {
            val s = o.trim().removePrefix("{").removeSuffix("}")
            val name = extractString(s, "name") ?: continue
            out.add(
                Target(
                    name = name,
                    url = extractString(s, "url"),
                    ping = extractString(s, "ping"),
                ),
            )
        }
        return out
    }

    /** Extract "key":"value" (value may be null) from a flat JSON object body. */
    private fun extractString(obj: String, key: String): String? {
        val needle = "\"$key\":"
        val i = obj.indexOf(needle)
        if (i < 0) return null
        var v = obj.substring(i + needle.length).trimStart()
        if (v.startsWith("null")) return null
        if (!v.startsWith("\"")) return null
        v = v.substring(1)
        val end = findClosingQuote(v) ?: return null
        return unescape(v.substring(0, end))
    }

    private fun findClosingQuote(s: String): Int? {
        var i = 0
        while (i < s.length) {
            when (s[i]) {
                '\\' -> i += 2
                '"' -> return i
                else -> i++
            }
        }
        return null
    }

    private fun unescape(s: String): String {
        val sb = StringBuilder(s.length)
        var i = 0
        while (i < s.length) {
            val c = s[i]
            if (c == '\\' && i + 1 < s.length) {
                when (s[i + 1]) {
                    'n' -> sb.append('\n')
                    't' -> sb.append('\t')
                    'r' -> sb.append('\r')
                    'u' -> {
                        if (i + 5 < s.length) {
                            s.substring(i + 2, i + 6).toIntOrNull(16)?.let { sb.append(it.toChar()) }
                            i += 6
                            continue
                        }
                    }
                    else -> sb.append(s[i + 1])
                }
                i += 2
            } else {
                sb.append(c)
                i++
            }
        }
        return sb.toString()
    }
}
