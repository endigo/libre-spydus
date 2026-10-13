package com.librespydus.app.feature.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.BuildConfig
import com.librespydus.app.core.AppState
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.feature.onboarding.AccountCard
import com.librespydus.app.feature.onboarding.AccountDivider
import com.librespydus.app.feature.onboarding.AccountIcons
import com.librespydus.app.feature.onboarding.AccountRowButton
import com.librespydus.app.feature.onboarding.AccountSectionHeader
import com.librespydus.app.feature.onboarding.LibraryLogo
import com.librespydus.app.feature.onboarding.LibraryPickerLogic
import com.librespydus.app.feature.onboarding.PickerRegion
import com.librespydus.app.feature.onboarding.VerificationWebCookies
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.nav.AppNavigator
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsTabBar
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch
import uniffi.librespydus_core.LibraryInfo

/** Profile's pure bits (unit-tested). */
object ProfileText {
    /** "Version 0.1.0 (1)". */
    fun versionText(versionName: String? = BuildConfig.VERSION_NAME, versionCode: Int = BuildConfig.VERSION_CODE): String =
        "Version ${versionName ?: "?"} ($versionCode)"

    /** Initials for the account avatar; library names often come as "Surname, Given". */
    fun initials(accountName: String?): String {
        val name = accountName?.trim()?.takeIf { it.isNotEmpty() } ?: return ""
        val parts = name.split(',').map { it.trim() }
        val ordered = if (parts.size == 2) "${parts[1]} ${parts[0]}" else name
        return LibraryPickerLogic.monogram(ordered)
    }

    /** The library card's location chip: the directory's country, else the region's name. */
    fun place(library: LibraryInfo): String =
        library.country?.takeIf { it.isNotBlank() } ?: PickerRegion.of(library.region).title
}

private enum class AccountLoad { IDLE, LOADING, FAILED }

/**
 * Profile (docs/native-apps.md §6): the library card + Change library, the account (Sign in /
 * Sign out), notifications while signed in (§5.8), human verification for libraries that needed
 * it, About, and (DEBUG) demo mode.
 */
@Composable
fun ProfileScreen(appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    val library by appState.library.collectAsStateWithLifecycle()
    val accountName by appState.accountName.collectAsStateWithLifecycle()
    val signedIn by appState.isSignedIn.collectAsStateWithLifecycle()
    val demo by appState.isDemo.collectAsStateWithLifecycle()
    val verified by appState.hasVerificationCookie.collectAsStateWithLifecycle()
    val snapshot by appState.account.snapshot.collectAsStateWithLifecycle()
    val accountRefreshing by appState.account.isRefreshing.collectAsStateWithLifecycle()
    val refreshError by appState.account.lastRefreshError.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var loadingName by remember { mutableStateOf(false) }
    var confirmingSignOut by rememberSaveable { mutableStateOf(false) }
    var confirmingClear by rememberSaveable { mutableStateOf(false) }
    val libraryName = library?.name ?: "your library"

    // The name comes with the account snapshot (§5.2a). Only when a refresh failed on something
    // unexpected before any snapshot arrived (e.g. an unreadable loans page) is the name asked for
    // on its own with account(), which may still work.
    val noSnapshot = snapshot == null
    LaunchedEffect(signedIn, noSnapshot, refreshError) {
        if (!signedIn || !noSnapshot || refreshError?.kind != UserFacingError.Kind.UNEXPECTED || appState.accountName.value != null) return@LaunchedEffect
        loadingName = true
        try {
            appState.refreshAccount()
        } finally {
            loadingName = false
        }
    }
    // A library page without a name isn't an error: with a snapshot, Profile just says "Signed in".
    val accountLoad = when {
        !signedIn || accountName != null || !noSnapshot -> AccountLoad.IDLE
        accountRefreshing || loadingName || refreshError == null -> AccountLoad.LOADING
        else -> AccountLoad.FAILED
    }

    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = LsSpacing.l)
            // Clear the raised Search button at max scroll.
            .padding(bottom = LsSpacing.xxl),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.xl),
    ) {
        Text(
            "Profile",
            style = LsType.greeting,
            color = LsColors.textPrimary,
            modifier = Modifier
                .padding(top = LsSpacing.l)
                .semantics { heading() },
        )
        val current = library
        if (current != null) {
            LibraryCard(current, onChange = appState::showLibraryPicker)
            AccountSection(
                signedIn = signedIn,
                accountName = accountName,
                load = accountLoad,
                libraryName = libraryName,
                onRetry = { appState.account.refresh(force = true) },
                onSignIn = appState::requestLogin,
                onSignOut = { confirmingSignOut = true },
            )
            if (signedIn) NotificationsSection(appState)
            if (verified) VerificationSection(libraryName, onClear = { confirmingClear = true })
        } else {
            EmptyStateView(
                title = "No library selected",
                message = "Choose your library to search its catalogue and sign in.",
                icon = AccountIcons.Place,
                actionLabel = "Choose library",
                onAction = appState::showLibraryPicker,
                modifier = Modifier.height(360.dp),
            )
        }
        AboutSection()
        if (BuildConfig.DEBUG) DeveloperSection(demo)
    }

    if (confirmingSignOut) {
        AlertDialog(
            onDismissRequest = { confirmingSignOut = false },
            title = { Text("Sign out of $libraryName?") },
            text = { Text("You can sign in again at any time.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmingSignOut = false
                    scope.launch { appState.signOut() }
                }) { Text("Sign out", color = LsColors.danger) }
            },
            dismissButton = { TextButton(onClick = { confirmingSignOut = false }) { Text("Cancel", color = LsColors.textPrimary) } },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }
    if (confirmingClear) {
        AlertDialog(
            onDismissRequest = { confirmingClear = false },
            title = { Text("Clear verification?") },
            text = { Text("$libraryName will ask you to confirm you're human again.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmingClear = false
                    val origin = appState.backend.value?.origin
                    scope.launch {
                        appState.clearVerification()
                        origin?.let(VerificationWebCookies::clear)
                    }
                }) { Text("Clear verification", color = LsColors.danger) }
            },
            dismissButton = { TextButton(onClick = { confirmingClear = false }) { Text("Cancel", color = LsColors.textPrimary) } },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }
}

