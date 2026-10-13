package com.librespydus.app.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.UserFacingError
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** Centered spinner (+ optional message). [onSheet] for white surfaces. */
@Composable
fun LoadingView(modifier: Modifier = Modifier, message: String? = null, onSheet: Boolean = false) {
    Column(
        modifier
            .fillMaxSize()
            .padding(LsSpacing.xl),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        CircularProgressIndicator(color = LsColors.primary, strokeWidth = 3.dp, modifier = Modifier.size(36.dp))
        if (message != null) {
            Spacer(Modifier.height(LsSpacing.l))
            Text(message, style = LsType.body, color = if (onSheet) LsColors.onSheetSecondary else LsColors.textSecondary, textAlign = TextAlign.Center)
        }
    }
}

/** Empty state: icon, title, message and an optional action ("No results for …"). */
@Composable
fun EmptyStateView(
    title: String,
    modifier: Modifier = Modifier,
    message: String? = null,
    icon: ImageVector = LsIcons.Book,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
    onSheet: Boolean = false,
) {
    StateLayout(
        modifier = modifier,
        icon = icon,
        iconTint = if (onSheet) LsColors.onSheetSecondary else LsColors.textSecondary,
        title = title,
        message = message,
        onSheet = onSheet,
    ) {
        if (actionLabel != null && onAction != null) {
            Spacer(Modifier.height(LsSpacing.l))
            PrimaryPillButton(actionLabel, onAction)
        }
    }
}

/**
 * Error state (§5.7): the user-facing message, Retry, and the technical detail behind a
 * "Details" disclosure.
 */
@Composable
fun ErrorStateView(
    error: UserFacingError,
    onRetry: (() -> Unit)?,
    modifier: Modifier = Modifier,
    onSheet: Boolean = false,
) {
    var showDetail by rememberSaveable { mutableStateOf(false) }
    StateLayout(
        modifier = modifier,
        icon = LsIcons.Error,
        iconTint = LsColors.danger,
        title = error.message,
        message = null,
        onSheet = onSheet,
    ) {
        if (onRetry != null) {
            Spacer(Modifier.height(LsSpacing.l))
            SecondaryPillButton(Copy.Common.retry, onRetry, color = if (onSheet) LsColors.onSheet else LsColors.textPrimary)
        }
        val detail = error.detail
        if (!detail.isNullOrBlank()) {
            Spacer(Modifier.height(LsSpacing.m))
            Row(
                Modifier
                    .clip(RoundedCornerShape(LsRadius.pill))
                    .clickable(role = Role.Button) { showDetail = !showDetail }
                    .padding(horizontal = LsSpacing.m, vertical = LsSpacing.xs),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val muted = if (onSheet) LsColors.onSheetSecondary else LsColors.textMuted
                Text(Copy.Errors.details, style = LsType.caption, color = muted)
                Icon(LsIcons.ExpandMore, null, tint = muted, modifier = Modifier.size(18.dp).rotate(if (showDetail) 180f else 0f))
            }
            AnimatedVisibility(showDetail) {
                Text(
                    detail,
                    style = LsType.caption,
                    color = if (onSheet) LsColors.onSheetSecondary else LsColors.textMuted,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(top = LsSpacing.s),
                )
            }
        }
    }
}

@Composable
private fun StateLayout(
    modifier: Modifier,
    icon: ImageVector,
    iconTint: androidx.compose.ui.graphics.Color,
    title: String,
    message: String?,
    onSheet: Boolean,
    extra: @Composable ColumnScope.() -> Unit,
) {
    Column(
        modifier
            .fillMaxSize()
            .padding(horizontal = LsSpacing.xxl, vertical = LsSpacing.xl),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier
                .size(72.dp)
                .background(if (onSheet) LsColors.chipOnSheet else LsColors.surface, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(icon, contentDescription = null, tint = iconTint, modifier = Modifier.size(32.dp))
        }
        Spacer(Modifier.height(LsSpacing.l))
        Text(title, style = LsType.section, color = if (onSheet) LsColors.onSheet else LsColors.textPrimary, textAlign = TextAlign.Center)
        if (message != null) {
            Spacer(Modifier.height(LsSpacing.s))
            Text(message, style = LsType.body, color = if (onSheet) LsColors.onSheetSecondary else LsColors.textSecondary, textAlign = TextAlign.Center)
        }
        extra()
    }
}

/**
 * A navy screen with a large title (like "Your Books") and a centered placeholder: used by the
 * wave-1 feature stubs, and for anything not built yet. [content] goes under the message.
 */
@Composable
fun PlaceholderScreen(
    title: String,
    modifier: Modifier = Modifier,
    message: String? = null,
    icon: ImageVector = LsIcons.Book,
    content: @Composable ColumnScope.() -> Unit = {},
) {
    Column(
        modifier
            .fillMaxSize()
            .background(LsColors.background)
            .statusBarsPadding(),
    ) {
        Text(
            title,
            style = LsType.title,
            color = LsColors.textPrimary,
            modifier = Modifier.padding(start = LsSpacing.xl, end = LsSpacing.xl, top = LsSpacing.xl, bottom = LsSpacing.l),
        )
        Column(
            Modifier
                .fillMaxWidth()
                .weight(1f)
                .padding(horizontal = LsSpacing.xl),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(Modifier.size(72.dp).background(LsColors.surface, CircleShape), contentAlignment = Alignment.Center) {
                Icon(icon, contentDescription = null, tint = LsColors.textSecondary, modifier = Modifier.size(32.dp))
            }
            if (message != null) {
                Spacer(Modifier.height(LsSpacing.l))
                Text(message, style = LsType.body, color = LsColors.textSecondary, textAlign = TextAlign.Center)
            }
            content()
        }
    }
}
