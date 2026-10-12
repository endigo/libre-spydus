package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.Tab
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsShadow
import com.librespydus.app.ui.theme.LsTabBar
import com.librespydus.app.ui.theme.LsType

/** Height the tab bar occupies at the bottom of the screen (bar + navigation bar inset). */
@Composable
fun tabBarContentPadding(): Dp = LsTabBar.height + WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

/**
 * The custom tab bar (§5.5, §7): white bar with Home · Borrowed · [Search] · Favourites ·
 * Profile, the Search button a raised white circle. The composable is taller than the bar by
 * `centerButtonLift`; that strip is transparent so content shows around the circle.
 */
@Composable
fun LsTabBar(
    selected: Tab,
    onSelect: (Tab) -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(modifier.fillMaxWidth()) {
        Column(
            Modifier
                .padding(top = LsTabBar.centerButtonLift)
                .fillMaxWidth()
                .shadow(16.dp, RectangleShape, ambientColor = LsShadow.tabCenterColor, spotColor = LsShadow.tabCenterColor)
                .background(LsColors.tabBar),
        ) {
            Row(Modifier.fillMaxWidth().height(LsTabBar.height)) {
                Tab.entries.forEach { tab ->
                    TabSlot(
                        tab = tab,
                        selected = tab == selected,
                        onClick = { onSelect(tab) },
                        modifier = Modifier.weight(1f),
                    )
                }
            }
            Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars))
        }
        CenterSearchButton(
            selected = selected == Tab.SEARCH,
            onClick = { onSelect(Tab.SEARCH) },
            modifier = Modifier.align(Alignment.TopCenter),
        )
    }
}

@Composable
private fun TabSlot(tab: Tab, selected: Boolean, onClick: () -> Unit, modifier: Modifier) {
    val color = if (selected) LsColors.tabActive else LsColors.tabInactive
    val interaction = remember { MutableInteractionSource() }
    Column(
        modifier
            .fillMaxHeight()
            .clickable(interactionSource = interaction, indication = null, role = Role.Tab, onClick = onClick)
            .semantics {
                this.selected = selected
                contentDescription = tab.label
            }
            .padding(bottom = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Bottom,
    ) {
        if (tab != Tab.SEARCH) {
            Icon(tab.icon(selected), contentDescription = null, tint = color, modifier = Modifier.size(24.dp))
            Spacer(Modifier.height(4.dp))
        }
        Text(tab.label, style = LsType.tab, color = color, maxLines = 1)
    }
}

@Composable
private fun CenterSearchButton(selected: Boolean, onClick: () -> Unit, modifier: Modifier) {
    val d = LsTabBar.centerButtonDiameter
    Box(
        modifier
            .size(d)
            .shadow(LsShadow.tabCenter, CircleShape, ambientColor = LsShadow.tabCenterColor, spotColor = Color.Black.copy(alpha = 0.3f))
            .clip(CircleShape)
            .background(LsColors.tabBar)
            .clickable(role = Role.Tab, onClick = onClick)
            .semantics {
                this.selected = selected
                contentDescription = Tab.SEARCH.label
            },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier
                .size(d - 12.dp)
                .background(if (selected) LsColors.background else LsColors.chipOnSheet, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                LsIcons.Search,
                contentDescription = null,
                tint = if (selected) LsColors.textPrimary else LsColors.tabActive,
                modifier = Modifier.size(26.dp),
            )
        }
    }
}

private fun Tab.icon(selected: Boolean): ImageVector = when (this) {
    Tab.HOME -> if (selected) LsIcons.HomeFilled else LsIcons.Home
    Tab.BORROWED -> if (selected) LsIcons.BookFilled else LsIcons.Book
    Tab.SEARCH -> LsIcons.Search
    Tab.FAVOURITES -> if (selected) LsIcons.BookmarkFilled else LsIcons.Bookmark
    Tab.PROFILE -> if (selected) LsIcons.PersonFilled else LsIcons.Person
}
