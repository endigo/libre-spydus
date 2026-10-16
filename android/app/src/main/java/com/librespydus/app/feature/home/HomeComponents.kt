package com.librespydus.app.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Formatting
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.feature.search.SkeletonBlock
import com.librespydus.app.feature.search.browseIcon
import com.librespydus.app.feature.search.browseTitle
import com.librespydus.app.ui.components.CoverImage
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** One cover on a Home shelf. */
data class HomeShelfEntry(
    val ref: BookRef,
    val caption: String?,
    val captionColor: Color = LsColors.textSecondary,
    /** Read after the title and author (loans: "3 days left"). */
    val accessibilityCaption: String? = null,
)

/** A horizontal row of covers; tap → book detail. */
@Composable
fun HomeCoverShelf(entries: List<HomeShelfEntry>, onOpen: (BookRef) -> Unit) {
    LazyRow(
        contentPadding = PaddingValues(horizontal = LsSpacing.l),
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        items(entries, key = { it.ref.id }) { entry ->
            val label = listOfNotNull(entry.ref.title, Formatting.byLine(entry.ref.author), entry.accessibilityCaption).joinToString(", ")
            Column(
                Modifier
                    .width(LsCover.shelfWidth)
                    .clip(RoundedCornerShape(LsRadius.cover))
                    .clickable { onOpen(entry.ref) }
                    .clearAndSetSemantics {
                        contentDescription = label
                        role = Role.Button
                        onClick { onOpen(entry.ref); true }
                    },
                verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
            ) {
                CoverImage(url = entry.ref.coverUrl, title = entry.ref.title, width = LsCover.shelfWidth)
                if (entry.caption != null) {
                    Text(entry.caption, style = LsType.caption, color = entry.captionColor, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}

/** A shelf of cover-shaped skeletons. */
@Composable
fun HomeShelfSkeleton() {
    Row(
        Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState(), enabled = false)
            .padding(horizontal = LsSpacing.l)
            .clearAndSetSemantics { contentDescription = Copy.Common.loading },
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        repeat(4) {
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
                SkeletonBlock(Modifier.width(LsCover.shelfWidth).aspectRatio(LsCover.ASPECT), radius = LsRadius.cover)
                SkeletonBlock(Modifier.width(LsCover.shelfWidth * 0.7f).height(10.dp))
            }
        }
    }
}

/** A section's inline failure: what went wrong and Retry, without taking over the screen. */
@Composable
fun HomeSectionErrorCard(error: UserFacingError, onRetry: () -> Unit) {
    HomeCard {
        Icon(error.browseIcon, contentDescription = null, tint = LsColors.warning, modifier = Modifier.size(24.dp))
        Column(
            Modifier
                .weight(1f)
                .semantics(mergeDescendants = true) {},
            verticalArrangement = Arrangement.spacedBy(LsSpacing.xs),
        ) {
            Text(error.browseTitle, style = LsType.bodyStrong, color = LsColors.textPrimary)
            Text(error.message, style = LsType.caption, color = LsColors.textSecondary)
        }
        if (error.isRetryable) {
            Text(
                Copy.Common.retry,
                style = LsType.bodyStrong,
                color = LsColors.primary,
                modifier = Modifier
                    .clip(RoundedCornerShape(LsRadius.pill))
                    .clickable(role = Role.Button, onClick = onRetry)
                    .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
            )
        }
    }
}

/** A section's empty state: an icon and a line or two. */
@Composable
fun HomeSectionMessageCard(icon: ImageVector, title: String, message: String?) {
    HomeCard(Modifier.semantics(mergeDescendants = true) {}) {
        Icon(icon, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(24.dp))
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
            Text(title, style = LsType.bodyStrong, color = LsColors.textPrimary)
            if (message != null) Text(message, style = LsType.caption, color = LsColors.textSecondary)
        }
    }
}

/** Signed out, in place of "Your loans". */
@Composable
fun HomeSignInCard(libraryName: String, onSignIn: () -> Unit) {
    HomeCard {
        Box(
            Modifier
                .size(44.dp)
                .background(LsColors.accent, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(LsIcons.Library, contentDescription = null, tint = LsColors.onAccent, modifier = Modifier.size(22.dp))
        }
        Column(
            Modifier
                .weight(1f)
                .semantics(mergeDescendants = true) {},
            verticalArrangement = Arrangement.spacedBy(LsSpacing.xs),
        ) {
            Text(Copy.Home.signInCardTitle, style = LsType.bodyStrong, color = LsColors.textPrimary)
            Text(Copy.Home.signInCardMessage(libraryName), style = LsType.caption, color = LsColors.textSecondary)
        }
        Box(
            Modifier
                .clip(RoundedCornerShape(LsRadius.pill))
                .background(LsColors.primaryGradient)
                .clickable(role = Role.Button, onClick = onSignIn)
                .padding(horizontal = LsSpacing.l, vertical = LsSpacing.s),
        ) {
            Text(Copy.Account.signIn, style = LsType.bodyStrong, color = LsColors.onPrimary, maxLines = 1)
        }
    }
}

/** Home's section card on `surface`. */
@Composable
private fun HomeCard(modifier: Modifier = Modifier, content: @Composable RowScope.() -> Unit) {
    Row(
        modifier
            .padding(horizontal = LsSpacing.l)
            .fillMaxWidth()
            .clip(RoundedCornerShape(LsRadius.card))
            .background(LsColors.surface)
            .padding(LsSpacing.l),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
        content = content,
    )
}

/** The library's logo on a white rounded square (its initial until the image loads, or without one). */
@Composable
fun HomeLibraryLogo(name: String, logoUrl: String?, onClick: () -> Unit, modifier: Modifier = Modifier, size: Dp = 48.dp) {
    val shape = RoundedCornerShape(12.dp)
    var loaded by remember(logoUrl) { mutableStateOf(false) }
    Box(
        modifier
            .size(size)
            .shadow(6.dp, shape)
            .clip(shape)
            .background(Color.White)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = Copy.A11y.labelAndHint(Copy.Home.profileButton(name), Copy.Home.profileButtonHint) },
        contentAlignment = Alignment.Center,
    ) {
        if (!loaded) {
            Text(
                name.trim().firstOrNull()?.uppercase() ?: "?",
                color = LsColors.background,
                fontSize = 22.sp,
                fontWeight = FontWeight.SemiBold,
            )
        }
        if (!logoUrl.isNullOrBlank()) {
            AsyncImage(
                model = logoUrl,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                onSuccess = { loaded = true },
                onError = { loaded = false },
                modifier = Modifier
                    .padding(5.dp)
                    .graphicsLayer { alpha = if (loaded) 1f else 0f },
            )
        }
    }
}

/** A Material icon for a genre tile. */
fun HomeGenreIcon.vector(): ImageVector = when (this) {
    HomeGenreIcon.PERSON -> LsIcons.PersonFilled
    HomeGenreIcon.STORIES -> BrowseIcons.AutoStories
    HomeGenreIcon.SCIENCE -> LsIcons.Science
    HomeGenreIcon.MUSEUM -> LsIcons.Museum
    HomeGenreIcon.WAND -> BrowseIcons.AutoFixHigh
    HomeGenreIcon.FINGERPRINT -> BrowseIcons.Fingerprint
    HomeGenreIcon.DINING -> LsIcons.Restaurant
    HomeGenreIcon.CHILD -> BrowseIcons.ChildCare
    HomeGenreIcon.BOOKS -> BrowseIcons.MenuBook
}
