package com.librespydus.app.feature.onboarding

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.AppState
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.PlaceholderScreen
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsSpacing
import kotlinx.coroutines.launch
import uniffi.librespydus_core.LibraryInfo

/**
 * Library picker (§6, owned by A2): first-launch onboarding ([onDismiss] null) or "Change
 * library" from Profile. Wave-1 stub: offers Canning so the app is usable before A2 lands.
 */
@Composable
fun LibraryPickerScreen(appState: AppState, onDismiss: (() -> Unit)?, modifier: Modifier = Modifier) {
    val scope = rememberCoroutineScope()
    Box(modifier) {
        PlaceholderScreen(
            title = "Choose your library",
            message = "Search Civica's directory of Spydus libraries, grouped by region.",
            icon = LsIcons.Library,
        ) {
            Spacer(Modifier.height(LsSpacing.xl))
            PrimaryPillButton("Continue with Canning", onClick = { scope.launch { appState.selectLibrary(CANNING) } })
        }
        if (onDismiss != null) {
            CircleIconButton(
                LsIcons.Close, "Close", onDismiss,
                background = LsColors.surface, shadow = false, size = 44.dp,
                modifier = Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(LsSpacing.l),
            )
        }
    }
}

private val CANNING = LibraryInfo(
    id = "canning",
    name = "City of Canning",
    origin = "https://canning.spydus.com",
    logoUrl = null,
    country = "Australia",
    region = "AU",
    isTest = false,
)
