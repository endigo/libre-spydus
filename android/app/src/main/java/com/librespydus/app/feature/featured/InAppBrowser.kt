package com.librespydus.app.feature.featured

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import androidx.browser.customtabs.CustomTabColorSchemeParams
import androidx.browser.customtabs.CustomTabsIntent
import androidx.compose.ui.graphics.toArgb
import androidx.core.net.toUri
import com.librespydus.app.ui.theme.LsColors

/**
 * Opens a web page in the in-app browser (§5.10: Featured web links, a collection page without
 * sections): a Custom Tab in the app's navy, or the default browser when no browser supports
 * Custom Tabs.
 */
object InAppBrowser {
    fun open(context: Context, url: String) {
        val uri = url.toUri()
        val colors = CustomTabColorSchemeParams.Builder()
            .setToolbarColor(LsColors.background.toArgb())
            .setNavigationBarColor(LsColors.background.toArgb())
            .build()
        val intent = CustomTabsIntent.Builder()
            .setDefaultColorSchemeParams(colors)
            .setColorScheme(CustomTabsIntent.COLOR_SCHEME_DARK)
            .setShowTitle(true)
            .setShareState(CustomTabsIntent.SHARE_STATE_ON)
            .build()
        try {
            intent.launchUrl(context, uri)
        } catch (_: ActivityNotFoundException) {
            try {
                context.startActivity(Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            } catch (_: ActivityNotFoundException) {
                // No browser at all: nothing to open it with.
            }
        }
    }
}
