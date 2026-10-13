package com.librespydus.app.feature.mycard

import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.DemoData
import com.librespydus.app.core.FakeBackend
import com.librespydus.app.core.LibraryBackend
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.BarcodeSymbology
import java.io.File
import java.time.LocalDate

/** My card's pure parts (§5.9): what the sheet shows, the barcode's pixel layout, the symbology. */
class MyCardLogicTest {
    // region Content

    @Test
    fun noNumberAsksForOne() {
        assertEquals(CardContent.Entry, MyCardLogic.content(null, BarcodeSymbology.CODE128))
        assertEquals(CardContent.Entry, MyCardLogic.content("   ", null))
    }

    @Test
    fun aSpinnerUntilTheSymbologyIsKnown() {
        assertEquals(CardContent.Loading("CL114D0001234"), MyCardLogic.content(" CL114D0001234 ", null))
    }

    @Test
    fun theBarcodeIsInTheLibrarysSymbology() {
        val code128 = MyCardLogic.content("CL114D0001234", BarcodeSymbology.CODE128) as CardContent.Barcode
        assertEquals(BarcodeSymbology.CODE128, code128.barcode.symbology)
        assertEquals("CL114D0001234", code128.barcode.text)
        assertTrue(code128.barcode.modules.first() && code128.barcode.modules.last()) // bars at both ends, no quiet zones

        val code39 = MyCardLogic.content("cl114d0001234", BarcodeSymbology.CODE39) as CardContent.Barcode
        assertEquals(BarcodeSymbology.CODE39, code39.barcode.symbology)
        assertEquals("CL114D0001234", code39.barcode.text) // Code 39 prints upper case
        assertEquals("cl114d0001234", code39.number) // the number as saved
    }

    @Test
    fun aNumberTheSymbologyCantEncodeFallsBackToCode128() {
        val codabar = MyCardLogic.content("CL114D0001234", BarcodeSymbology.CODABAR) as CardContent.Barcode
        assertEquals(BarcodeSymbology.CODE128, codabar.barcode.symbology)
    }

    @Test
    fun aNonAsciiNumberCantBeShownAsABarcode() {
        assertEquals(CardContent.Unencodable("CL114Ä"), MyCardLogic.content("CL114Ä", BarcodeSymbology.CODE128))
        // Known at once: no spinner while the symbology loads.
        assertEquals(CardContent.Unencodable("CL114Ä"), MyCardLogic.content("CL114Ä", null))
    }

    @Test
    fun theBarcodeIsOneTalkBackElementNamingTheNumber() {
        assertEquals("Barcode for card DEMO0012345", MyCardLogic.barcodeDescription("DEMO0012345"))
    }

    // endregion
    // region Layout

    @Test
    fun modulesAreAWholeNumberOfPixelsWithQuietZonesAndCentred() {
        // 100 modules + 20 quiet modules in 1000 px → 8 px each (960 px), 800 px of bars.
        val layout = BarcodeLayout.of(moduleCount = 100, availablePx = 1000)
        assertEquals(8, layout.modulePx)
        assertEquals(800, layout.widthPx)
        assertEquals(100, layout.startPx)
        assertEquals(layout.startPx, 1000 - layout.startPx - layout.widthPx) // centred
        assertTrue(layout.startPx >= 10 * layout.modulePx)
    }

    @Test
    fun theModuleWidthRoundsDownAndOddLeftoversStillCentre() {
        val layout = BarcodeLayout.of(moduleCount = 156, availablePx = 953) // 953 / 176 = 5.41
        assertEquals(5, layout.modulePx)
        assertEquals(780, layout.widthPx)
        assertEquals(86, layout.startPx) // (953 - 780) / 2
        val right = 953 - layout.startPx - layout.widthPx
        assertTrue(right - layout.startPx in 0..1)
        assertTrue(layout.startPx >= 10 * 5 && right >= 10 * 5)
    }

    @Test
    fun exactlyEnoughRoomGivesExactlyTenQuietModules() {
        val layout = BarcodeLayout.of(moduleCount = 80, availablePx = 300) // 100 modules × 3 px
        assertEquals(3, layout.modulePx)
        assertEquals(30, layout.startPx)
        assertEquals(30, 300 - layout.startPx - layout.widthPx)
    }

    @Test
    fun tooNarrowStillDrawsOnePixelPerModule() {
        val layout = BarcodeLayout.of(moduleCount = 200, availablePx = 150)
        assertEquals(1, layout.modulePx)
        assertEquals(200, layout.widthPx)
        assertEquals(-25, layout.startPx) // centred, overflowing both sides equally
    }

