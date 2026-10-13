package com.librespydus.app.feature.onboarding

import android.annotation.SuppressLint
import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Bitmap
import android.webkit.CookieManager
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
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
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.net.toUri
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Copy
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.ui.components.CircleIconButton
import com.librespydus.app.ui.components.ErrorStateView
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.PrimaryPillButton
import com.librespydus.app.ui.components.SecondaryPillButton
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Human-verification sheet content (docs/native-apps.md §6 "Verification", core/README.md
 * "Human verification"): the library's page in an [WebView] for the **user** to complete the AWS
 * WAF check, with the core's User-Agent. When the library's `aws-waf-token` cookie appears in
 * [CookieManager], `AppState.completeVerification` stores and applies it, the sheet closes and
 * the failed screen retries (`verificationGeneration`). The app never injects scripts into or
 * interacts with the check: it only reads cookies and main-frame response headers.
 *
 * After two completed verifications the library still rejects, the sheet stops offering the
 * check and points to the library's website instead (core/README.md step 3), until the user
 * clears the verification in Profile or picks the library again.
 */
@Composable
fun VerificationScreen(appState: AppState, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    val library by appState.library.collectAsStateWithLifecycle()
    val backend by appState.backend.collectAsStateWithLifecycle()
    val origin = backend?.origin ?: library?.origin ?: return
    val libraryName = library?.name ?: Copy.Common.yourLibrary
    // Fixed when the sheet opens: completing a check bumps the count and closes the sheet. Profile →
    // "Clear verification" drops the stored token, which offers the check again.
    val unavailable = remember(origin) {
        appState.verificationAttempts.value >= MAX_VERIFICATIONS && appState.hasVerificationCookie.value
    }
    val session = remember(origin) { VerificationSession(origin, libraryName) }
    val webView = remember(origin) { WebViewHolder() }

    Column(modifier.fillMaxSize()) {
        TopActions(onCancel = onDismiss, onReload = if (unavailable) null else { { webView.reload(session.pageUrl) } })
        VerificationHeader(libraryName, unavailable)
        if (unavailable) {
            UnavailableContent(libraryName, origin, onDismiss)
        } else {
            VerificationWebContent(appState, session, webView, onDismiss)
        }
    }
}

/** The sheet's web view, for the Reload button. */
private class WebViewHolder {
    var view: WebView? = null

    fun reload(pageUrl: String) {
        val webView = view ?: return
        if (webView.url == null) webView.loadUrl(pageUrl) else webView.reload()
    }
}

/** Completed verifications after which the library is treated as unusable from the app. */
private const val MAX_VERIFICATIONS = 2

@Composable
private fun VerificationHeader(libraryName: String, unavailable: Boolean) {
    Column(
        Modifier
            .fillMaxWidth()
            .padding(start = LsSpacing.l, end = LsSpacing.l, bottom = LsSpacing.l),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
    ) {
        Text(
            if (unavailable) Copy.Errors.libraryUnavailableTitle else Copy.Verification.title,
            style = LsType.title,
            color = LsColors.textPrimary,
            modifier = Modifier.semantics { heading() },
        )
        Text(
            if (unavailable) Copy.Errors.libraryUnavailable(libraryName).sentenceStart()
            else Copy.Verification.message(libraryName).sentenceStart(),
            style = LsType.body,
            color = LsColors.textSecondary,
        )
        if (!unavailable) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(AccountIcons.VerifiedUser, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(14.dp))
                Spacer(Modifier.width(LsSpacing.xs))
                Text(Copy.Verification.closesByItself, style = LsType.caption, color = LsColors.textMuted)
            }
        }
    }
}

@Composable
private fun TopActions(onCancel: () -> Unit, onReload: (() -> Unit)?) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = LsSpacing.s),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            Copy.Common.cancel,
            style = LsType.bodyStrong,
            color = LsColors.primary,
            modifier = Modifier
                .clip(RoundedCornerShape(LsRadius.pill))
                .clickable(role = Role.Button, onClick = onCancel)
                .padding(horizontal = LsSpacing.s, vertical = LsSpacing.s),
        )
        Spacer(Modifier.weight(1f))
        if (onReload != null) {
            CircleIconButton(
                LsIcons.Refresh, Copy.Verification.reload, onReload,
                size = 44.dp, iconSize = 22.dp, background = Color.Transparent, contentColor = LsColors.textSecondary, shadow = false,
            )
        }
    }
}

