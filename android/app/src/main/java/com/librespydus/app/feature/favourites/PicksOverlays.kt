package com.librespydus.app.feature.favourites

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.PicksNotice
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.components.ToastPill
import com.librespydus.app.ui.components.ToastTone
import com.librespydus.app.ui.components.tabBarContentPadding
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsSpacing
import kotlinx.coroutines.delay

/**
 * My Picks' app-wide pieces (§5.11), over the tabs: the short confirmation or failure of a save or
 * remove (and the copy's summary) as a pill above the tab bar, and the once-per-library offer to
 * copy this phone's favourites, shown once the sign-in sheet has closed.
 */
@Composable
fun PicksOverlays(appState: AppState) {
    val notice by appState.picks.notice.collectAsStateWithLifecycle()
    val offer by appState.picks.offer.collectAsStateWithLifecycle()
    val loginNeeded by appState.loginNeeded.collectAsStateWithLifecycle()

    var shown by remember { mutableStateOf<PicksNotice?>(null) }
    LaunchedEffect(notice) { notice?.let { shown = it } }
    LaunchedEffect(notice?.id) {
        val current = notice ?: return@LaunchedEffect
        delay(if (current.isError) ERROR_MILLIS else CONFIRMATION_MILLIS)
        appState.picks.dismissNotice(current.id)
    }
    Box(
        Modifier
            .fillMaxSize()
            .padding(bottom = tabBarContentPadding() + LsSpacing.m, start = LsSpacing.l, end = LsSpacing.l),
        contentAlignment = Alignment.BottomCenter,
    ) {
        AnimatedVisibility(
            visible = notice != null,
            enter = slideInVertically { it / 2 } + fadeIn(),
            exit = slideOutVertically { it / 2 } + fadeOut(),
        ) {
            shown?.let {
                ToastPill(
                    it.message,
                    tone = if (it.isError) ToastTone.ERROR else ToastTone.SUCCESS,
                    successIcon = LsIcons.BookmarkFilled,
                    onDismiss = { appState.picks.dismissNotice(it.id) },
                )
            }
        }
    }

    // After the sign-in sheet has gone, so the alert doesn't sit on a closing sheet.
    var offerVisible by remember { mutableStateOf(false) }
    LaunchedEffect(offer, loginNeeded) {
        offerVisible = false
        if (offer != null && !loginNeeded) {
            delay(OFFER_DELAY_MILLIS)
            offerVisible = true
        }
    }
    val current = offer
    if (offerVisible && current != null) {
        AlertDialog(
            onDismissRequest = { appState.picks.answerOffer(add = false) },
            title = { Text(Copy.Picks.offerTitle) },
            text = { Text(current.message) },
            confirmButton = {
                TextButton(onClick = { appState.picks.answerOffer(add = true) }) { Text(Copy.Common.add, color = LsColors.primary) }
            },
            dismissButton = {
                TextButton(onClick = { appState.picks.answerOffer(add = false) }) { Text(Copy.Common.notNow, color = LsColors.textPrimary) }
            },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }
}

private const val CONFIRMATION_MILLIS = 2_500L
private const val ERROR_MILLIS = 4_000L
private const val OFFER_DELAY_MILLIS = 450L
