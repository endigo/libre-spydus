package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsShadow
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/**
 * A book cover (§7): async image (Syndetics etc.) over a generated placeholder (the title on a
 * token colour). The placeholder stays visible while loading, on failure, and for the 1×1 "no
 * image" GIFs cover services return. Rounded corners + soft shadow; cover aspect from tokens.
 *
 * [width] defaults to the shelf width; pass [Dp.Unspecified] to size it from [modifier].
 */
@Composable
fun CoverImage(
    url: String?,
    title: String,
    modifier: Modifier = Modifier,
    width: Dp = LsCover.shelfWidth,
    cornerRadius: Dp = LsRadius.cover,
    shadow: Boolean = true,
) {
    val shape = RoundedCornerShape(cornerRadius)
    var loaded by remember(url) { mutableStateOf(false) }
    Box(
        modifier
            .then(if (width != Dp.Unspecified) Modifier.width(width) else Modifier)
            .aspectRatio(LsCover.ASPECT)
            .then(
                if (shadow) Modifier.shadow(LsShadow.card / 2, shape, ambientColor = LsShadow.cardColor, spotColor = LsShadow.cardColor)
                else Modifier,
            )
            .clip(shape)
            .semantics { contentDescription = Copy.A11y.coverOf(title) },
    ) {
        if (!loaded) CoverPlaceholder(title, Modifier.matchParentSize())
        if (!url.isNullOrBlank()) {
            AsyncImage(
                model = url,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                onSuccess = { state -> loaded = state.result.image.width > 2 && state.result.image.height > 2 },
                onError = { loaded = false },
                modifier = Modifier
                    .matchParentSize()
                    .graphicsLayer { alpha = if (loaded) 1f else 0f },
            )
        }
    }
}

/** Generated cover: [title] on a palette colour chosen from the title, with a spine shade. */
@Composable
fun CoverPlaceholder(title: String, modifier: Modifier = Modifier) {
    val base = LsColors.genreColor(title)
    BoxWithConstraints(
        modifier.background(Brush.linearGradient(listOf(base, base.copy(alpha = 0.78f).compositeOverNavy()))),
    ) {
        val small = maxWidth < 90.dp
        Box(
            Modifier
                .fillMaxHeight()
                .width(maxWidth * 0.06f)
                .background(Color.Black.copy(alpha = 0.18f)),
        )
        Text(
            text = title.ifBlank { "?" },
            color = Color.White,
            style = if (small) LsType.caption.copy(fontWeight = FontWeight.SemiBold, fontSize = 10.sp, lineHeight = 13.sp)
            else LsType.bodyStrong,
            textAlign = TextAlign.Center,
            maxLines = if (small) 4 else 5,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier
                .align(Alignment.Center)
                .padding(horizontal = if (small) 6.dp else LsSpacing.m),
        )
    }
}

private fun Color.compositeOverNavy(): Color {
    val a = alpha
    val n = LsColors.background
    return Color(red * a + n.red * (1 - a), green * a + n.green * (1 - a), blue * a + n.blue * (1 - a))
}

/**
 * A cover with an optional caption beneath (Home shelves, "Your loans" with
 * "3 days left", carousels). [showTitle] adds the title above the caption.
 */
@Composable
fun ShelfCover(
    title: String,
    coverUrl: String?,
    modifier: Modifier = Modifier,
    width: Dp = LsCover.shelfWidth,
    caption: String? = null,
    captionColor: Color = LsColors.textSecondary,
    showTitle: Boolean = false,
    titleColor: Color = LsColors.textPrimary,
    onClick: (() -> Unit)? = null,
) {
    Column(
        modifier
            .width(width)
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        CoverImage(url = coverUrl, title = title, width = width)
        if (showTitle) {
            Text(
                title,
                style = LsType.caption.copy(fontWeight = FontWeight.SemiBold),
                color = titleColor,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        if (caption != null) {
            Text(
                caption,
                style = LsType.caption,
                color = captionColor,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = if (showTitle) TextAlign.Start else TextAlign.Center,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
