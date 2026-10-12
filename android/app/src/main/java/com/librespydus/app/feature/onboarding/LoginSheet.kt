package com.librespydus.app.feature.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.librespydus.app.BuildConfig
import com.librespydus.app.core.AppState
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.launch
import uniffi.librespydus_core.SpydusException

/** Sign-in sheet content (§6, owned by A2). Wave-1 stub: card number + PIN → `signIn`. */
@Composable
fun LoginSheet(appState: AppState, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    val library by appState.library.collectAsStateWithLifecycle()
    val demo by appState.isDemo.collectAsStateWithLifecycle()
    var card by remember { mutableStateOf("") }
    var pin by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<UserFacingError?>(null) }
    val scope = rememberCoroutineScope()
    Column(
        modifier
            .fillMaxWidth()
            .padding(horizontal = LsSpacing.xl)
            .padding(bottom = LsSpacing.xl)
            .navigationBarsPadding()
            .imePadding(),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        Text("Sign in to ${library?.name.orEmpty()}", style = LsType.title, color = LsColors.textPrimary)
        OutlinedTextField(card, { card = it }, label = { Text("Library card number") }, singleLine = true, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(
            pin, { pin = it }, label = { Text("PIN / password") }, singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.fillMaxWidth(),
        )
        error?.let { Text(it.message, style = LsType.body, color = LsColors.danger) }
        if (BuildConfig.DEBUG && demo) Text("Demo mode: card “demo”, PIN “demo”.", style = LsType.caption, color = LsColors.textSecondary)
        PrimaryPillButton(
            "Sign in",
            loading = busy,
            enabled = card.isNotBlank() && pin.isNotEmpty(),
            modifier = Modifier.fillMaxWidth(),
            onClick = {
                busy = true
                error = null
                scope.launch {
                    try {
                        appState.signIn(card, pin)
                        onDismiss()
                    } catch (e: SpydusException) {
                        error = UserFacingError.from(e, library?.name)
                    } finally {
                        busy = false
                    }
                }
            },
        )
    }
}
