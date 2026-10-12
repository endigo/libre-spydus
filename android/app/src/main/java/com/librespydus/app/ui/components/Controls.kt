package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** "New arrivals ……… View All" (§6). No action when [onAction] is null. */
@Composable
fun SectionHeader(
    title: String,
    modifier: Modifier = Modifier,
    actionLabel: String = "View All",
    onAction: (() -> Unit)? = null,
    color: Color = LsColors.textPrimary,
    actionColor: Color = LsColors.textSecondary,
) {
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(title, style = LsType.section, color = color, modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
        if (onAction != null) {
            Text(
                actionLabel,
                style = LsType.caption,
                color = actionColor,
                modifier = Modifier
                    .clip(RoundedCornerShape(LsRadius.pill))
                    .clickable(role = Role.Button, onClick = onAction)
                    .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
            )
        }
    }
}

/**
 * The rounded search field (§6). Editable by default; with [onClick] it is a button that looks
 * like the field (Home → focuses the Search tab).
 */
@Composable
fun SearchField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "Search by title, author, etc.",
    onSearch: ((String) -> Unit)? = null,
    onClick: (() -> Unit)? = null,
    focusRequester: FocusRequester? = null,
) {
    val shape = RoundedCornerShape(LsRadius.field)
    val container = modifier
        .fillMaxWidth()
        .heightIn(min = 52.dp)
        .clip(shape)
        .background(LsColors.field, shape)
    val decoration: @Composable (@Composable () -> Unit) -> Unit = { inner ->
        Row(Modifier.padding(horizontal = LsSpacing.l, vertical = LsSpacing.m), verticalAlignment = Alignment.CenterVertically) {
            Icon(LsIcons.Search, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(20.dp))
            Spacer(Modifier.width(LsSpacing.m))
            Box(Modifier.weight(1f)) {
                if (value.isEmpty()) Text(placeholder, style = LsType.body, color = LsColors.textMuted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                inner()
            }
            if (value.isNotEmpty() && onClick == null) {
                Icon(
                    LsIcons.Close,
                    contentDescription = "Clear search",
                    tint = LsColors.textSecondary,
                    modifier = Modifier
                        .size(28.dp)
                        .clip(CircleShape)
                        .clickable { onValueChange("") }
                        .padding(4.dp),
                )
            }
        }
    }
    if (onClick != null) {
        Box(container.clickable(role = Role.Button, onClick = onClick).semantics { contentDescription = placeholder }) {
            decoration { Text(value, style = LsType.body, color = LsColors.textPrimary, maxLines = 1) }
        }
    } else {
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = true,
            textStyle = LsType.body.copy(color = LsColors.textPrimary),
            cursorBrush = SolidColor(LsColors.primary),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { onSearch?.invoke(value) }),
            modifier = container.then(if (focusRequester != null) Modifier.focusRequester(focusRequester) else Modifier),
            decorationBox = { inner -> decoration(inner) },
        )
    }
}

