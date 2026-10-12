package com.librespydus.app.feature.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.core.AppState
import com.librespydus.app.ui.components.SecondaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/**
 * Human verification sheet content (§6, owned by A2): a web view on the library origin with
 * `CoreHelpers.userAgent`, watching for `aws-waf-token`, then `completeVerification`.
 * Wave-1 stub: copy + close only (no web view yet).
 */
@Composable
fun VerificationScreen(appState: AppState, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    val library by appState.library.collectAsStateWithLifecycle()
    Column(
        modifier
            .fillMaxWidth()
            .padding(LsSpacing.xl)
            .navigationBarsPadding(),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
    ) {
        Text("Verify you're human", style = LsType.title, color = LsColors.textPrimary)
        Text(
            "${library?.name ?: "Your library"} asks you to confirm you're human. Complete the check below.",
            style = LsType.body,
            color = LsColors.textSecondary,
        )
        SecondaryPillButton("Close", onClick = onDismiss)
    }
}
