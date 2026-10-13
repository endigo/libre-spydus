package com.librespydus.app.feature.mycard

import com.librespydus.app.core.LibraryBackend
import uniffi.librespydus_core.BarcodeSymbology
import uniffi.librespydus_core.CardBarcode
import uniffi.librespydus_core.cardBarcode
import java.util.concurrent.ConcurrentHashMap

/** What the My card sheet shows (docs/native-apps.md §5.9). */
sealed interface CardContent {
    /** No number saved: the entry form. */
    data object Entry : CardContent

    /** The library's symbology isn't known yet (first open per library): a spinner. */
    data class Loading(val number: String) : CardContent

    data class Barcode(val number: String, val barcode: CardBarcode) : CardContent

    /** Not even Code 128 can encode [number] (non-ASCII): the number large, no bars. */
    data class Unencodable(val number: String) : CardContent
}

object MyCardLogic {
    /**
     * What to show for [number] once [symbology] is known (null while it loads). A number Code 128
     * can't encode is [CardContent.Unencodable] at once, since `cardBarcode` falls back to Code 128
     * whatever the library's symbology.
     */
    fun content(
        number: String?,
        symbology: BarcodeSymbology?,
        encode: (String, BarcodeSymbology) -> CardBarcode? = ::cardBarcode,
    ): CardContent {
        val trimmed = number?.trim()?.takeIf { it.isNotEmpty() } ?: return CardContent.Entry
        if (symbology == null) {
            return if (encode(trimmed, BarcodeSymbology.CODE128) == null) CardContent.Unencodable(trimmed) else CardContent.Loading(trimmed)
        }
        val barcode = encode(trimmed, symbology) ?: return CardContent.Unencodable(trimmed)
        return CardContent.Barcode(trimmed, barcode)
    }

    /** TalkBack reads the barcode as one element. */
    fun barcodeDescription(number: String) = "Barcode for card $number"

    fun symbologyName(symbology: BarcodeSymbology) = when (symbology) {
        BarcodeSymbology.CODE128 -> "Code 128"
        BarcodeSymbology.CODE39 -> "Code 39"
        BarcodeSymbology.CODABAR -> "Codabar"
    }
}

/**
 * Where a barcode's modules go in a row of [availablePx] device pixels: every module is the same
 * whole number of pixels ([modulePx], at least 1) so a scanner sees crisp, even bars, with at
 * least [QUIET_ZONE_MODULES] white modules each side and the bars centred.
 */
data class BarcodeLayout(val modulePx: Int, val startPx: Int, val widthPx: Int) {
    /** The black bars as pixel ranges (adjacent black modules merged into one bar). */
    fun bars(modules: List<Boolean>): List<IntRange> = BarcodeGeometry.runs(modules).map { run ->
        val left = startPx + run.first * modulePx
        left until left + (run.last - run.first + 1) * modulePx
    }

    companion object {
        const val QUIET_ZONE_MODULES = 10

        fun of(moduleCount: Int, availablePx: Int): BarcodeLayout {
            val module = maxOf(1, availablePx / (moduleCount + 2 * QUIET_ZONE_MODULES))
            val width = module * moduleCount
            return BarcodeLayout(module, (availablePx - width) / 2, width)
        }
    }
}

internal object BarcodeGeometry {
    /** Runs of black modules as module-index ranges, left to right. */
    fun runs(modules: List<Boolean>): List<IntRange> {
        val runs = mutableListOf<IntRange>()
        var start = -1
        modules.forEachIndexed { i, black ->
            if (black && start < 0) start = i
            if (!black && start >= 0) {
                runs += start until i
                start = -1
            }
        }
        if (start >= 0) runs += start until modules.size
        return runs
    }
}

/**
 * Each library's last known card symbology while the app runs, so the sheet shows a spinner only
 * on its first open per library. Every open still asks again ([load]): the core answers from its
 * own cache (10 min), and a Code 128 fallback given while the library couldn't be reached gets
 * corrected. The core never fails the call: Code 128 when the library has no config.
 */
class CardSymbologies {
    private val known = ConcurrentHashMap<String, BarcodeSymbology>()

    fun cached(origin: String): BarcodeSymbology? = known[origin]

    suspend fun load(backend: LibraryBackend): BarcodeSymbology =
        backend.cardBarcodeSymbology().also { known[backend.origin] = it }

    companion object {
        /** The app's one cache (process lifetime). */
        val shared = CardSymbologies()
    }
}

/**
 * Full screen brightness while the card is up (§5.9): [raise] remembers the window's own setting
 * and goes to full; [restore] (sheet closed, app left the foreground) puts that setting back.
 * Repeated calls are harmless.
 */
class BrightnessBoost(private val read: () -> Float, private val write: (Float) -> Unit) {
    private var saved: Float? = null

    val isRaised: Boolean get() = saved != null

    fun raise() {
        if (saved != null) return
        saved = read()
        write(FULL)
    }

    fun restore() {
        val previous = saved ?: return
        saved = null
        write(previous)
    }

    companion object {
        /** `WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_FULL`. */
        const val FULL = 1f
    }
}
