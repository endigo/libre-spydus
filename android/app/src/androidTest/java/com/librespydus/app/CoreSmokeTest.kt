package com.librespydus.app

import android.system.Os
import android.system.OsConstants
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.librespydus_core.SpydusClient

/**
 * On-device smoke test of the real core: loads liblibrespydus_core.so through JNA, runs an async
 * UniFFI call on the coroutine bridge and does a TLS public GET (search only — no session, no POST).
 */
@RunWith(AndroidJUnit4::class)
class CoreSmokeTest {
    @Test
    fun canningSearchReturnsRecords() = runBlocking {
        Log.i(TAG, "page size: ${Os.sysconf(OsConstants._SC_PAGESIZE)}")
        SpydusClient("https://canning.spydus.com").use { client ->
            assertEquals("https://canning.spydus.com", client.origin())
            val results = client.search("harry potter")
            Log.i(TAG, "total=${results.totalResults} page=${results.records.size} first=${results.records.firstOrNull()?.title}")
            assertTrue("expected records for 'harry potter'", results.records.isNotEmpty())
            assertTrue(results.records.any { it.title?.contains("Harry Potter", ignoreCase = true) == true })
        }
    }

    private companion object {
        const val TAG = "CoreSmokeTest"
    }
}
