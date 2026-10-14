package com.librespydus.app.feature.onboarding

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.font.FontWeight
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BiometricText
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.theme.LsColors
import kotlinx.coroutines.delay

/**
 * After a typed sign-in (§5.14): "Sign in with fingerprint next time?" — Use fingerprint / Not
 * now. Shown once the sign-in sheet has closed, so it doesn't sit on a closing sheet;
 * My Picks' copy offer waits for the answer. Turned on from Profile's switch, nothing is asked: the
 * check itself starts at that moment.
 */
@Composable
fun BiometricOfferDialog(appState: AppState) {
    val offer by appState.biometricOffer.collectAsStateWithLifecycle()
    val loginNeeded by appState.loginNeeded.collectAsStateWithLifecycle()

    var visible by remember { mutableStateOf(false) }
    LaunchedEffect(offer, loginNeeded) {
        visible = false
        val current = offer
        if (current != null && !loginNeeded) {
            delay(OFFER_DELAY_MILLIS)
            if (current.asks) visible = true else appState.answerBiometricOffer(use = true)
        }
    }
    val current = offer
    if (visible && current != null) {
        AlertDialog(
            onDismissRequest = { appState.answerBiometricOffer(use = false) },
            title = { Text(current.title) },
            text = { Text(current.message) },
            confirmButton = {
                TextButton(onClick = { appState.answerBiometricOffer(use = true) }) {
                    Text(Copy.Biometrics.use(BiometricText.METHOD), color = LsColors.textPrimary, fontWeight = FontWeight.SemiBold)
                }
            },
            dismissButton = {
                TextButton(onClick = { appState.answerBiometricOffer(use = false) }) { Text(Copy.Common.notNow, color = LsColors.textPrimary) }
            },
            containerColor = LsColors.surfaceElevated,
            titleContentColor = LsColors.textPrimary,
            textContentColor = LsColors.textSecondary,
        )
    }
}

private const val OFFER_DELAY_MILLIS = 450L
