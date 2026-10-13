package com.librespydus.app.feature.featured

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp
import com.librespydus.app.feature.search.BrowseIcons
import com.librespydus.app.ui.theme.LsIcons

/**
 * Icons for the Featured tiles beyond `LsIcons` / `BrowseIcons` (Material "filled" icon geometry,
 * 24dp grid), built like them instead of pulling the material-icons artifacts.
 */
object FeaturedIcons {
    val Celebration = icon("celebration", "m2,22 14,-5 -9,-9zM14.53,12.53 L20.12,6.94a1.25,1.25 0,0 1,1.77 0l0.59,0.59 1.06,-1.06 -0.59,-0.59a2.758,2.758 0,0 0,-3.89 0l-5.59,5.59 1.06,1.06zM10.06,6.88 L9.47,7.47 10.53,8.53 11.12,7.94a2.758,2.758 0,0 0,0 -3.89l-0.59,-0.59 -1.06,1.07 0.59,0.59c0.48,0.48 0.48,1.28 0,1.76zM17.06,11.88 L15.47,13.47 16.53,14.53 18.12,12.94a1.25,1.25 0,0 1,1.77 0l1.61,1.61 1.06,-1.06 -1.61,-1.61a2.758,2.758 0,0 0,-3.89 0zM15.06,5.88 L11.47,9.47 12.53,10.53 16.12,6.94a2.758,2.758 0,0 0,0 -3.89l-1.59,-1.59 -1.06,1.06 1.59,1.59c0.48,0.49 0.48,1.29 0,1.77z")
    val Computer = icon("computer", "M20,18c1.1,0 1.99,-0.9 1.99,-2L22,6c0,-1.1 -0.9,-2 -2,-2H4c-1.1,0 -2,0.9 -2,2v10c0,1.1 0.9,2 2,2H0v2h24v-2h-4zM4,6h16v10H4V6z")
    val Circle = icon("circle", "M12,2C6.47,2 2,6.47 2,12s4.47,10 10,10 10,-4.47 10,-10S17.53,2 12,2z")
    val Extension = icon("extension", "M20.5,11H19V7c0,-1.1 -0.9,-2 -2,-2h-4V3.5a2.5,2.5 0,0 0,-5 0V5H4c-1.1,0 -1.99,0.9 -1.99,2v3.8H3.5c1.49,0 2.7,1.21 2.7,2.7s-1.21,2.7 -2.7,2.7H2V20c0,1.1 0.9,2 2,2h3.8v-1.5c0,-1.49 1.21,-2.7 2.7,-2.7 1.49,0 2.7,1.21 2.7,2.7V22H17c1.1,0 2,-0.9 2,-2v-4h1.5a2.5,2.5 0,0 0,0 -5z")
    val Map = icon("map", "M20.5,3l-0.16,0.03L15,5.1 9,3 3.36,4.9c-0.21,0.07 -0.36,0.25 -0.36,0.48V20.5c0,0.28 0.22,0.5 0.5,0.5l0.16,-0.03L9,18.9l6,2.1 5.64,-1.9c0.21,-0.07 0.36,-0.25 0.36,-0.48V3.5c0,-0.28 -0.22,-0.5 -0.5,-0.5zM15,19l-6,-2.11V5l6,2.11V19z")
    val AccountCircle = icon("account_circle", "M12,2C6.48,2 2,6.48 2,12s4.48,10 10,10 10,-4.48 10,-10S17.52,2 12,2zM12,6c1.93,0 3.5,1.57 3.5,3.5S13.93,13 12,13s-3.5,-1.57 -3.5,-3.5S10.07,6 12,6zM12,20c-2.03,0 -4.43,-0.82 -6.14,-2.88a9.947,9.947 0,0 1,12.28 0C16.43,19.18 14.03,20 12,20z")

    private fun icon(name: String, path: String): ImageVector =
        ImageVector.Builder(
            name = name,
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        ).addPath(pathData = addPathNodes(path), fill = SolidColor(Color.Black)).build()
}

/** The vector for a Featured tile (§5.10 icon mapping). */
fun FeaturedIcon.vector(): ImageVector = when (this) {
    FeaturedIcon.BOOK -> LsIcons.BookFilled
    FeaturedIcon.CELEBRATION -> FeaturedIcons.Celebration
    FeaturedIcon.COMPUTER -> FeaturedIcons.Computer
    FeaturedIcon.CIRCLE -> FeaturedIcons.Circle
    FeaturedIcon.GLOBE -> LsIcons.Globe
    FeaturedIcon.PUZZLE -> FeaturedIcons.Extension
    FeaturedIcon.MAP -> FeaturedIcons.Map
    FeaturedIcon.PERSON_CIRCLE -> FeaturedIcons.AccountCircle
    FeaturedIcon.SPARKLES -> BrowseIcons.AutoAwesome
}