@Composable
private fun UnavailableContent(libraryName: String, origin: String, onDismiss: () -> Unit) {
    val context = LocalContext.current
    Column(
        Modifier
            .fillMaxWidth()
            .navigationBarsPadding()
            .padding(horizontal = LsSpacing.l, vertical = LsSpacing.xl),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
    ) {
        PrimaryPillButton(
            Copy.Common.openWebsite,
            onClick = {
                try {
                    context.startActivity(Intent(Intent.ACTION_VIEW, "$origin/".toUri()))
                } catch (_: ActivityNotFoundException) {
                    // No browser: nothing else to offer.
                }
            },
            modifier = Modifier.fillMaxWidth(),
        )
        SecondaryPillButton(Copy.Common.close, onClick = onDismiss, modifier = Modifier.fillMaxWidth())
        Text(
            Copy.Verification.keepsAsking(libraryName).sentenceStart(),
            style = LsType.caption,
            color = LsColors.textMuted,
        )
    }
}

@SuppressLint("SetJavaScriptEnabled")
@Composable
private fun VerificationWebContent(appState: AppState, session: VerificationSession, holder: WebViewHolder, onDismiss: () -> Unit) {
    // Storing the token must finish even if the sheet goes away meanwhile.
    val compositionScope = rememberCoroutineScope()
    val scope = (LocalActivity.current as? ComponentActivity)?.lifecycleScope ?: compositionScope
    val currentDismiss by rememberUpdatedState(onDismiss)

    fun check() {
        val cookie = session.verifiedCookie(CookieManager.getInstance().getCookie(session.pageUrl)) ?: return
        CookieManager.getInstance().flush()
        scope.launch {
            appState.completeVerification(cookie)
            currentDismiss()
        }
    }

    // Android's CookieManager has no change callback: look for the token twice a second while
    // the sheet is up, and after every page load.
    LaunchedEffect(session) {
        while (session.completedCookie == null) {
            delay(500)
            check()
        }
    }

    Box(
        Modifier
            .fillMaxSize()
            .clip(RoundedCornerShape(topStart = LsRadius.card, topEnd = LsRadius.card))
            .background(LsColors.sheet),
    ) {
        AndroidView(
            factory = { context ->
                WebView(context).apply {
                    setBackgroundColor(android.graphics.Color.WHITE)
                    settings.javaScriptEnabled = true // the WAF check is a script-driven page
                    settings.domStorageEnabled = true
                    // The WAF token may be tied to the User-Agent, so the web view sends the core's.
                    settings.userAgentString = CoreHelpers.userAgent
                    settings.setSupportMultipleWindows(false)
                    CookieManager.getInstance().setAcceptCookie(true)
                    webViewClient = VerificationWebClient(session, onPageLoaded = ::check)
                    holder.view = this
                    loadUrl(session.pageUrl)
                }
            },
            onRelease = { webView ->
                holder.view = null
                webView.stopLoading()
                webView.webViewClient = WebViewClient()
                webView.destroy()
            },
            modifier = Modifier
                .fillMaxSize()
                .navigationBarsPadding(),
        )
        val error = session.loadError
        when {
            error != null -> ErrorStateView(
                error,
                onRetry = { holder.reload(session.pageUrl) },
                modifier = Modifier.background(LsColors.background),
            )
            session.isLoading && session.showingChallenge == null ->
                LoadingView(message = Copy.Verification.opening(session.libraryName), modifier = Modifier.background(LsColors.background))
            session.finishedWithoutCheck -> Box(Modifier.fillMaxSize().padding(LsSpacing.l), contentAlignment = Alignment.BottomCenter) {
                Box(Modifier.clip(RoundedCornerShape(LsRadius.field)).background(LsColors.background)) {
                    AccountBanner(
                        Copy.Verification.noCheck(session.libraryName).sentenceStart(),
                        tone = AccountBannerTone.INFO,
                    )
                }
            }
        }
    }
}

/** "your library asks…" → "Your library asks…": the stand-in name can start a sentence. */
private fun String.sentenceStart(): String = replaceFirstChar { it.uppercaseChar() }

/** Reads main-frame page events and WAF response headers only. */
private class VerificationWebClient(
    private val session: VerificationSession,
    private val onPageLoaded: () -> Unit,
) : WebViewClient() {
    override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) {
        session.didStartLoading()
    }

    override fun onPageFinished(view: WebView, url: String?) {
        session.didFinishLoading()
        onPageLoaded()
    }

    override fun onReceivedHttpError(view: WebView, request: WebResourceRequest, errorResponse: WebResourceResponse) {
        if (request.isForMainFrame) session.didReceiveMainFrameHttpError(errorResponse.responseHeaders)
    }

    override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
        if (request.isForMainFrame) session.didFailLoading(error.description?.toString())
    }
}

/** The verification token as the web view stores it (Profile → "Clear verification"). */
object VerificationWebCookies {
    /** Expires the library's `aws-waf-token` cookies, so the next check starts fresh. */
    fun clear(origin: String) {
        val manager = CookieManager.getInstance()
        val url = origin.trimEnd('/') + "/"
        VerificationCookie.expiringCookies(LibraryPickerLogic.host(origin)).forEach { manager.setCookie(url, it) }
        manager.flush()
    }
}
