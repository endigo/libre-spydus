package com.librespydus.app.feature.favourites

import android.app.Activity
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.Copy
import com.librespydus.app.core.DemoBackend
import com.librespydus.app.core.Formatting
import com.librespydus.app.feature.borrowed.BorrowedDebugHooks
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.ShelfCover
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LocalAppLayout
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsTabBar
import com.librespydus.app.ui.theme.LsType
import com.librespydus.app.ui.theme.PageLayout
import com.librespydus.app.ui.theme.pageColumn
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** Favourites' layout maths (unit-tested). */
object FavouritesLayout {
    /** Two columns across the screen's gutters (16dp each side and between). */
    fun columnWidth(width: Dp): Dp = maxOf(80.dp, Dp(((width - LsSpacing.l * 3) / 2).value.toInt().toFloat()))
}

/**
 * The My Picks tab: signed in, the reader's Pick list on the library account ([MyPicksScreen],
 * §5.11); signed out, this phone's own favourites ([LocalFavouritesScreen], §5.3).
 */
@Composable
fun FavouritesScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val signedIn by appState.isSignedIn.collectAsStateWithLifecycle()
    val library by appState.library.collectAsStateWithLifecycle()
    val activity = LocalActivity.current
    LaunchedEffect(library?.origin) { FavouritesDebugHooks.seed(activity, appState) }
    when (FavouritesMode.of(signedIn)) {
        FavouritesMode.MY_PICKS -> MyPicksScreen(appState, navigator, modifier)
        FavouritesMode.LOCAL -> LocalFavouritesScreen(appState, navigator, modifier)
    }
}