/** Coral gradient pill — the primary action ("Reserve", "Sign in"). */
@Composable
fun PrimaryPillButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    loading: Boolean = false,
) {
    val shape = RoundedCornerShape(LsRadius.pill)
    Box(
        modifier
            .defaultMinSize(minWidth = 120.dp)
            .height(52.dp)
            .alpha(if (enabled) 1f else 0.5f)
            .shadow(if (enabled) 10.dp else 0.dp, shape, ambientColor = LsColors.primary, spotColor = LsColors.primary)
            .clip(shape)
            .background(LsColors.primaryGradient)
            .clickable(enabled = enabled && !loading, role = Role.Button, onClick = onClick)
            .padding(horizontal = LsSpacing.xl),
        contentAlignment = Alignment.Center,
    ) {
        if (loading) {
            CircularProgressIndicator(color = LsColors.onPrimary, strokeWidth = 2.dp, modifier = Modifier.size(22.dp))
        } else {
            Text(text, style = LsType.bodyStrong, color = LsColors.onPrimary, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** A quiet outlined pill (secondary actions such as "Retry", "Sign out"). */
@Composable
fun SecondaryPillButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    color: Color = LsColors.textPrimary,
    enabled: Boolean = true,
) {
    val shape = RoundedCornerShape(LsRadius.pill)
    Box(
        modifier
            .defaultMinSize(minWidth = 96.dp)
            .height(44.dp)
            .alpha(if (enabled) 1f else 0.5f)
            .clip(shape)
            .border(1.dp, color.copy(alpha = 0.5f), shape)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .padding(horizontal = LsSpacing.l),
        contentAlignment = Alignment.Center,
    ) {
        Text(text, style = LsType.bodyStrong, color = color, maxLines = 1)
    }
}

/** Round icon button: blue bookmark on the detail screen, back buttons, etc. */
@Composable
fun CircleIconButton(
    icon: ImageVector,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 52.dp,
    iconSize: Dp = 22.dp,
    background: Color = LsColors.accent,
    contentColor: Color = LsColors.onAccent,
    shadow: Boolean = true,
) {
    Box(
        modifier
            .size(size)
            .shadow(if (shadow) 8.dp else 0.dp, CircleShape, ambientColor = background, spotColor = background)
            .clip(CircleShape)
            .background(background)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { this.contentDescription = contentDescription },
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, contentDescription = null, tint = contentColor, modifier = Modifier.size(iconSize))
    }
}

/** Screen top bar on navy: optional back button, optional title, trailing slot. */
@Composable
fun LsTopBar(
    modifier: Modifier = Modifier,
    title: String? = null,
    onBack: (() -> Unit)? = null,
    contentColor: Color = LsColors.textPrimary,
    trailing: @Composable () -> Unit = {},
) {
    Row(
        modifier
            .fillMaxWidth()
            .height(56.dp)
            .padding(horizontal = LsSpacing.s),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (onBack != null) {
            CircleIconButton(
                LsIcons.ArrowBack, "Back", onBack,
                size = 44.dp, iconSize = 24.dp, background = Color.Transparent, contentColor = contentColor, shadow = false,
            )
        } else {
            Spacer(Modifier.width(LsSpacing.s))
        }
        Text(
            title.orEmpty(),
            style = LsType.title,
            color = contentColor,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier
                .weight(1f)
                .padding(horizontal = LsSpacing.s),
        )
        trailing()
    }
}

/** A rounded colour tile with an icon and a label underneath ("Popular genres"). */
@Composable
fun GenreTile(
    label: String,
    icon: ImageVector,
    color: Color,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 84.dp,
) {
    val shape = RoundedCornerShape(LsRadius.tile)
    Column(
        modifier
            .width(size)
            .clip(RoundedCornerShape(LsRadius.tile))
            .clickable(role = Role.Button, onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
    ) {
        Box(
            Modifier
                .size(size)
                .clip(shape)
                .background(color),
            contentAlignment = Alignment.Center,
        ) {
            Box(Modifier.size(size * 0.5f).background(Color.White.copy(alpha = 0.16f), CircleShape))
            Icon(icon, contentDescription = null, tint = Color.White, modifier = Modifier.size(size * 0.36f))
        }
        Text(label, style = LsType.caption, color = LsColors.textSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center)
    }
}

/**
 * A small rounded label: format chips (navy), subject chips (white sheet, [onSheet]).
 * [tint] colours it (e.g. `LsColors.digital` for digital formats).
 */
@Composable
fun Chip(
    text: String,
    modifier: Modifier = Modifier,
    selected: Boolean = false,
    onSheet: Boolean = false,
    tint: Color? = null,
    onClick: (() -> Unit)? = null,
) {
    val shape = RoundedCornerShape(LsRadius.pill)
    val (container, content) = when {
        // Selected = coral, as on iOS (design/tokens.json `primary`).
        selected -> LsColors.primary to LsColors.onPrimary
        tint != null -> tint.copy(alpha = 0.18f) to tint
        onSheet -> LsColors.chipOnSheet to LsColors.onSheet
        else -> LsColors.surfaceElevated to LsColors.textPrimary
    }
    Box(
        modifier
            .clip(shape)
            .background(container)
            .then(if (tint != null && !selected) Modifier.border(1.dp, tint.copy(alpha = 0.5f), shape) else Modifier)
            .then(if (onClick != null) Modifier.clickable(role = Role.Tab, onClick = onClick) else Modifier)
            .semantics { this.selected = selected }
            .padding(horizontal = LsSpacing.m, vertical = 6.dp),
    ) {
        Text(text, style = LsType.caption, color = content, maxLines = 1)
    }
}

/** Two or more segments in a pill track ("Loans | Reservations", "Relevance | Newest"). */
@Composable
fun SegmentedToggle(
    options: List<String>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val shape = RoundedCornerShape(LsRadius.pill)
    Row(
        modifier
            .clip(shape)
            .background(LsColors.field)
            .padding(4.dp),
    ) {
        options.forEachIndexed { index, label ->
            val selected = index == selectedIndex
            val interaction = remember { MutableInteractionSource() }
            Box(
                Modifier
                    .weight(1f)
                    .heightIn(min = 36.dp)
                    .clip(shape)
                    .background(if (selected) LsColors.primary else Color.Transparent)
                    .clickable(interactionSource = interaction, indication = null, role = Role.Tab) { onSelect(index) }
                    .semantics {
                        this.selected = selected
                        role = Role.Tab
                    }
                    .padding(horizontal = LsSpacing.m, vertical = LsSpacing.s),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    label,
                    style = LsType.bodyStrong,
                    color = if (selected) LsColors.onPrimary else LsColors.textSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}
