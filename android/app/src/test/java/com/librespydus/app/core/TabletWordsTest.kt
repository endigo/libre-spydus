package com.librespydus.app.core

import com.librespydus.app.isTabletDevice
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * A tablet says "tablet" where a phone says "phone" (docs/native-apps.md §6.1, §7.1): the words
 * are chosen at run time from [CopyDevice], which the app sets at launch from the device.
 */
class TabletWordsTest {
    @Test
    fun aPhoneIsTheDefault() {
        assertNull(CopyDevice.isTabletOverride)
        assertEquals(CopyDevice.isTablet, CopyDevice.usesTabletWords)
    }

    @Test
    fun theOverridePinsOneDevicesWordsAndIsRestored() {
        CopyDevice.withTabletWords(true) {
            assertTrue(CopyDevice.usesTabletWords)
            CopyDevice.withTabletWords(false) { assertFalse(CopyDevice.usesTabletWords) }
            assertTrue(CopyDevice.usesTabletWords)
        }
        assertNull(CopyDevice.isTabletOverride)
    }

    @Test
    fun aTabletSaysTabletWhereAPhoneSaysPhone() {
        CopyDevice.withTabletWords(false) {
            assertEquals("Saved on this phone", Copy.Picks.subtitlePhone)
            assertEquals("Tap the bookmark on a book to save it on this phone.", Copy.Picks.emptyPhoneMessage)
            assertEquals("Couldn’t load My Picks from this phone.", Copy.Picks.loadFailed)
            assertEquals("Couldn’t save My Picks on this phone.", Copy.Picks.saveFailed)
            assertEquals("Add this phone’s picks to your library account?", Copy.Picks.offerTitle)
            assertEquals("1 book saved on this phone can be added to My Picks on your library account.", Copy.Picks.offerMessage(1))
            assertEquals("3 books saved on this phone can be added to My Picks on your library account.", Copy.Picks.offerMessage(3))
            assertEquals("Verified on this phone", Copy.Profile.verified)
        }
        CopyDevice.withTabletWords(true) {
            assertEquals("Saved on this tablet", Copy.Picks.subtitlePhone)
            assertEquals("Tap the bookmark on a book to save it on this tablet.", Copy.Picks.emptyPhoneMessage)
            assertEquals("Couldn’t load My Picks from this tablet.", Copy.Picks.loadFailed)
            assertEquals("Couldn’t save My Picks on this tablet.", Copy.Picks.saveFailed)
            assertEquals("Add this tablet’s picks to your library account?", Copy.Picks.offerTitle)
            assertEquals("1 book saved on this tablet can be added to My Picks on your library account.", Copy.Picks.offerMessage(1))
            assertEquals("3 books saved on this tablet can be added to My Picks on your library account.", Copy.Picks.offerMessage(3))
            assertEquals("Verified on this tablet", Copy.Profile.verified)
        }
    }

    @Test
    fun theBiometricNotesNameTheDevice() {
        CopyDevice.withTabletWords(false) {
            assertEquals(
                "When City of Canning signs you out, sign back in with fingerprint instead of typing your PIN. " +
                    "Your card number and PIN stay on this phone, locked by fingerprint.",
                Copy.Biometrics.offerMessage("City of Canning", "fingerprint"),
            )
            assertEquals(
                "LibreSpydus signs in directly with City of Canning. It keeps your PIN on this phone only if you turn on fingerprint.",
                Copy.Login.privacyNoteBiometrics("City of Canning", "fingerprint"),
            )
        }
        CopyDevice.withTabletWords(true) {
            assertEquals(
                "When City of Canning signs you out, sign back in with fingerprint instead of typing your PIN. " +
                    "Your card number and PIN stay on this tablet, locked by fingerprint.",
                Copy.Biometrics.offerMessage("City of Canning", "fingerprint"),
            )
            assertEquals(
                "LibreSpydus signs in directly with City of Canning. It keeps your PIN on this tablet only if you turn on fingerprint.",
                Copy.Login.privacyNoteBiometrics("City of Canning", "fingerprint"),
            )
        }
    }

    @Test
    fun wordsWithoutATabletFormAreTheSameOnBoth() {
        CopyDevice.withTabletWords(true) {
            assertEquals("Saved on your library account", Copy.Picks.subtitleAccount)
            assertEquals("fingerprint", Copy.Biometrics.fingerprint)
        }
    }

    @Test
    fun aFoldableIsAPhoneOnEitherScreen() {
        // Pixel Tablet, Pixel 9.
        assertTrue(isTabletDevice(smallestWidthDp = 800, hasHinge = false))
        assertFalse(isTabletDevice(smallestWidthDp = 411, hasHinge = false))
        // A Galaxy Z Fold 8 opened (inner screen) and closed (cover screen).
        assertFalse(isTabletDevice(smallestWidthDp = 704, hasHinge = true))
        assertFalse(isTabletDevice(smallestWidthDp = 475, hasHinge = true))
    }
}
