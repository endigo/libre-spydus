package com.librespydus.app.feature.profile

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.ui.components.PlaceholderScreen
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.components.SecondaryPillButton
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsSpacing
import kotlinx.coroutines.launch

/** Profile tab: library card, account, sign in/out, change library (§6, owned by A2). Wave-1 stub. */
@Composable
fun ProfileScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val library by appState.library.collectAsStateWithLifecycle()
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val signedIn by appState.isSignedIn.collectAsStateWithLifecycle()
    val demo by appState.isDemo.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    PlaceholderScreen(
        title = "Profile",
        message = listOfNotNull(
            library?.name,
            library?.origin,
            if (signedIn) "Signed in as ${accountName ?: "…"}" else "Not signed in",
            if (demo) "Demo mode" else null,
        ).joinToString("\n"),
        icon = LsIcons.Person,
        modifier = modifier,
    ) {
        Spacer(Modifier.height(LsSpacing.xl))
        if (signedIn) SecondaryPillButton("Sign out", onClick = { scope.launch { appState.signOut() } })
        else PrimaryPillButton("Sign in", onClick = appState::requestLogin)
        Spacer(Modifier.height(LsSpacing.m))
        SecondaryPillButton("Change library", onClick = appState::showLibraryPicker)
    }
}