/**
 * Favourites signed out (docs/native-apps.md §6): this library's saved books, newest first, as a
 * two-column cover grid. Tap → detail; remove from the long-press menu or in Edit mode. Favourites
 * are local and per library, so switching library shows that library's list.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun LocalFavouritesScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val favourites by appState.currentFavourites.collectAsStateWithLifecycle()
    val activity = LocalActivity.current
    val scope = rememberCoroutineScope()
    val view = LocalView.current
    val gridState = rememberLazyGridState()
    var editing by rememberSaveable { mutableStateOf(FavouritesDebugHooks.startsEditing(activity)) }

    LaunchedEffect(favourites.isEmpty()) { if (favourites.isEmpty()) editing = false }
    LaunchedEffect(favourites.size) {
        if (favourites.isNotEmpty() && FavouritesDebugHooks.scrollsToBottom(activity)) {
            delay(800)
            gridState.scrollToItem(favourites.size)
            gridState.scrollBy(100_000f)
        }
    }

    fun remove(book: BookRef) {
        scope.launch { appState.removeFavourite(book.id) }
        @Suppress("DEPRECATION")
        view.announceForAccessibility(Copy.Picks.removedTitle(book.title))
    }

    BoxWithConstraints(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding()
            .pageColumn(),
    ) {
        // The wide layout (§6.1): as many columns as keep covers 120 dp wide, up to six.
        val coverGrid = if (LocalAppLayout.current.isWide) PageLayout.coverGrid(maxWidth - LsSpacing.l * 2) else null
        val columnWidth = coverGrid?.coverWidth ?: FavouritesLayout.columnWidth(maxWidth)
        LazyVerticalGrid(
            columns = GridCells.Fixed(coverGrid?.columns ?: 2),
            state = gridState,
            contentPadding = PaddingValues(
                start = LsSpacing.l,
                end = LsSpacing.l,
                // Clear the raised Search button at max scroll.
                bottom = LsSpacing.xxl,
            ),
            horizontalArrangement = Arrangement.spacedBy(LsSpacing.l),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
            modifier = Modifier.fillMaxSize(),
        ) {
            item(key = "header", span = { GridItemSpan(maxLineSpan) }) {
                Header(
                    count = favourites.size,
                    editing = editing,
                    onToggleEditing = { editing = !editing },
                )
            }
            if (favourites.isEmpty()) {
                item(key = "empty", span = { GridItemSpan(maxLineSpan) }) {
                    EmptyStateView(
                        title = Copy.Picks.emptyTitle,
                        message = FavouritesMode.LOCAL.emptyMessage,
                        icon = LsIcons.Bookmark,
                        actionLabel = Copy.Picks.findABook,
                        onAction = { appState.openSearch() },
                        modifier = Modifier.fillMaxWidth().heightIn(min = 420.dp),
                    )
                }
            } else {
                items(favourites, key = { it.id }) { book ->
                    FavouriteCell(
                        book = book,
                        width = columnWidth,
                        editing = editing,
                        onOpen = { navigator.openBook(book) },
                        onRemove = { remove(book) },
                        modifier = Modifier.animateItem(),
                    )
                }
            }
        }
    }
}

@Composable
private fun Header(count: Int, editing: Boolean, onToggleEditing: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(top = LsSpacing.l), verticalAlignment = Alignment.Top) {
        Column(Modifier.weight(1f)) {
            Text(FavouritesMode.LOCAL.title, style = LsType.greeting, color = LsColors.textPrimary, modifier = Modifier.semantics { heading() })
            Text(FavouritesMode.LOCAL.subtitle, style = LsType.caption, color = LsColors.textSecondary)
        }
        if (count > 0) {
            val label = if (editing) Copy.Common.done else Copy.Common.edit
            val hint = if (editing) Copy.Picks.doneEditingHint else Copy.Picks.editHint
            Text(
                label,
                style = LsType.bodyStrong,
                color = LsColors.primary,
                modifier = Modifier
                    .padding(top = LsSpacing.xs)
                    .clip(RoundedCornerShape(LsRadius.pill))
                    .clickable(role = Role.Button, onClick = onToggleEditing)
                    .semantics { contentDescription = Copy.A11y.labelAndHint(label, hint) }
                    .padding(horizontal = LsSpacing.s, vertical = LsSpacing.s),
            )
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun FavouriteCell(
    book: BookRef,
    width: Dp,
    editing: Boolean,
    onOpen: () -> Unit,
    onRemove: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var menu by remember { mutableStateOf(false) }
    val haptics = LocalHapticFeedback.current
    Box(modifier) {
        Box(
            Modifier
                .clip(RoundedCornerShape(LsRadius.cover))
                .combinedClickable(
                    enabled = !editing,
                    role = Role.Button,
                    onClickLabel = Copy.Common.open,
                    onLongClickLabel = Copy.Common.moreOptions,
                    onLongClick = {
                        haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                        menu = true
                    },
                    onClick = onOpen,
                )
                .semantics {
                    customActions = listOf(CustomAccessibilityAction(Copy.Picks.remove) { onRemove(); true })
                },
        ) {
            ShelfCover(
                title = book.title,
                coverUrl = book.coverUrl,
                width = width,
                caption = Formatting.displayAuthor(book.author),
                showTitle = true,
            )
        }
        if (editing) {
            Box(
                Modifier
                    .align(Alignment.TopEnd)
                    .offset(x = 10.dp, y = (-10).dp)
                    .size(44.dp)
                    .clip(CircleShape)
                    .clickable(role = Role.Button, onClick = onRemove)
                    .semantics { contentDescription = Copy.Picks.removeTitle(book.title) },
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    Modifier
                        .size(30.dp)
                        .clip(CircleShape)
                        .background(LsColors.danger)
                        .border(2.dp, LsColors.background, CircleShape),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(AccountIcons.Remove, contentDescription = null, tint = LsColors.onPrimary, modifier = Modifier.size(18.dp))
                }
            }
        }
        DropdownMenu(
            expanded = menu,
            onDismissRequest = { menu = false },
            containerColor = LsColors.surfaceElevated,
        ) {
            DropdownMenuItem(
                text = { Text(Copy.Common.open, color = LsColors.textPrimary) },
                leadingIcon = { Icon(LsIcons.Book, contentDescription = null, tint = LsColors.textSecondary) },
                onClick = {
                    menu = false
                    onOpen()
                },
            )
            DropdownMenuItem(
                text = { Text(Copy.Picks.remove, color = LsColors.danger) },
                leadingIcon = { Icon(AccountIcons.BookmarkRemove, contentDescription = null, tint = LsColors.danger) },
                onClick = {
                    menu = false
                    onRemove()
                },
            )
        }
    }
}

/** DEBUG screenshot hooks for Favourites (see `BorrowedDebugHooks` for the extras). */
internal object FavouritesDebugHooks {
    fun startsEditing(activity: Activity?): Boolean =
        BorrowedDebugHooks.flag(activity, BorrowedDebugHooks.FAVOURITES_EDITING)

    fun scrollsToBottom(activity: Activity?): Boolean =
        BorrowedDebugHooks.flag(activity, BorrowedDebugHooks.SCROLL_BOTTOM)

    /**
     * `--ez ls_seed_favourites true` in demo mode: saves the demo account's books in this phone's
     * favourites (local only, signed in or not: signed in, they are what the My Picks copy offers).
     */
    suspend fun seed(activity: Activity?, appState: AppState) {
        if (!BorrowedDebugHooks.flag(activity, BorrowedDebugHooks.SEED_FAVOURITES)) return
        val backend = appState.backend.value?.takeIf { it.isDemo } ?: return
        if (appState.currentFavourites.value.isNotEmpty()) return
        val loans = backend.loans(DemoBackend.SESSION).items
        val reservations = backend.reservations(DemoBackend.SESSION).let { it.ready.items + it.pending.items }
        for (item in (reservations + loans).reversed()) {
            val ref = BookRef.from(item)
            if (!appState.isFavourite(ref.id)) {
                appState.toggleFavourite(ref)
                delay(5) // distinct savedAt, so the order is stable
            }
        }
    }
}
