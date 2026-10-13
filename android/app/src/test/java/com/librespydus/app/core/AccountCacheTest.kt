package com.librespydus.app.core

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import kotlin.experimental.xor

/** The saved-account files: per origin, sealed, and dropped when unreadable. */
class AccountCacheTest {
    @get:Rule val tmp = TemporaryFolder()

    private val canning = "https://canning.spydus.com"
    private val hutt = "https://hutt.spydus.co.nz"

    // A plain AES key stands in for the Keystore one on the JVM.
    private val key: SecretKey = newKey()
    private val dir get() = File(tmp.root, FileAccountSnapshotStorage.DIR_NAME)

    private fun newKey(): SecretKey = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()

    private fun storage(sealKey: SecretKey = key) = FileAccountSnapshotStorage(dir, AesGcmSealer { sealKey })

    @Test
    fun savesLoadsAndClearsPerOrigin() {
        val s = storage()
        assertNull(s.load(canning))
        s.save(canning, accountSnapshot("Canning Reader"))
        s.save(hutt, accountSnapshot("Hutt Reader"))
        assertEquals(accountSnapshot("Canning Reader"), s.load(canning))
        assertEquals("Hutt Reader", s.load(hutt)?.accountName)
        // A cold start (a new instance) reads the same files.
        assertEquals(accountSnapshot("Canning Reader"), storage().load(canning))

        s.save(canning, accountSnapshot("Canning Reader", fetchedAtMs = 2_000))
        assertEquals(2_000L, s.load(canning)?.fetchedAtMs)

        s.clear(canning)
        assertNull(s.load(canning))
        assertEquals("Hutt Reader", s.load(hutt)?.accountName)
        s.clearAll()
        assertNull(s.load(hutt))
    }

    @Test
    fun filesAreNamedByTheOriginsHashAndSealed() {
        val s = storage()
        s.save(canning, accountSnapshot())
        val file = File(dir, "e20e5e3f49866bfbb2a5eeaa2e2a33ca4ee95304db1f2738eca4584b1bc229c6.json")
        assertEquals(file, s.file(canning))
        assertEquals(listOf(file.name), dir.list()!!.toList()) // no temporary file left behind
        val raw = String(file.readBytes(), Charsets.ISO_8859_1)
        for (plain in listOf("Project Hail Mary", "Test Borrower", "fetchedAtMs", "{\"version\"")) assertFalse(plain, raw.contains(plain))
    }

    @Test
    fun aSnapshotFromAnotherVersionIsDiscarded() {
        val s = storage()
        s.save(canning, accountSnapshot().copy(version = DemoBackend.SNAPSHOT_VERSION + 1u))
        assertTrue(s.file(canning).exists())
        assertNull(s.load(canning))
        assertFalse(s.file(canning).exists())
    }

    @Test
    fun corruptFilesAreDiscarded() {
        val s = storage()
        val file = s.file(canning)
        dir.mkdirs()

        file.writeBytes(byteArrayOf(1, 2, 3))
        assertNull(s.load(canning))
        assertFalse(file.exists())

        // Sealed correctly, but not a snapshot.
        file.writeBytes(AesGcmSealer { key }.seal("account-snapshot:$canning", "{\"version\":1,".toByteArray()))
        assertNull(s.load(canning))
        assertFalse(file.exists())

        // Tampered with.
        s.save(canning, accountSnapshot())
        val bytes = file.readBytes()
        bytes[bytes.size - 1] = bytes.last() xor 1
        file.writeBytes(bytes)
        assertNull(s.load(canning))
        assertFalse(file.exists())
    }

    @Test
    fun aFileOnlyOpensForItsOwnOriginAndKey() {
        val s = storage()
        s.save(canning, accountSnapshot())
        s.file(canning).copyTo(s.file(hutt))
        assertNull(s.load(hutt)) // moved to another library: the origin is sealed in
        assertNull(storage(sealKey = newKey()).load(canning)) // the Keystore key was replaced
        assertNull(s.load(canning))
    }

    @Test
    fun theSealerRoundTripsAndChecksTheAssociatedData() {
        val sealer = AesGcmSealer { key }
        val sealed = sealer.seal("a", "secret".toByteArray())
        assertArrayEquals("secret".toByteArray(), sealer.open("a", sealed))
        // A fresh IV each time.
        assertFalse(sealed.contentEquals(sealer.seal("a", "secret".toByteArray())))
        assertTrue(runCatching { sealer.open("b", sealed) }.isFailure)
        assertTrue(runCatching { sealer.open("a", sealed.copyOf(10)) }.isFailure)
    }
}
