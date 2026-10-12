package com.librespydus.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsCover
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/** A trailing text action on a [BookRowCard] ("Cancel", "Edit"). */
data class CardAction(val label: String, val onClick: () -> Unit, val color: Color = LsColors.primary)

/**
 * The Borrowed-style card (§6, §7): a `surface` card whose cover rises above its top edge, with
 * title, "By author", a meta line and coral trailing text actions. [highlighted] marks e.g.
 * ready-for-pickup reservations; [badge] is a small label above the title.
 */
@Composable
fun BookRowCard(
    title: String,
    coverUrl: String?,
    modifier: Modifier = Modifier,
    byLine: String? = null,
    meta: String? = null,
    metaColor: Color = LsColors.textSecondary,
    badge: String? = null,
    badgeColor: Color = LsColors.success,
    highlighted: Boolean = false,
    actions: List<CardAction> = emptyList(),
    onClick: (() -> Unit)? = null,
) {
    val shape = RoundedCornerShape(LsRadius.card)
    val rise = 14.dp
    Box(modifier.fillMaxWidth()) {
        Box(
            Modifier
                .matchParentSize()
                .padding(top = rise)
                .clip(shape)
                .background(if (highlighted) LsColors.surfaceElevated else LsColors.surface)
                .then(if (highlighted) Modifier.border(1.dp, badgeColor.copy(alpha = 0.6f), shape) else Modifier)
                .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier),
        )
        Row(
            Modifier.padding(start = LsSpacing.l, end = LsSpacing.l, bottom = LsSpacing.l),
            verticalAlignment = Alignment.Top,
        ) {
            CoverImage(url = coverUrl, title = title, width = LsCover.smallWidth)
            Spacer(Modifier.width(LsSpacing.l))
            Column(
                Modifier
                    .weight(1f)
                    .padding(top = rise + LsSpacing.m),
                verticalArrangement = Arrangement.spacedBy(LsSpacing.xs),
            ) {
                if (badge != null) {
                    Text(badge, style = LsType.caption, color = badgeColor, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                Text(title, style = LsType.bodyStrong, color = LsColors.textPrimary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (byLine != null) Text(byLine, style = LsType.caption, color = LsColors.textSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (meta != null) {
                    Spacer(Modifier.height(LsSpacing.xs))
                    Text(meta, style = LsType.caption, color = metaColor, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
            }
            if (actions.isNotEmpty()) {
                Column(
                    Modifier.padding(top = rise + LsSpacing.m, start = LsSpacing.s),
                    horizontalAlignment = Alignment.End,
                    verticalArrangement = Arrangement.spacedBy(LsSpacing.s),
                ) {
                    actions.forEach { action ->
                        Text(
                            action.label,
                            style = LsType.bodyStrong,
                            color = action.color,
                            modifier = Modifier
                                .clip(RoundedCornerShape(LsRadius.pill))
                                .clickable(role = Role.Button, onClick = action.onClick)
                                .padding(horizontal = LsSpacing.s, vertical = LsSpacing.xs),
                        )
                    }
                }
            }
        }
    }
}