    @Test
    fun barsMergeAdjacentBlackModulesAtWholePixels() {
        val modules = listOf(true, true, false, true, false, false, true, true, true)
        assertEquals(listOf(0..1, 3..3, 6..8), BarcodeGeometry.runs(modules))
        val layout = BarcodeLayout(modulePx = 4, startPx = 40, widthPx = 36)
        assertEquals(listOf(40 until 48, 52 until 56, 64 until 76), layout.bars(modules))
        assertEquals(emptyList<IntRange>(), BarcodeGeometry.runs(listOf(false, false)))
    }

    @Test
    fun aRealBarcodeFitsAPhoneWidth() {
        val barcode = (MyCardLogic.content("DEMO0012345", BarcodeSymbology.CODE128) as CardContent.Barcode).barcode
        val layout = BarcodeLayout.of(barcode.modules.size, availablePx = 953) // Pixel 9 card width
        assertTrue(layout.modulePx >= 4)
        val bars = layout.bars(barcode.modules)
        assertEquals(layout.startPx, bars.first().first)
        assertEquals(layout.startPx + layout.widthPx - 1, bars.last().last)
        assertTrue(bars.all { (it.last - it.first + 1) % layout.modulePx == 0 && (it.first - layout.startPx) % layout.modulePx == 0 })
    }

    // endregion
    // region Symbology

    private class SymbologyBackend(origin: String, var symbology: BarcodeSymbology) : LibraryBackend by FakeBackend(origin) {
        var calls = 0

        override suspend fun cardBarcodeSymbology(): BarcodeSymbology {
            calls++
            return symbology
        }
    }

    @Test
    fun theLibrarysSymbologyIsPassedThroughAndRememberedPerLibrary() = runTest {
        val cache = CardSymbologies()
        val cumberland = SymbologyBackend("https://cumberlandlibraries.spydus.com", BarcodeSymbology.CODE39)
        assertEquals(null, cache.cached(cumberland.origin)) // first open: spinner
        assertEquals(BarcodeSymbology.CODE39, cache.load(cumberland))
        assertEquals(BarcodeSymbology.CODE39, cache.cached(cumberland.origin)) // later opens: at once

        val canning = SymbologyBackend("https://canning.spydus.com", BarcodeSymbology.CODE128)
        assertEquals(BarcodeSymbology.CODE128, cache.load(canning))
        assertEquals(BarcodeSymbology.CODE39, cache.cached(cumberland.origin))

        val shown = MyCardLogic.content("CL114D0001234", cache.cached(cumberland.origin)) as CardContent.Barcode
        assertEquals(BarcodeSymbology.CODE39, shown.barcode.symbology)
    }

    @Test
    fun eachOpenAsksAgainSoAFallbackGetsCorrected() = runTest {
        val cache = CardSymbologies()
        // The library couldn't be reached: the core fell back to Code 128 (and didn't cache that).
        val cumberland = SymbologyBackend("https://cumberlandlibraries.spydus.com", BarcodeSymbology.CODE128)
        assertEquals(BarcodeSymbology.CODE128, cache.load(cumberland))
        cumberland.symbology = BarcodeSymbology.CODE39
        assertEquals(BarcodeSymbology.CODE39, cache.load(cumberland))
        assertEquals(BarcodeSymbology.CODE39, cache.cached(cumberland.origin))
        assertEquals(2, cumberland.calls)
    }

    @Test
    fun aBackendWithoutAConfigUsesCode128() = runTest {
        assertEquals(BarcodeSymbology.CODE128, FakeBackend().cardBarcodeSymbology())
    }

    @Test
    fun demoModeAsksTheRealLibrary() = runTest {
        val public = SymbologyBackend("https://cumberlandlibraries.spydus.com", BarcodeSymbology.CODE39)
        val data = DemoData.load({ File("src/main/assets/demo/$it").readText() }, public.origin, LocalDate.of(2026, 10, 13))
        val demo = DemoBackend(public, data)
        assertEquals(BarcodeSymbology.CODE39, demo.cardBarcodeSymbology())
        assertEquals(1, public.calls)
    }

    // endregion
    // region Brightness

    @Test
    fun brightnessGoesToFullAndBackToTheWindowsOwnSetting() {
        var level = -1f // BRIGHTNESS_OVERRIDE_NONE
        val boost = BrightnessBoost(read = { level }, write = { level = it })
        boost.raise()
        assertEquals(1f, level)
        assertTrue(boost.isRaised)
        boost.raise() // again (back in the foreground): still remembers -1, not 1
        boost.restore()
        assertEquals(-1f, level)
        assertFalse(boost.isRaised)
        level = 0.4f
        boost.restore() // nothing to restore
        assertEquals(0.4f, level)
    }

    // endregion
}
