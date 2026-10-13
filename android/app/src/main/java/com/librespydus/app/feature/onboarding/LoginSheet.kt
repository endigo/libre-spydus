package com.librespydus.app.feature.onboarding

import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import com.librespydus.app.BuildConfig
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** The sign-in form: card number + PIN → `AppState.signIn`, errors per docs/native-apps.md §5.7. */
class LoginFormState {
    var cardNumber by mutableStateOf("")
    var pin by mutableStateOf("")
    var isSigningIn by mutableStateOf(false)
        private set

    /**
     * Shown inline under the fields. A verification challenge isn't shown here: AppState presents
     * the verification sheet instead.
     */
    var error by mutableStateOf<UserFacingError?>(null)
        private set

    val canSubmit: Boolean get() = cardNumber.isNotBlank() && pin.isNotEmpty() && !isSigningIn

    /** Signs in; true on success (AppState clears `loginNeeded`, which closes the sheet). */
    suspend fun submit(appState: AppState): Boolean {
        if (!canSubmit) return false
        isSigningIn = true
        error = null
        try {
            appState.signIn(cardNumber.trim(), pin)
            pin = ""
            return true
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            // AppState.signIn has already routed it (verification sheet, …).
            val shown = UserFacingError.from(e, appState.library.value?.name)
            error = shown.takeUnless { it.kind == UserFacingError.Kind.VERIFICATION }
            return false
        } finally {
            isSigningIn = false
        }
    }

    fun clearError() {
        error = null
    }
}

/**
 * Sign-in sheet content (docs/native-apps.md §6 "Login"): library card number + PIN →
 * `AppState.signIn`. Closes on success (`signIn` clears `loginNeeded`); errors show inline.
 */
