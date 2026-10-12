package com.librespydus.app.core

import android.content.Intent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LaunchOptionsTest {
    @Test
    fun parsesTheDocumentedExample() {
        val options = LaunchOptions.parse(
            mapOf("ls_origin" to "https://canning.spydus.com", "ls_demo" to true, "ls_route" to "borrowed"),
        )
        assertEquals(LaunchOptions(origin = "https://canning.spydus.com", route = LaunchRoute.Borrowed, demo = true), options)
    }

    @Test
    fun originIsTrimmedAndBlankIgnored() {
        assertEquals("https://hutt.spydus.co.nz", LaunchOptions.parse(mapOf("ls_origin" to " https://hutt.spydus.co.nz/ ")).origin)
        assertNull(LaunchOptions.parse(mapOf("ls_origin" to "  ")).origin)
    }

    @Test
    fun booleansAcceptStringsAndNumbers() {
        assertTrue(LaunchOptions.parse(mapOf("ls_demo" to "true")).demo)
        assertTrue(LaunchOptions.parse(mapOf("ls_reset" to "1")).reset)
        assertTrue(LaunchOptions.parse(mapOf("ls_reset" to 1)).reset)
        assertFalse(LaunchOptions.parse(mapOf("ls_demo" to "no")).demo)
        assertFalse(LaunchOptions.parse(emptyMap()).demo)
        assertTrue(LaunchOptions.parse(emptyMap()).isEmpty)
    }

    @Test
    fun parsesTheSavedAccountDemoFlags() {
        val options = LaunchOptions.parse(mapOf("ls_demo" to true, "ls_demo_persist" to true, "ls_demo_refresh_fails" to "1"))
        assertTrue(options.demoPersist)
        assertTrue(options.demoRefreshFails)
        val plain = LaunchOptions.parse(mapOf("ls_demo" to true))
        assertFalse(plain.demoPersist)
        assertFalse(plain.demoRefreshFails)
    }

    @Test
    fun parsesEveryRoute() {
        val cases = mapOf(
            "home" to LaunchRoute.Home,
            "search:harry potter" to LaunchRoute.Search("harry potter"),
            "search" to LaunchRoute.Search(""),
            "results:new fiction" to LaunchRoute.Results("new fiction"),
            "detail:https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=457540" to
                LaunchRoute.Detail("https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=457540"),
            "borrowed" to LaunchRoute.Borrowed,
            "favourites" to LaunchRoute.Favourites,
            "profile" to LaunchRoute.Profile,
            "libraries" to LaunchRoute.Libraries,
            "login" to LaunchRoute.Login,
            "verify" to LaunchRoute.Verify,
            "gallery" to LaunchRoute.Gallery,
            " Profile " to LaunchRoute.Profile,
        )
        for ((raw, expected) in cases) assertEquals(raw, expected, LaunchRoute.parse(raw))
    }

    @Test
    fun unknownOrIncompleteRoutesAreNull() {
        assertNull(LaunchRoute.parse("settings"))
        assertNull(LaunchRoute.parse("detail:"))
        assertNull(LaunchRoute.parse("results"))
        assertNull(LaunchOptions.parse(mapOf("ls_route" to "nope")).route)
    }

    @Test
    fun releaseBuildsIgnoreTheIntent() {
        assertTrue(LaunchOptions.fromIntent(Intent(), debug = false).isEmpty)
        assertTrue(LaunchOptions.fromIntent(null, debug = true).isEmpty)
    }
}
