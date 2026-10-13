package com.librespydus.app.core

import com.librespydus.app.feature.favourites.FavouritesMode
import com.librespydus.app.feature.home.HomeGreeting
import com.librespydus.app.feature.home.LoanShelfItem
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.librespydus_core.SpydusException

/**
 * Key screens take their words from the shared catalogue (design/copy.json → [Copy], docs §7.1),
 * so iOS and Android say the same thing. The literals here are the catalogue's wording, checked
 * once so a regenerated catalogue that changes a glossary term fails loudly.
 */
class CopyWordingTest {
    @Test
    fun tabsUseTheGlossaryNames() {
        assertEquals(
            listOf(Copy.Tabs.home, Copy.Tabs.yourBooks, Copy.Tabs.search, Copy.Tabs.myPicks, Copy.Tabs.profile),
            Tab.entries.map { it.label },
        )
        assertEquals(listOf("Home", "Your Books", "Search", "My Picks", "Profile"), Tab.entries.map { it.label })
    }

    @Test
    fun myPicksIsTheTitleSignedInAndOut() {
        val signedIn = FavouritesMode.of(signedIn = true)
        val signedOut = FavouritesMode.of(signedIn = false)
        assertEquals(Copy.Picks.title, signedIn.title)
        assertEquals(Copy.Picks.title, signedOut.title)
        assertEquals("My Picks", signedOut.title)
        assertEquals(Copy.Picks.subtitleAccount, signedIn.subtitle)
        assertEquals("Saved on your library account", signedIn.subtitle)
        assertEquals(Copy.Picks.subtitlePhone, signedOut.subtitle)
        assertEquals("Saved on this phone", signedOut.subtitle)
        assertEquals(Copy.Picks.emptyPhoneMessage, signedOut.emptyMessage)
        assertEquals(Copy.Picks.emptyAccountMessage, signedIn.emptyMessage)
    }

    @Test
    fun myPicksToastsAndCopySummary() {
        assertEquals("Saved to My Picks", Copy.Picks.saved)
        assertEquals("Removed from My Picks", Copy.Picks.removed)
        assertEquals("Couldn’t update My Picks", Copy.Picks.updateFailed)
        assertEquals(Copy.Picks.added(3) + Copy.Common.separator + Copy.Picks.notAdded(1), PicksText.copied(3, 1))
        assertEquals("Added 3 books to My Picks · 1 couldn’t be added", PicksText.copied(3, 1))
    }

    @Test
    fun homeSaysHelloAndCountsDownLoans() {
        assertEquals(Copy.Home.greeting("Test"), HomeGreeting.title("Test Borrower"))
        assertEquals(Copy.Home.greetingSignedOut, HomeGreeting.title(null))
        assertEquals("Your loans", Copy.Home.yourLoans)
        assertEquals(Copy.Loans.daysLeft(3), LoanShelfItem.daysText(3))
        assertEquals(Copy.Loans.daysLeft(1), LoanShelfItem.daysText(1))
        assertEquals(Copy.Loans.dueToday, LoanShelfItem.daysText(0))
        assertEquals(Copy.Loans.daysOverdue(1), LoanShelfItem.daysText(-1))
        assertEquals("2 days overdue", LoanShelfItem.daysText(-2))
    }

    @Test
    fun errorsUseTheCatalogueAndTheStandInName() {
        assertEquals(Copy.Errors.network("City of Canning"), UserFacingError.from(SpydusException.Network("timeout"), "City of Canning").message)
        assertEquals("Can’t reach your library. Check your connection.", UserFacingError.from(SpydusException.Network("timeout")).message)
        assertEquals("Your library asks you to confirm you’re human.", UserFacingError.from(SpydusException.HumanVerificationRequired("waf")).message)
        assertEquals(Copy.Errors.signInAgain, UserFacingError.from(SpydusException.NotLoggedIn("not logged in")).message)
        assertEquals(Copy.Errors.unexpected, UserFacingError.from(SpydusException.Parse("bad page")).message)
    }

    @Test
    fun notificationChannelsAreNamedFromTheCatalogue() {
        assertEquals(Copy.Alerts.readyChannel, AlertKind.READY.channelName)
        assertEquals(Copy.Alerts.dueChannel, AlertKind.DUE.channelName)
    }
}