@Composable
private fun LibraryCard(library: LibraryInfo, onChange: () -> Unit) {
    AccountCard {
        Row(
            Modifier.semantics(mergeDescendants = true) {},
            verticalAlignment = Alignment.CenterVertically,
        ) {
            LibraryLogo(library, size = 60.dp)
            Spacer(Modifier.width(LsSpacing.l))
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                Text(library.name, style = LsType.section, color = LsColors.textPrimary)
                Text(
                    LibraryPickerLogic.host(library.origin),
                    style = LsType.caption,
                    color = LsColors.textSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.MiddleEllipsis,
                )
                PlaceChip(ProfileText.place(library), Modifier.padding(top = 2.dp))
            }
        }
        AccountDivider()
        AccountRowButton("Change library", AccountIcons.SwapHoriz, onChange)
    }
}

@Composable
private fun PlaceChip(text: String, modifier: Modifier = Modifier) {
    Row(
        modifier
            .clip(RoundedCornerShape(LsRadius.pill))
            .background(LsColors.surfaceElevated)
            .padding(horizontal = LsSpacing.m, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(AccountIcons.Place, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.size(14.dp))
        Spacer(Modifier.width(LsSpacing.xs))
        Text(text, style = LsType.caption, color = LsColors.textPrimary, maxLines = 1)
    }
}

@Composable
private fun AccountSection(
    signedIn: Boolean,
    accountName: String?,
    load: AccountLoad,
    libraryName: String,
    onRetry: () -> Unit,
    onSignIn: () -> Unit,
    onSignOut: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        AccountSectionHeader("Account")
        AccountCard {
            if (signedIn) {
                Row(Modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        Modifier
                            .size(52.dp)
                            .clip(CircleShape)
                            .background(LsColors.accent)
                            .clearAndSetSemantics {},
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(ProfileText.initials(accountName), color = LsColors.onAccent, fontWeight = FontWeight.Bold, fontSize = 19.sp, maxLines = 1)
                    }
                    Spacer(Modifier.width(LsSpacing.l))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                        when {
                            accountName != null -> Text(accountName, style = LsType.bodyStrong, color = LsColors.textPrimary)
                            load == AccountLoad.LOADING -> Row(verticalAlignment = Alignment.CenterVertically) {
                                CircularProgressIndicator(color = LsColors.textSecondary, strokeWidth = 2.dp, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(LsSpacing.s))
                                Text("Loading your account…", style = LsType.body, color = LsColors.textSecondary)
                            }
                            else -> Text("Signed in", style = LsType.bodyStrong, color = LsColors.textPrimary)
                        }
                        Text("Signed in to $libraryName", style = LsType.caption, color = LsColors.textSecondary)
                    }
                }
                if (load == AccountLoad.FAILED && accountName == null) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            "Couldn't load your account details.",
                            style = LsType.caption,
                            color = LsColors.warning,
                            modifier = Modifier.weight(1f),
                        )
                        Text(
                            "Retry",
                            style = LsType.bodyStrong,
                            color = LsColors.primary,
                            modifier = Modifier
                                .clip(RoundedCornerShape(LsRadius.pill))
                                .clickable(role = Role.Button, onClick = onRetry)
                                .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
                        )
                    }
                }
                AccountDivider()
                AccountRowButton("Sign out", AccountIcons.Logout, onSignOut, tint = LsColors.danger)
            } else {
                Row(Modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.Top) {
                    Icon(AccountIcons.AccountCircle, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(44.dp))
                    Spacer(Modifier.width(LsSpacing.l))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                        Text("You're not signed in", style = LsType.bodyStrong, color = LsColors.textPrimary)
                        Text(
                            "Sign in with your library card to see your loans and reservations.",
                            style = LsType.caption,
                            color = LsColors.textSecondary,
                        )
                    }
                }
                PrimaryPillButton("Sign in", onClick = onSignIn, modifier = Modifier.fillMaxWidth().padding(top = LsSpacing.xs))
            }
        }
    }
}

