package com.qurb

import android.content.Context
import android.util.Log
import java.io.File

/**
 * What the app's own storage is made of, folder by folder, written to the
 * log (the owner, 2026-11-14: "why are phone apps taking so much space?").
 * Android shows one figure; this says where it goes, two levels down.
 */
object Space {
    private const val TAG = "QurbSpace"

    private fun size(file: File): Long =
        if (file.isFile) file.length() else file.listFiles()?.sumOf { size(it) } ?: 0L

    fun log(context: Context) {
        val places = listOfNotNull(context.dataDir, context.getExternalFilesDir(null), context.externalCacheDir)
        for (place in places.distinct()) {
            Log.i(TAG, "space ${place.path}: ${size(place) / 1_000_000} MB")
            walk(place, place, 1)
        }
    }

    /** Each folder of a megabyte or more, four levels down, largest first. */
    private fun walk(top: File, dir: File, depth: Int) {
        if (depth > 4) return
        for (child in dir.listFiles().orEmpty().map { it to size(it) }.sortedByDescending { it.second }.take(10)) {
            if (child.second < 1_000_000) continue
            // The app's own files by name; the person's (their Vault, what
            // they opened) as "a file", its name being theirs, not the log's.
            val path = child.first.relativeTo(top).path
            val shown = if (child.first.isDirectory || "/.qurb/" in "/$path") path else "${child.first.parentFile?.relativeTo(top)?.path}/(a file)"
            Log.i(TAG, "space ${"  ".repeat(depth)}$shown: ${child.second / 1_000_000} MB")
            if (child.first.isDirectory) walk(top, child.first, depth + 1)
        }
    }
}
