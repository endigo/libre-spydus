package com.librespydus.app.feature.onboarding

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.librespydus.app.core.Copy
import com.librespydus.app.ui.theme.LsColors
import com.librespydus.app.ui.theme.LsIcons
import com.librespydus.app.ui.theme.LsRadius
import com.librespydus.app.ui.theme.LsSpacing
import com.librespydus.app.ui.theme.LsType

/*
 * Small pieces shared by the account-side screens (A2: onboarding, Profile, Borrowed, Favourites).
 * Kept here, not in ui/components, so they never clash with other features' helpers.
 */

/** Extra Material icons (24dp grid) the account screens need beyond [LsIcons]. */
object AccountIcons {
    val SwapHoriz = icon("swap_horiz", "M6.99,11L3,15l3.99,4v-3H14v-2H6.99v-3zM21,9l-3.99,-4v3H10v2h7.01v3L21,9z")
    val Logout = icon("logout", "M17,7l-1.41,1.41L18.17,11H8v2h10.17l-2.58,2.58L17,17l5,-5zM4,5h8V3H4c-1.1,0 -2,0.9 -2,2v14c0,1.1 0.9,2 2,2h8v-2H4V5z")
    val Login = icon("login", "M11,7L9.6,8.4l2.6,2.6H2v2h10.2l-2.6,2.6L11,17l5,-5L11,7zM20,19h-8v2h8c1.1,0 2,-0.9 2,-2V5c0,-1.1 -0.9,-2 -2,-2h-8v2h8V19z")
    val VerifiedUser = icon("verified_user", "M12,1L3,5v6c0,5.55 3.84,10.74 9,12 5.16,-1.26 9,-6.45 9,-12V5l-9,-4zM10,17l-4,-4 1.41,-1.41L10,14.17l6.59,-6.59L18,9l-8,8z")
    val ShieldOff = icon("gpp_bad", "M12,2L4,5v6.09c0,5.05 3.41,9.76 8,10.91c4.59,-1.15 8,-5.86 8,-10.91V5L12,2zM15.5,14.09l-1.41,1.41L12,13.42L9.91,15.5L8.5,14.09L10.59,12L8.5,9.91L9.91,8.5L12,10.59l2.09,-2.09l1.41,1.41L13.41,12L15.5,14.09z")
    val Info = icon("info", "M12,2C6.48,2 2,6.48 2,12s4.48,10 10,10 10,-4.48 10,-10S17.52,2 12,2zM13,17h-2v-6h2v6zM13,9h-2V7h2v2z")
    val Warning = icon("warning", "M1,21h22L12,2 1,21zM13,18h-2v-2h2v2zM13,14h-2v-4h2v4z")
    val CheckCircle = icon("check_circle", "M12,2C6.48,2 2,6.48 2,12s4.48,10 10,10 10,-4.48 10,-10S17.52,2 12,2zM10,17l-5,-5 1.41,-1.41L10,14.17l7.59,-7.59L19,8l-9,9z")
    val WifiOff = icon("wifi_off", "M23.64,7c-0.45,-0.34 -4.93,-4 -11.64,-4 -1.5,0 -2.89,0.19 -4.15,0.48L18.18,13.8 23.64,7zM17.04,15.22L3.27,1.44 2,2.72l2.05,2.06C1.91,5.76 0.59,6.82 0.36,7l11.63,14.49 0.01,0.01 0.01,-0.01 3.9,-4.86 3.32,3.32 1.27,-1.27 -3.46,-3.46z")
    val Place = icon("place", "M12,2C8.13,2 5,5.13 5,9c0,5.25 7,13 7,13s7,-7.75 7,-13c0,-3.87 -3.13,-7 -7,-7zM12,11.5c-1.38,0 -2.5,-1.12 -2.5,-2.5s1.12,-2.5 2.5,-2.5 2.5,1.12 2.5,2.5 -1.12,2.5 -2.5,2.5z")
    val AccountCircle = icon("account_circle", "M12,2C6.48,2 2,6.48 2,12s4.48,10 10,10 10,-4.48 10,-10S17.52,2 12,2zM12,6c1.93,0 3.5,1.57 3.5,3.5S13.93,13 12,13s-3.5,-1.57 -3.5,-3.5S10.07,6 12,6zM12,20c-2.03,0 -4.43,-0.82 -6.14,-2.88C7.55,15.8 9.68,15 12,15s4.45,0.8 6.14,2.12C16.43,19.18 14.03,20 12,20z")
    val Build = icon("build", "M22.7,19l-9.1,-9.1c0.9,-2.3 0.4,-5 -1.5,-6.9 -2,-2 -5,-2.4 -7.4,-1.3L9,6 6,9 1.6,4.7C0.4,7.1 0.9,10.1 2.9,12.1c1.9,1.9 4.6,2.4 6.9,1.5l9.1,9.1c0.4,0.4 1,0.4 1.4,0l2.3,-2.3c0.5,-0.4 0.5,-1.1 0.1,-1.4z")
    val Schedule = icon("schedule", "M11.99,2C6.47,2 2,6.48 2,12s4.47,10 9.99,10C17.52,22 22,17.52 22,12S17.52,2 11.99,2zM12,20c-4.42,0 -8,-3.58 -8,-8s3.58,-8 8,-8 8,3.58 8,8 -3.58,8 -8,8zM12.5,7H11v6l5.25,3.15 0.75,-1.23 -4.5,-2.67z")
    val Shipping = icon("local_shipping", "M20,8h-3V4H3c-1.1,0 -2,0.9 -2,2v11h2c0,1.66 1.34,3 3,3s3,-1.34 3,-3h6c0,1.66 1.34,3 3,3s3,-1.34 3,-3h2v-5l-3,-4zM6,18.5c-0.83,0 -1.5,-0.67 -1.5,-1.5s0.67,-1.5 1.5,-1.5 1.5,0.67 1.5,1.5 -0.67,1.5 -1.5,1.5zM19.5,9.5l1.96,2.5H17V9.5h2.5zM18,18.5c-0.83,0 -1.5,-0.67 -1.5,-1.5s0.67,-1.5 1.5,-1.5 1.5,0.67 1.5,1.5 -0.67,1.5 -1.5,1.5z")
    val Event = icon("event", "M17,12h-5v5h5v-5zM16,1v2H8V1H6v2H5c-1.11,0 -1.99,0.9 -1.99,2L3,19c0,1.1 0.89,2 2,2h14c1.1,0 2,-0.9 2,-2V5c0,-1.1 -0.9,-2 -2,-2h-1V1h-2zM19,19H5V8h14v11z")
    val Remove = icon("remove", "M19,13H5v-2h14v2z")
    val BookmarkRemove = icon("bookmark_remove", "M17,11v6.97l-5,-2.14l-5,2.14V5h6V3H7C5.9,3 5,3.9 5,5v16l7,-3l7,3V11H17zM21,7h-6V5h6V7z")
    val OpenInNew = icon("open_in_new", "M19,19H5V5h7V3H5c-1.11,0 -2,0.9 -2,2v14c0,1.1 0.89,2 2,2h14c1.1,0 2,-0.9 2,-2v-7h-2v7zM14,3v2h3.59l-9.83,9.83 1.41,1.41L19,6.41V10h2V3h-7z")
    val Sync = icon("sync", "M12,4V1L8,5l4,4V6c3.31,0 6,2.69 6,6 0,1.01 -0.25,1.97 -0.7,2.8l1.46,1.46C19.54,15.03 20,13.57 20,12c0,-4.42 -3.58,-8 -8,-8zM12,18c-3.31,0 -6,-2.69 -6,-6 0,-1.01 0.25,-1.97 0.7,-2.8L5.24,7.74C4.46,8.97 4,10.43 4,12c0,4.42 3.58,8 8,8v3l4,-4 -4,-4v3z")