@Composable
fun LoginSheet(appState: AppState, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    val library by appState.library.collectAsStateWithLifecycle()
    val demo by appState.isDemo.collectAsStateWithLifecycle()
    val activity = LocalActivity.current
    val form = remember { LoginFormState() }
    // Signing in (and the account refresh after it) must outlive the sheet, which closes as soon
    // as AppState clears `loginNeeded`.
    val compositionScope = rememberCoroutineScope()
    val scope = (activity as? ComponentActivity)?.lifecycleScope ?: compositionScope
    val focus = LocalFocusManager.current
    val pinFocus = remember { FocusRequester() }
    val libraryName = library?.name ?: Copy.Common.yourLibrary

    fun submit() {
        if (!form.canSubmit) return
        focus.clearFocus()
        scope.launch { if (form.submit(appState)) onDismiss() }
    }

    LaunchedEffect(Unit) {
        // DEBUG screenshot hooks, demo mode only (never against a real library).
        if (!demo || form.cardNumber.isNotEmpty()) return@LaunchedEffect
        val card = OnboardingDebugHooks.string(activity, OnboardingDebugHooks.LOGIN_CARD) ?: return@LaunchedEffect
        form.cardNumber = card
        form.pin = OnboardingDebugHooks.string(activity, OnboardingDebugHooks.LOGIN_PIN).orEmpty()
        if (OnboardingDebugHooks.flag(activity, OnboardingDebugHooks.LOGIN_SUBMIT) && appState.backend.value?.isDemo == true) {
            delay(400)
            submit()
        }
    }

    Column(
        modifier
            .fillMaxWidth()
            .imePadding()
            .navigationBarsPadding()
            .verticalScroll(rememberScrollState()),
    ) {
        Row(Modifier.fillMaxWidth().padding(horizontal = LsSpacing.m), horizontalArrangement = Arrangement.End) {
            CircleIconButton(
                LsIcons.Close, Copy.Common.close, onDismiss,
                size = 36.dp, iconSize = 18.dp, background = LsColors.surface, contentColor = LsColors.textSecondary, shadow = false,
            )
        }
        Column(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = LsSpacing.xl)
                .padding(bottom = LsSpacing.xxl),
            verticalArrangement = Arrangement.spacedBy(LsSpacing.l),
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.m), modifier = Modifier.padding(bottom = LsSpacing.s)) {
                library?.let { LibraryLogo(it, size = 64.dp) }
                Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.xs)) {
                    Text(Copy.Login.title, style = LsType.greeting, color = LsColors.textPrimary, modifier = Modifier.semantics { heading() })
                    Text(libraryName.replaceFirstChar { it.uppercaseChar() }, style = LsType.section, color = LsColors.textSecondary)
                }
            }

            LoginField(
                label = Copy.Login.cardLabel,
                value = form.cardNumber,
                onValueChange = {
                    form.cardNumber = it
                    form.clearError()
                },
                placeholder = Copy.Login.cardPlaceholder,
                contentType = ContentType.Username,
                keyboardOptions = KeyboardOptions(
                    capitalization = KeyboardCapitalization.None,
                    autoCorrectEnabled = false,
                    keyboardType = KeyboardType.Ascii,
                    imeAction = ImeAction.Next,
                ),
                keyboardActions = KeyboardActions(onNext = { pinFocus.requestFocus() }),
                enabled = !form.isSigningIn,
            )
            LoginField(
                label = Copy.Login.pinLabel,
                value = form.pin,
                onValueChange = {
                    form.pin = it
                    form.clearError()
                },
                placeholder = Copy.Login.pinPlaceholder,
                contentType = ContentType.Password,
                keyboardOptions = KeyboardOptions(
                    autoCorrectEnabled = false,
                    keyboardType = KeyboardType.Password,
                    imeAction = ImeAction.Go,
                ),
                keyboardActions = KeyboardActions(onGo = { submit() }),
                visualTransformation = PasswordVisualTransformation(),
                enabled = !form.isSigningIn,
                modifier = Modifier.focusRequester(pinFocus),
            )

            AnimatedVisibility(form.error != null, enter = fadeIn(), exit = fadeOut()) {
                form.error?.let { AccountBanner(it.message, tone = AccountBannerTone.ERROR) }
            }

            PrimaryPillButton(
                text = if (form.isSigningIn) Copy.Account.signingIn else Copy.Account.signIn,
                onClick = ::submit,
                loading = form.isSigningIn,
                enabled = form.canSubmit || form.isSigningIn,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = LsSpacing.xs),
            )

            Text(
                Copy.Login.privacyNote(libraryName),
                style = LsType.caption,
                color = LsColors.textMuted,
            )

            if (BuildConfig.DEBUG && demo) {
                AccountBanner(
                    "Demo: demo / demo",
                    tone = AccountBannerTone.INFO,
                    modifier = Modifier.semantics(mergeDescendants = true) {
                        contentDescription = "Demo mode. Card number demo, PIN demo."
                    },
                )
            }
        }
    }
}

/** A caption over a rounded navy field (card number / PIN). */
@Composable
private fun LoginField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    contentType: ContentType,
    keyboardOptions: KeyboardOptions,
    keyboardActions: KeyboardActions,
    enabled: Boolean,
    modifier: Modifier = Modifier,
    visualTransformation: VisualTransformation = VisualTransformation.None,
) {
    Column(verticalArrangement = Arrangement.spacedBy(LsSpacing.s)) {
        Text(label, style = LsType.caption, color = LsColors.textSecondary)
        val shape = RoundedCornerShape(LsRadius.field)
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            enabled = enabled,
            singleLine = true,
            textStyle = LsType.body.copy(color = LsColors.textPrimary),
            cursorBrush = SolidColor(LsColors.primary),
            keyboardOptions = keyboardOptions,
            keyboardActions = keyboardActions,
            visualTransformation = visualTransformation,
            modifier = modifier
                .fillMaxWidth()
                .semantics {
                    this.contentType = contentType
                    contentDescription = label
                },
            decorationBox = { inner ->
                Box(
                    Modifier
                        .fillMaxWidth()
                        .heightIn(min = 52.dp)
                        .clip(shape)
                        .background(LsColors.field)
                        .padding(horizontal = LsSpacing.l, vertical = LsSpacing.m),
                    contentAlignment = Alignment.CenterStart,
                ) {
                    if (value.isEmpty()) Text(placeholder, style = LsType.body, color = LsColors.textMuted)
                    inner()
                }
            },
        )
    }
}