@Composable
private fun VerificationSection(libraryName: String, onClear: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        AccountSectionHeader("Human verification")
        AccountCard {
            Row(Modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.Top) {
                Icon(AccountIcons.VerifiedUser, contentDescription = null, tint = LsColors.success, modifier = Modifier.size(28.dp))
                Spacer(Modifier.width(LsSpacing.l))
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                    Text("Verified on this device", style = LsType.bodyStrong, color = LsColors.textPrimary)
                    Text(
                        "$libraryName asked you to confirm you're human. The app sends the library's verification cookie with its requests.",
                        style = LsType.caption,
                        color = LsColors.textSecondary,
                    )
                }
            }
            AccountDivider()
            AccountRowButton("Clear verification", AccountIcons.ShieldOff, onClear)
        }
    }
}

@Composable
private fun AboutSection() {
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        AccountSectionHeader("About")
        AccountCard {
            Row(Modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.CenterVertically) {
                Text("LibreSpydus", style = LsType.bodyStrong, color = LsColors.textPrimary, modifier = Modifier.weight(1f))
                Text(ProfileText.versionText(), style = LsType.caption, color = LsColors.textSecondary)
            }
            AccountDivider()
            Text("LibreSpydus is not affiliated with Civica or your library.", style = LsType.caption, color = LsColors.textSecondary)
        }
    }
}

@Composable
private fun DeveloperSection(demo: Boolean) {
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m)) {
        AccountSectionHeader("Developer")
        AccountCard {
            Row(Modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.CenterVertically) {
                Icon(AccountIcons.Build, contentDescription = null, tint = LsColors.textPrimary, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(LsSpacing.m))
                Text("Demo mode", style = LsType.bodyStrong, color = LsColors.textPrimary, modifier = Modifier.weight(1f))
                val tint = if (demo) LsColors.success else LsColors.textSecondary
                Text(
                    if (demo) "On" else "Off",
                    style = LsType.caption,
                    color = tint,
                    modifier = Modifier
                        .clip(RoundedCornerShape(LsRadius.pill))
                        .background(tint.copy(alpha = 0.18f))
                        .padding(horizontal = LsSpacing.m, vertical = 6.dp),
                )
            }
            Text(
                if (demo) "Account data is sample data and never reaches the library. Sign in with demo / demo. Relaunch without ls_demo to turn it off."
                else "Relaunch with --ez ls_demo true to use sample account data (debug builds only).",
                style = LsType.caption,
                color = LsColors.textSecondary,
            )
        }
    }
}
