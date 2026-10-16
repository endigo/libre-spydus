package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.feature.onboarding.LibraryLogo
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import com.librespydus.app.ui.theme.PageLayout
import kotlinx.coroutines.launch

/**
 * The wide layout's sidebar (docs/native-apps.md §6.1), in place of the tab bar: the library and
 * Change library at the top, the five sections in the tab bar's order (the selected one a white
 * pill with navy text), and the account at the bottom (its name, which opens Profile, and Sign
 * out, confirmed as on Profile; or Sign in). Navy, [PageLayout.sidebarWidth] wide, running under
 * the status bar with its content below it, and a 1 dp divider along its trailing edge.
 */
@Composable
fun LsSidebar(
    appState: AppState,
    selected: Tab,
    onSelect: (Tab) -> Unit,
    modifier: Modifier = Modifier,
) {
    val library by appState.library.collectAsStateWithLifecycle()
    val signedIn by appState.isSignedIn.collectAsStateWithLifecycle()
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var confirmingSignOut by rememberSaveable { mutableStateOf(false) }
    val libraryName = library?.name ?: Copy.Common.yourLibrary

    Row(
        modifier
            .width(PageLayout.sidebarWidth)
            .fillMaxHeight()
            .background(LsColors.background),
    ) {
        Column(
            Modifier
                .weight(1f)
                .fillMaxHeight()
                .statusBarsPadding(),
        ) {
            Row(
                Modifier
                    .padding(horizontal = LsSpacing.l)
                    .padding(top = LsSpacing.l, bottom = LsSpacing.xl),
                horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                LibraryLogo(libraryName, library?.logoUrl, size = PageLayout.sidebarLogoSize)
                Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.xs / 2)) {
                    Text(
                        libraryName,
                        style = LsType.bodyStrong,
                        color = LsColors.textPrimary,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.semantics { heading() },
                    )
                    Text(
                        Copy.Profile.changeLibrary,
                        style = LsType.caption,
                        color = LsColors.textSecondary,
                        modifier = Modifier
                            .clip(RoundedCornerShape(LsRadius.pill))
                            .clickable(role = Role.Button, onClick = appState::showLibraryPicker)
                            .padding(vertical = 2.dp),
                    )
                }
            }
            Column(
                Modifier.padding(horizontal = LsSpacing.s),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.xs),
            ) {
                Tab.entries.forEach { tab ->
                    SidebarItem(tab, selected = tab == selected, onClick = { onSelect(tab) })
                }
            }
            Spacer(Modifier.weight(1f).heightIn(min = LsSpacing.l))
            HorizontalDivider(color = LsColors.divider, thickness = 1.dp)
            Column(
                Modifier.padding(LsSpacing.l),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
            ) {
                if (signedIn) {
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(LsRadius.field))
                            .clickable(role = Role.Button, onClickLabel = Copy.Tabs.profile, onClick = { onSelect(Tab.PROFILE) })
                            .padding(vertical = LsSpacing.xs),
                        horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(AccountIcons.AccountCircle, contentDescription = null, tint = LsColors.textPrimary, modifier = Modifier.size(22.dp))
                        Text(
                            accountName ?: Copy.Account.signedIn,
                            style = LsType.bodyStrong,
                            color = LsColors.textPrimary,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    Text(
                        Copy.Account.signOut,
                        style = LsType.caption,
                        color = LsColors.textSecondary,
                        modifier = Modifier
                            .clip(RoundedCornerShape(LsRadius.pill))
                            .clickable(role = Role.Button, onClick = { confirmingSignOut = true })
                            .padding(vertical = 2.dp),
                    )
                } else {
                    PrimaryPillButton(Copy.Account.signIn, onClick = appState::requestLogin, modifier = Modifier.fillMaxWidth())
                }
            }
        }
        VerticalDivider(color = LsColors.divider, thickness = 1.dp)
    }

    if (confirmingSignOut) {
        AlertDialog(
            onDismissRequest = { confirmingSignOut = false },
            title = { Text(Copy.Profile.signOutTitle(libraryName)) },
            text = { Text(Copy.Profile.signOutMessage) },
            confirmButton = {
                TextButton(onClick = {
                    confirmingSignOut = false
                    scope.launch { appState.signOut() }
                }) { Text(Copy.Account.signOut, color = LsColors.danger) }
            },
            dismissButton = { TextButton(onClick = { confirmingSignOut = false }) { Text(Copy.Common.cancel, color = LsColors.textPrimary) } },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }
}

/** One section: icon (filled while selected) and title; the selected one a white pill with navy text. */
@Composable
private fun SidebarItem(tab: Tab, selected: Boolean, onClick: () -> Unit) {
    val color = if (selected) LsColors.tabActive else LsColors.textSecondary
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 44.dp)
            .clip(RoundedCornerShape(LsRadius.field))
            .background(if (selected) LsColors.tabBar else Color.Transparent)
            .selectable(selected = selected, role = Role.Tab, onClick = onClick)
            .padding(horizontal = LsSpacing.m),
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.m),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(tab.icon(selected), contentDescription = null, tint = color, modifier = Modifier.size(24.dp))
        Text(
            tab.label,
            style = if (selected) LsType.bodyStrong else LsType.body,
            color = color,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}
