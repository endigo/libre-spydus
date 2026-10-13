package com.librespydus.app.feature.featured

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.librespydus.app.ui.components.SectionHeader
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import uniffi.librespydus_core.FeaturedKind
import uniffi.librespydus_core.FeaturedLink

/**
 * Home's "Featured" section (§5.10): a horizontal row of tiles, one per link of the library's
 * Featured list, each opening its collection, results list or web page. Draws nothing without links.
 */
@Composable
fun FeaturedSection(links: List<FeaturedLink>, navigator: AppNavigator, modifier: Modifier = Modifier) {
    if (links.isEmpty()) return
    Column(modifier, verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        SectionHeader("Featured", modifier = Modifier.padding(horizontal = LsSpacing.l))
        LazyRow(
            contentPadding = PaddingValues(horizontal = LsSpacing.l),
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
        ) {
            itemsIndexed(links, key = { index, link -> "$index:${link.url}" }) { index, link ->
                FeaturedTile(
                    link = link,
                    color = LsColors.genre[index % LsColors.genre.size],
                    onClick = { navigator.openFeatured(link) },
                )
            }
        }
    }
}

/** A rounded `surface` card: the link's icon on a tinted circle, its title (two lines at most) under it. */
@Composable
fun FeaturedTile(link: FeaturedLink, color: Color, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(LsRadius.card)
    // The title, then the library's tooltip or what the tap does.
    val hint = link.description?.takeIf { it.isNotBlank() && it != link.title }
        ?: if (link.kind == FeaturedKind.WEB) "Opens the library's website" else "Shows this collection"
    Column(
        modifier
            .width(TileWidth)
            .clip(shape)
            .background(LsColors.surface)
            .clickable(role = Role.Button, onClick = onClick)
            .clearAndSetSemantics {
                contentDescription = "${link.title}. $hint"
                role = Role.Button
                onClick { onClick(); true }
            }
            .padding(LsSpacing.m),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        Box(
            Modifier
                .size(IconDiameter)
                .background(color.copy(alpha = 0.18f), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(FeaturedIcon.forName(link.icon).vector(), contentDescription = null, tint = color, modifier = Modifier.size(22.dp))
        }
        // Always two lines tall, so every tile in the row is the same height.
        Text(
            link.title,
            style = LsType.caption.copy(fontWeight = FontWeight.SemiBold),
            color = LsColors.textPrimary,
            minLines = 2,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

private val TileWidth = 128.dp
private val IconDiameter = 44.dp
