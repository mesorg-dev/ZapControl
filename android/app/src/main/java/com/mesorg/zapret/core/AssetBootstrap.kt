package com.mesorg.zapret.core

import android.content.Context
import java.io.File

/**
 * Copies bundled assets (strategies/, utils/targets.txt) to app files dir
 * on first launch so Rust-side code can read them via normal file paths.
 */
object AssetBootstrap {

    /** Returns dir containing *.txt strategy templates. */
    fun strategiesDir(context: Context): File = File(context.filesDir, "strategies").apply {
        copyAssetDir(context, "strategies", this)
    }

    /** Returns path to targets.txt (extracted from assets). */
    fun targetsFile(context: Context): File = File(context.filesDir, "targets.txt").apply {
        if (!exists()) {
            context.assets.open("utils/targets.txt").use { input ->
                outputStream().use { output -> input.copyTo(output) }
            }
        }
    }

    private fun copyAssetDir(context: Context, assetPath: String, dest: File) {
        if (dest.exists() && dest.list()?.isNotEmpty() == true) return
        dest.mkdirs()
        val children = context.assets.list(assetPath) ?: return
        for (child in children) {
            val childAsset = "$assetPath/$child"
            val childDest = File(dest, child)
            val subChildren = context.assets.list(childAsset)
            if (subChildren != null && subChildren.isNotEmpty()) {
                copyAssetDir(context, childAsset, childDest)
            } else {
                context.assets.open(childAsset).use { input ->
                    childDest.outputStream().use { output -> input.copyTo(output) }
                }
            }
        }
    }
}
