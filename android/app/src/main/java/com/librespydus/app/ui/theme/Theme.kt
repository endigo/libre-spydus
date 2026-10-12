package com.librespydus.app.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/*
 * design/tokens.json as Compose values. Keep this file in sync with the tokens (the iOS app reads
 * the same file). Screens use these directly (LsColors.surface, LsType.title, LsSpacing.l, …);
 * MaterialTheme is mapped onto them so stock Material 3 components fit in.
 */

object LsColors {
    val background = Color(0xFF2B2D42)
    val surface = Color(0xFF35374E)
    val surfaceElevated = Color(0xFF3E405A)
    val field = Color(0xFF3A3C55)
    val divider = Color(0xFF44465F)
    val textPrimary = Color(0xFFFFFFFF)
    val textSecondary = Color(0xFFA4A7C1)
    val textMuted = Color(0xFF7D809B)
    val primary = Color(0xFFF2665E)
    val primaryGradientStart = Color(0xFFF7806F)
    val primaryGradientEnd = Color(0xFFEF5A5C)
    val onPrimary = Color(0xFFFFFFFF)
    val accent = Color(0xFF4B5BD6)
    val onAccent = Color(0xFFFFFFFF)
    val sheet = Color(0xFFFFFFFF)
    val onSheet = Color(0xFF24263A)
    val onSheetSecondary = Color(0xFF5F627A)
    val chipOnSheet = Color(0xFFEEF0F7)
    val tabBar = Color(0xFFFFFFFF)
    val tabActive = Color(0xFF2B2D42)
    val tabInactive = Color(0xFFB4B6C6)
    val success = Color(0xFF3CC48A)
    val warning = Color(0xFFF5A524)
    val danger = Color(0xFFF2665E)
    val digital = Color(0xFF2EC4B6)
    val genre = listOf(
        Color(0xFFF26B5B), Color(0xFF9B59D0), Color(0xFF4F6BE3), Color(0xFFF29F4B),
        Color(0xFF2EC4B6), Color(0xFFE85D9B), Color(0xFF5DB85C), Color(0xFF7A6FF0),
    )

    /** Coral pill gradient (primary button). */
    val primaryGradient: Brush get() = Brush.horizontalGradient(listOf(primaryGradientStart, primaryGradientEnd))

    /** A stable palette colour for a string (genre tiles, cover placeholders). */
    fun genreColor(key: String): Color = genre[Math.floorMod(key.hashCode(), genre.size)]
}

object LsRadius {
    val card = 16.dp
    val cover = 10.dp
    val field = 14.dp
    val tile = 18.dp
    val sheet = 28.dp
    val pill = 999.dp
}

object LsSpacing {
    val xs = 4.dp
    val s = 8.dp
    val m = 12.dp
    val l = 16.dp
    val xl = 24.dp
    val xxl = 32.dp
}

/** Type ramp (sp, so it follows the system font scale). System font = Roboto. */
object LsType {
    val greeting = TextStyle(fontSize = 28.sp, lineHeight = 34.sp, fontWeight = FontWeight.SemiBold)
    val title = TextStyle(fontSize = 22.sp, lineHeight = 28.sp, fontWeight = FontWeight.SemiBold)
    val section = TextStyle(fontSize = 18.sp, lineHeight = 24.sp, fontWeight = FontWeight.SemiBold)
    val body = TextStyle(fontSize = 15.sp, lineHeight = 21.sp, fontWeight = FontWeight.Normal)
    val bodyStrong = TextStyle(fontSize = 15.sp, lineHeight = 21.sp, fontWeight = FontWeight.SemiBold)
    val caption = TextStyle(fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight.Normal)
    val tab = TextStyle(fontSize = 11.sp, lineHeight = 14.sp, fontWeight = FontWeight.Medium)
}

object LsCover {
    /** width / height */
    const val ASPECT = 0.68f
    val shelfWidth = 120.dp
    val smallWidth = 72.dp
    val detailWidth = 140.dp

    fun height(width: Dp): Dp = width / ASPECT
}

object LsTabBar {
    val height = 64.dp
    val centerButtonDiameter = 64.dp
    val centerButtonLift = 22.dp
}

/** tokens.shadow: elevation-style approximations (Compose shadows take one blur/offset). */
object LsShadow {
    val card = 12.dp
    val cardColor = Color.Black.copy(alpha = 0.25f)
    val tabCenter = 10.dp
    val tabCenterColor = Color(0xFF2B2D42).copy(alpha = 0.18f)
}

private val LsColorScheme = darkColorScheme(
    primary = LsColors.primary,
    onPrimary = LsColors.onPrimary,
    secondary = LsColors.accent,
    onSecondary = LsColors.onAccent,
    tertiary = LsColors.digital,
    background = LsColors.background,
    onBackground = LsColors.textPrimary,
    surface = LsColors.background,
    onSurface = LsColors.textPrimary,
    surfaceVariant = LsColors.surface,
    onSurfaceVariant = LsColors.textSecondary,
    surfaceContainerLowest = LsColors.background,
    surfaceContainerLow = LsColors.surface,
    surfaceContainer = LsColors.surface,
    surfaceContainerHigh = LsColors.surfaceElevated,
    surfaceContainerHighest = LsColors.surfaceElevated,
    outline = LsColors.divider,
    outlineVariant = LsColors.divider,
    error = LsColors.danger,
    onError = LsColors.onPrimary,
)

private val LsMaterialTypography = Typography(
    displaySmall = LsType.greeting,
    headlineSmall = LsType.title,
    titleLarge = LsType.title,
    titleMedium = LsType.section,
    titleSmall = LsType.bodyStrong,
    bodyLarge = LsType.body,
    bodyMedium = LsType.body,
    bodySmall = LsType.caption,
    labelLarge = LsType.bodyStrong,
    labelMedium = LsType.caption,
    labelSmall = LsType.tab,
)

private val LsShapes = Shapes(
    small = RoundedCornerShape(LsRadius.cover),
    medium = RoundedCornerShape(LsRadius.field),
    large = RoundedCornerShape(LsRadius.card),
    extraLarge = RoundedCornerShape(LsRadius.sheet),
)

@Composable
fun LibreSpydusTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = LsColorScheme,
        typography = LsMaterialTypography,
        shapes = LsShapes,
        content = content,
    )
}
