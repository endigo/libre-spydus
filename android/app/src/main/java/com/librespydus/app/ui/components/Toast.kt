package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** What a [ToastPill] says: work in progress (a spinner), a confirmation or a failure. */
enum class ToastTone {
    PROGRESS,
    SUCCESS,
    ERROR,
    ;

    /** How long it stays up; a progress toast stays until the next one replaces it (null). */
    val millis: Long? get() = when (this) {
        PROGRESS -> null
        SUCCESS -> 2_500L
        ERROR -> 4_000L
    }
}

/**
 * A short message in a pill above the tab bar (My Picks' saves, a loan's renewal): a spinner while
 * [tone] is [ToastTone.PROGRESS], else [successIcon] (accent) or an error icon (danger). Announced
 * by TalkBack; tapping it dismisses it.
 */
@Composable
fun ToastPill(
    message: String,
    tone: ToastTone,
    successIcon: ImageVector,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val shape = RoundedCornerShape(LsRadius.pill)
    Row(
        modifier
            .widthIn(max = 520.dp)
            .shadow(8.dp, shape)
            .clip(shape)
            .background(LsColors.surfaceElevated)
            .clickable(onClickLabel = Copy.Common.dismiss, onClick = onDismiss)
            .semantics(mergeDescendants = true) {
                liveRegion = LiveRegionMode.Polite
                contentDescription = Copy.A11y.labelAndHint(message, Copy.A11y.dismissHint)
            }
            .padding(horizontal = LsSpacing.l, vertical = LsSpacing.m),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
    ) {
        when (tone) {
            ToastTone.PROGRESS -> CircularProgressIndicator(
                color = LsColors.textSecondary,
                strokeWidth = 2.dp,
                modifier = Modifier.size(18.dp),
            )
            ToastTone.SUCCESS -> Icon(successIcon, contentDescription = null, tint = LsColors.accent, modifier = Modifier.size(20.dp))
            ToastTone.ERROR -> Icon(LsIcons.Error, contentDescription = null, tint = LsColors.danger, modifier = Modifier.size(20.dp))
        }
        Text(message, style = LsType.bodyStrong, color = LsColors.textPrimary)
    }
}
