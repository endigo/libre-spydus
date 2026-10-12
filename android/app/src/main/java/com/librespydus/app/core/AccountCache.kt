package com.librespydus.app.core

import android.util.Log
import uniffi.librespydus_core.AccountSnapshot
import uniffi.librespydus_core.decodeAccountSnapshot
import uniffi.librespydus_core.encodeAccountSnapshot
import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

/**
 * Saved account snapshots (§5.2a), one per key: the library origin, or a separate key for the
 * DEBUG demo option that saves demo data. Never synced or backed up. Blocking: call off the main
 * thread.
 */
interface AccountSnapshotStorage {
    /** The saved snapshot, or null. An unreadable one is deleted and reads as absent. */
    fun load(key: String): AccountSnapshot?

    fun save(key: String, snapshot: AccountSnapshot)

    fun clear(key: String)

    /** Drops every library's snapshot (debug `ls_reset`). */
    fun clearAll()
}

/**
 * [AccountSnapshotStorage] as files in [dir] (the app's `noBackupFilesDir/account-cache`), named
 * `<sha256(key)>.json`: the core's `encodeAccountSnapshot` JSON sealed by [sealer] (AES-GCM under
 * the Keystore key that also protects the session), with the key bound in so a file can't be
 * moved to another library. Writes go to a temporary file first and are then moved into place,
 * so a crash never leaves half a file. A file that doesn't decrypt, isn't valid JSON or was
 * written with another `ACCOUNT_SNAPSHOT_VERSION` (`decodeAccountSnapshot` throws `Parse`) is
 * deleted; the next refresh writes a new one.
 */
class FileAccountSnapshotStorage(
    private val dir: File,
    private val sealer: AesGcmSealer,
) : AccountSnapshotStorage {
    @Synchronized
    override fun load(key: String): AccountSnapshot? {
        val file = file(key)
        if (!file.exists()) return null
        return try {
            decodeAccountSnapshot(String(sealer.open(associated(key), file.readBytes())))
        } catch (e: Exception) {
            Log.w(TAG, "dropping an unreadable account snapshot", e)
            file.delete()
            null
        }
    }

    @Synchronized
    override fun save(key: String, snapshot: AccountSnapshot) {
        dir.mkdirs()
        val sealed = sealer.seal(associated(key), encodeAccountSnapshot(snapshot).toByteArray())
        val tmp = File.createTempFile("snapshot", ".tmp", dir)
        try {
            tmp.writeBytes(sealed)
            Files.move(tmp.toPath(), file(key).toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
        } finally {
            tmp.delete()
        }
    }

    @Synchronized
    override fun clear(key: String) {
        file(key).delete()
    }

    @Synchronized
    override fun clearAll() {
        dir.listFiles()?.forEach { it.delete() }
    }

    internal fun file(key: String): File = File(dir, "${sha256(key)}.json")

    private fun associated(key: String) = "account-snapshot:$key"

    companion object {
        const val DIR_NAME = "account-cache"
        private const val TAG = "AccountCache"

        fun sha256(text: String): String =
            MessageDigest.getInstance("SHA-256").digest(text.toByteArray()).joinToString("") { "%02x".format(it) }
    }
}

/** In-memory [AccountSnapshotStorage] for tests. */
class InMemoryAccountSnapshotStorage : AccountSnapshotStorage {
    private val values = mutableMapOf<String, AccountSnapshot>()

    @Synchronized override fun load(key: String) = values[key]

    @Synchronized override fun save(key: String, snapshot: AccountSnapshot) {
        values[key] = snapshot
    }

    @Synchronized override fun clear(key: String) {
        values.remove(key)
    }

    @Synchronized override fun clearAll() = values.clear()

    @Synchronized fun keys(): Set<String> = values.keys.toSet()
}
