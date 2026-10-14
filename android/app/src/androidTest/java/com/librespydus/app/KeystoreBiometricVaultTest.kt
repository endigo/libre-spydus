package com.librespydus.app

import android.util.Log
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.librespydus.app.core.BiometricRead
import com.librespydus.app.core.KeystoreBiometricVault
import com.librespydus.app.core.SignInDetails
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.security.KeyStore

/**
 * The real Keystore vault (§5.14) on a device with a fingerprint enrolled: saving and reading each
 * go through the prompt with the cipher as a CryptoObject. Someone (or the host, with
 * `adb emu finger touch 1` in a loop) has to touch the sensor at each prompt. Uses its own key and
 * library, and deletes both.
 */
@RunWith(AndroidJUnit4::class)
class KeystoreBiometricVaultTest {
    @Test
    fun savesAndReadsTheDetailsBehindTheCheck() = runBlocking {
        val app = ApplicationProvider.getApplicationContext<LibreSpydusApplication>()
        ActivityScenario.launch(MainActivity::class.java).use {
            val prompter = app.biometricPrompter
            withTimeout(20_000) { combine(prompter.resumed, prompter.focused) { r, f -> r && f }.first { it } }
            assertTrue("a strong biometric must be enrolled", prompter.isAvailable())
            val vault = KeystoreBiometricVault(app, prompter, keyAlias = KEY)
            try {
                assertNull(vault.username(ORIGIN))
                Log.i(TAG, "touch the sensor: saving")
                assertTrue(vault.save(ORIGIN, SignInDetails("reader", "4321"), "Test Library"))
                assertEquals("reader", vault.username(ORIGIN))
                val stored = File(app.noBackupFilesDir, KeystoreBiometricVault.DIR_NAME).listFiles().orEmpty()
                assertFalse("the PIN is never written in plain", stored.any { it.readText().contains("4321") })

                Log.i(TAG, "touch the sensor: reading")
                assertEquals(BiometricRead.Unlocked(SignInDetails("reader", "4321")), vault.read(ORIGIN, "Test Library"))
            } finally {
                vault.delete(ORIGIN)
                KeyStore.getInstance("AndroidKeyStore").apply { load(null) }.deleteEntry(KEY)
            }
            assertNull(vault.username(ORIGIN))
        }
    }

    private companion object {
        const val TAG = "KeystoreVaultTest"
        const val KEY = "librespydus_signin_test"
        const val ORIGIN = "https://vault-test.invalid"
    }
}
