package com.librespydus.app.ui.theme

import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * The wide layout's arithmetic (docs/native-apps.md §6.1), the same numbers as the iOS app's
 * IPadLayoutTests: the 600 × 480 dp breakpoint, card columns, and cover grids.
 */
class LayoutTest {
    @Test
    fun wideFromSixHundredBySixHundredAndEighty() {
        assertEquals(AppLayout.WIDE, AppLayout.of(600.dp, 480.dp))
        assertEquals(AppLayout.COMPACT, AppLayout.of(599.dp, 480.dp))
        assertEquals(AppLayout.COMPACT, AppLayout.of(600.dp, 479.dp))
    }

    @Test
    fun tabletsAreWideEitherWayRoundAndPhonesCompact() {
        // Pixel Tablet: 1280 × 800 dp landscape, 800 × 1280 portrait.
        assertEquals(AppLayout.WIDE, AppLayout.of(1280.dp, 800.dp))
        assertEquals(AppLayout.WIDE, AppLayout.of(800.dp, 1280.dp))
        // Pixel 9: 411 × 923 dp; landscape it is wide enough but too short.
        assertEquals(AppLayout.COMPACT, AppLayout.of(411.dp, 923.dp))
        assertEquals(AppLayout.COMPACT, AppLayout.of(923.dp, 411.dp))
        // A narrow split screen on a tablet, and a small freeform window.
        assertEquals(AppLayout.COMPACT, AppLayout.of(400.dp, 800.dp))
        assertEquals(AppLayout.COMPACT, AppLayout.of(700.dp, 400.dp))
    }

    @Test
    fun cardColumnsKeepCardsAtLeastThreeHundredWide() {
        assertEquals(1, PageLayout.cardColumns(0.dp))
        assertEquals(1, PageLayout.cardColumns(611.dp))
        assertEquals(2, PageLayout.cardColumns(612.dp))
        assertEquals(2, PageLayout.cardColumns(740.dp))
        assertEquals(2, PageLayout.cardColumns(923.dp))
        assertEquals(3, PageLayout.cardColumns(924.dp))
        assertEquals(3, PageLayout.cardColumns(1008.dp))
    }

    @Test
    fun cardColumnsStopAtThree() {
        assertEquals(3, PageLayout.cardColumns(2000.dp))
    }

    @Test
    fun coverGridsHaveAtLeastTwoColumns() {
        val narrow = PageLayout.coverGrid(200.dp)
        assertEquals(2, narrow.columns)
        assertEquals(92.dp, narrow.coverWidth)
    }

    @Test
    fun coverGridsFitAsManyOneTwentyCoversAsTheWidthAllows() {
        val four = PageLayout.coverGrid(528.dp)
        assertEquals(4, four.columns)
        assertEquals(120.dp, four.coverWidth)
        // One dp short of a fifth column: the four covers widen to fill the row.
        val wider = PageLayout.coverGrid(663.dp)
        assertEquals(4, wider.columns)
        assertEquals(153.dp, wider.coverWidth)
        assertEquals(5, PageLayout.coverGrid(664.dp).columns)
    }

    @Test
    fun coverGridsStopAtSixColumnsAndFillTheWidth() {
        val wide = PageLayout.coverGrid(2000.dp)
        assertEquals(6, wide.columns)
        assertEquals(320.dp, wide.coverWidth)
        assertEquals(2000.dp, wide.coverWidth * wide.columns + PageLayout.coverSpacing * (wide.columns - 1))
    }

    @Test
    fun theMeasurementsAreTheContractsNumbers() {
        assertEquals(1040.dp, PageLayout.contentMaxWidth)
        assertEquals(260.dp, PageLayout.sidebarWidth)
        assertEquals(300.dp, PageLayout.cardMinWidth)
        assertEquals(12.dp, PageLayout.cardSpacing)
        assertEquals(120.dp, PageLayout.coverMinWidth)
        assertEquals(16.dp, PageLayout.coverSpacing)
        assertEquals(440.dp, PageLayout.detailLeftWidth)
        assertEquals(820.dp, PageLayout.detailTwoColumnMinWidth)
        assertEquals(420.dp, PageLayout.borrowedSegmentsWidth)
        assertEquals(240.dp, PageLayout.searchSortWidth)
        assertEquals(760.dp, PageLayout.pickerColumnWidth)
        assertEquals(40.dp, PageLayout.sidebarLogoSize)
    }
}
