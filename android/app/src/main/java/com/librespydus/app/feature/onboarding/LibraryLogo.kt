package com.librespydus.app.feature.onboarding

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.util.LruCache
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.imageLoader
import coil3.request.ImageRequest
import coil3.request.SuccessResult
import coil3.request.allowHardware
import coil3.toBitmap
import com.librespydus.app.ui.theme.LsColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import kotlinx.coroutines.withContext
import uniffi.librespydus_core.LibraryInfo

/**
 * A library's directory logo on a white rounded tile, or a monogram on a token colour while it
 * loads and when there's none (most directory entries have no logo). Hidden from accessibility:
 * the row next to it names the library.
 */
@Composable
fun LibraryLogo(name: String, logoUrl: String?, modifier: Modifier = Modifier, size: Dp = 44.dp) {
    val context = LocalContext.current
    val pixels = with(LocalDensity.current) { (size * 3).roundToPx() }
    var logo by remember(logoUrl) { mutableStateOf(logoUrl?.let(LibraryLogoLoader::cached)) }
    LaunchedEffect(logoUrl) {
        if (logo == null && !logoUrl.isNullOrBlank()) logo = LibraryLogoLoader.load(context, logoUrl, pixels)
    }
    val shape = RoundedCornerShape(size * 0.26f)
    Box(
        modifier
            .size(size)
            .clip(shape)
            .border(0.5.dp, Color.White.copy(alpha = 0.08f), shape)
            .clearAndSetSemantics {},
        contentAlignment = Alignment.Center,
    ) {
        val loaded = logo
        if (loaded != null) {
            Box(Modifier.fillMaxSize().background(if (loaded.lightOnClear) LsColors.background else Color.White))
            Image(
                loaded.bitmap,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize().padding(size * 0.1f),
            )
        } else {
            val tint = LsColors.genreColor(name)
            Box(Modifier.fillMaxSize().background(Brush.linearGradient(listOf(tint.copy(alpha = 0.8f), tint))))
            Text(
                LibraryPickerLogic.monogram(name),
                color = Color.White,
                fontWeight = FontWeight.Bold,
                fontSize = with(LocalDensity.current) { (size * 0.38f).toSp() },
                textAlign = TextAlign.Center,
                maxLines = 1,
            )
        }
    }
}

@Composable
fun LibraryLogo(library: LibraryInfo, modifier: Modifier = Modifier, size: Dp = 44.dp) =
    LibraryLogo(library.name, library.logoUrl, modifier, size)

/** A decoded logo and whether it needs a navy tile ([LogoTone]). */
class LoadedLogo(val bitmap: ImageBitmap, val lightOnClear: Boolean)

/**
 * Logos come from each library's own host, so no per-host limit bounds them: at most 4 logo
 * downloads run at once (docs/native-apps.md §2: ≤ 6 concurrent). A row scrolled away before its
 * turn gives up its place (its coroutine is cancelled while waiting).
 */
object LibraryLogoLoader {
    private val permits = Semaphore(4)
    private val cache = LruCache<String, LoadedLogo>(96)

    fun cached(url: String): LoadedLogo? = cache.get(url)

    suspend fun load(context: Context, url: String, pixels: Int): LoadedLogo? {
        cached(url)?.let { return it }
        return permits.withPermit {
            cached(url) ?: run {
                val request = ImageRequest.Builder(context)
                    .data(url)
                    .size(pixels)
                    // Software bitmaps, so the tone check can read pixels.
                    .allowHardware(false)
                    .build()
                val result = context.imageLoader.execute(request) as? SuccessResult ?: return@run null
                withContext(Dispatchers.Default) {
                    // Some logos decode to configs Compose and Bitmap.createScaledBitmap can't use
                    // (no colour space): redraw into plain ARGB_8888. A logo that still fails
                    // keeps its monogram.
                    runCatching {
                        val bitmap = LogoTone.normalized(result.image.toBitmap())
                        LoadedLogo(bitmap.asImageBitmap(), LogoTone.isLightOnClear(bitmap))
                    }.getOrNull()
                }?.also { cache.put(url, it) }
            }
        }
    }
}

/**
 * Some logos are white on a transparent background (City of Fremantle) and vanish on the white
 * tile; those go on navy instead. Decided once per image from a 16×16 thumbnail.
 */
object LogoTone {
    private const val SIDE = 16

    fun isLightOnClear(bitmap: Bitmap): Boolean {
        val thumb = redraw(bitmap, SIDE, SIDE)
        val pixels = IntArray(SIDE * SIDE)
        thumb.getPixels(pixels, 0, SIDE, 0, 0, SIDE, SIDE)
        thumb.recycle()
        return measure(pixels)
    }

    /** [bitmap] as a software ARGB_8888 bitmap (itself when it already is one). */
    fun normalized(bitmap: Bitmap): Bitmap =
        if (bitmap.config == Bitmap.Config.ARGB_8888 && bitmap.colorSpace != null) bitmap
        else redraw(bitmap, bitmap.width.coerceAtLeast(1), bitmap.height.coerceAtLeast(1))

    private fun redraw(bitmap: Bitmap, width: Int, height: Int): Bitmap {
        val target = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        Canvas(target).drawBitmap(bitmap, null, Rect(0, 0, width, height), Paint(Paint.FILTER_BITMAP_FLAG))
        return target
    }

    /**
     * [argb] are non-premultiplied ARGB pixels. Light (mean Rec. 709 luminance of the visible
     * pixels, weighted by alpha, > 0.85) and at least a quarter transparent.
     */
    fun measure(argb: IntArray): Boolean {
        if (argb.isEmpty()) return false
        var luminance = 0.0
        var coverage = 0.0
        var clear = 0
        for (pixel in argb) {
            val alpha = ((pixel ushr 24) and 0xFF) / 255.0
            if (alpha < 0.1) clear++
            if (alpha <= 0) continue
            val red = ((pixel shr 16) and 0xFF) / 255.0
            val green = ((pixel shr 8) and 0xFF) / 255.0
            val blue = (pixel and 0xFF) / 255.0
            luminance += (0.2126 * red + 0.7152 * green + 0.0722 * blue) * alpha
            coverage += alpha
        }
        if (coverage <= 0) return false
        return clear * 4 >= argb.size && luminance / coverage > 0.85
    }
}