    private fun icon(name: String, path: String): ImageVector =
        ImageVector.Builder(name, 24.dp, 24.dp, 24f, 24f)
            .addPath(pathData = addPathNodes(path), fill = SolidColor(Color.Black))
            .build()
}

/** Banner tones, as on iOS: success (green), info (blue), error (coral). */
enum class AccountBannerTone { SUCCESS, INFO, ERROR }

/** A tinted message box: login errors, the demo hint, Borrowed results, refresh failures. [onClick] dismisses it. */
@Composable
fun AccountBanner(
    message: String,
    modifier: Modifier = Modifier,
    tone: AccountBannerTone = AccountBannerTone.INFO,
    onClick: (() -> Unit)? = null,
) {
    val color = when (tone) {
        AccountBannerTone.SUCCESS -> LsColors.success
        AccountBannerTone.INFO -> LsColors.accent
        AccountBannerTone.ERROR -> LsColors.danger
    }
    val icon = when (tone) {
        AccountBannerTone.SUCCESS -> AccountIcons.CheckCircle
        AccountBannerTone.INFO -> AccountIcons.Info
        AccountBannerTone.ERROR -> AccountIcons.Warning
    }
    val shape = RoundedCornerShape(LsRadius.field)
    Row(
        modifier
            .fillMaxWidth()
            .clip(shape)
            .background(color.copy(alpha = 0.22f))
            .border(1.dp, color.copy(alpha = 0.6f), shape)
            .then(if (onClick != null) Modifier.clickable(onClickLabel = Copy.Common.dismiss, onClick = onClick) else Modifier)
            .padding(LsSpacing.m),
        horizontalArrangement = Arrangement.spacedBy(LsSpacing.s),
        verticalAlignment = Alignment.Top,
    ) {
        Icon(icon, contentDescription = null, tint = LsColors.textPrimary, modifier = Modifier.size(20.dp))
        Text(message, style = LsType.body, color = LsColors.textPrimary, modifier = Modifier.weight(1f))
    }
}

/** A `surface` card like Borrowed's, holding a column of rows (Profile). */
@Composable
fun AccountCard(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(LsRadius.card))
            .background(LsColors.surface)
            .padding(LsSpacing.l),
        verticalArrangement = Arrangement.spacedBy(LsSpacing.m),
        content = content,
    )
}

@Composable
fun AccountDivider(modifier: Modifier = Modifier) {
    Spacer(
        modifier
            .fillMaxWidth()
            .height(1.dp)
            .background(LsColors.divider),
    )
}

/** A section title above a card ("Account", "About"). */
@Composable
fun AccountSectionHeader(title: String, modifier: Modifier = Modifier) {
    Text(title, style = LsType.section, color = LsColors.textPrimary, modifier = modifier.semantics { heading() })
}

/** A full-width text action inside a card ("Change library", "Sign out"). */
@Composable
fun AccountRowButton(
    title: String,
    icon: ImageVector,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    tint: Color = LsColors.primary,
) {
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = 44.dp)
            .clip(RoundedCornerShape(LsRadius.field))
            .clickable(role = Role.Button, onClickLabel = title, onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, tint = tint, modifier = Modifier.size(22.dp))
        Spacer(Modifier.width(LsSpacing.m))
        Text(title, style = LsType.bodyStrong, color = tint, modifier = Modifier.weight(1f))
        Icon(LsIcons.ChevronRight, contentDescription = null, tint = LsColors.textMuted, modifier = Modifier.size(20.dp))
    }
}
