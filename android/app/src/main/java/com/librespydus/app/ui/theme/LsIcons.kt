package com.librespydus.app.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

/**
 * The app's icons as [ImageVector]s (Material icon geometry, 24dp grid), tinted by `Icon`.
 * Kept in-app instead of pulling the deprecated material-icons artifacts.
 */
object LsIcons {
    val Home = icon("home", "M12,5.69l5,4.5V18h-2v-6H9v6H7v-7.81l5,-4.5M12,3L2,12h3v8h6v-6h2v6h6v-8h3L12,3z")
    val HomeFilled = icon("home_filled", "M10,20v-6h4v6h5v-8h3L12,3 2,12h3v8z")
    val Book = icon("book", "M18,2H6c-1.1,0 -2,0.9 -2,2v16c0,1.1 0.9,2 2,2h12c1.1,0 2,-0.9 2,-2V4c0,-1.1 -0.9,-2 -2,-2zM9,4h2v5l-1,-0.75L9,9V4zM18,20H6V4h1v9l3,-2.25L13,13V4h5V20z")
    val BookFilled = icon("book_filled", "M18,2H6c-1.1,0 -2,0.9 -2,2v16c0,1.1 0.9,2 2,2h12c1.1,0 2,-0.9 2,-2V4c0,-1.1 -0.9,-2 -2,-2zM6,4h5v8l-2.5,-1.5L6,12V4z")
    val Search = icon("search", "M15.5,14h-0.79l-0.28,-0.27C15.41,12.59 16,11.11 16,9.5 16,5.91 13.09,3 9.5,3S3,5.91 3,9.5 5.91,16 9.5,16c1.61,0 3.09,-0.59 4.23,-1.57l0.27,0.28v0.79l5,4.99L20.49,19l-4.99,-5zM9.5,14C7.01,14 5,11.99 5,9.5S7.01,5 9.5,5 14,7.01 14,9.5 11.99,14 9.5,14z")
    val Bookmark = icon("bookmark", "M17,3H7c-1.1,0 -1.99,0.9 -1.99,2L5,21l7,-3 7,3V5c0,-1.1 -0.9,-2 -2,-2zM17,18l-5,-2.18L7,18V5h10v13z")
    val BookmarkFilled = icon("bookmark_filled", "M17,3H7c-1.1,0 -1.99,0.9 -1.99,2L5,21l7,-3 7,3V5c0,-1.1 -0.9,-2 -2,-2z")
    val Person = icon("person", "M12,5.9c1.16,0 2.1,0.94 2.1,2.1s-0.94,2.1 -2.1,2.1S9.9,9.16 9.9,8s0.94,-2.1 2.1,-2.1m0,9c2.97,0 6.1,1.46 6.1,2.1v1.1H5.9V17c0,-0.64 3.13,-2.1 6.1,-2.1M12,4C9.79,4 8,5.79 8,8s1.79,4 4,4 4,-1.79 4,-4 -1.79,-4 -4,-4zM12,13c-2.67,0 -8,1.34 -8,4v3h16v-3c0,-2.66 -5.33,-4 -8,-4z")
    val PersonFilled = icon("person_filled", "M12,12c2.21,0 4,-1.79 4,-4s-1.79,-4 -4,-4 -4,1.79 -4,4 1.79,4 4,4zM12,14c-2.67,0 -8,1.34 -8,4v2h16v-2c0,-2.66 -5.33,-4 -8,-4z")
    val ArrowBack = icon("arrow_back", "M20,11H7.83l5.59,-5.59L12,4l-8,8 8,8 1.41,-1.41L7.83,13H20v-2z", autoMirror = true)
    val Close = icon("close", "M19,6.41L17.59,5 12,10.59 6.41,5 5,6.41 10.59,12 5,17.59 6.41,19 12,13.41 17.59,19 19,17.59 13.41,12z")
    val Refresh = icon("refresh", "M17.65,6.35C16.2,4.9 14.21,4 12,4c-4.42,0 -7.99,3.58 -7.99,8s3.57,8 7.99,8c3.73,0 6.84,-2.55 7.73,-6h-2.08c-0.82,2.33 -3.04,4 -5.65,4 -3.31,0 -6,-2.69 -6,-6s2.69,-6 6,-6c1.66,0 3.14,0.69 4.22,1.78L13,11h7V4l-2.35,2.35z")
    val Error = icon("error", "M11,15h2v2h-2zM11,7h2v6h-2zM11.99,2C6.47,2 2,6.48 2,12s4.47,10 9.99,10C17.52,22 22,17.52 22,12S17.52,2 11.99,2zM12,20c-4.42,0 -8,-3.58 -8,-8s3.58,-8 8,-8 8,3.58 8,8 -3.58,8 -8,8z")
    val ChevronRight = icon("chevron_right", "M10,6L8.59,7.41 13.17,12l-4.58,4.59L10,18l6,-6z", autoMirror = true)
    val ExpandMore = icon("expand_more", "M16.59,8.59L12,13.17 7.41,8.59 6,10l6,6 6,-6z")
    val Check = icon("check", "M9,16.17L4.83,12l-1.42,1.41L9,19 21,7l-1.41,-1.41z")
    val Library = icon("local_library", "M12,11.55C9.64,9.35 6.48,8 3,8v11c3.48,0 6.64,1.35 9,3.55 2.36,-2.19 5.52,-3.55 9,-3.55V8c-3.48,0 -6.64,1.35 -9,3.55zM12,8c1.66,0 3,-1.34 3,-3s-1.34,-3 -3,-3 -3,1.34 -3,3 1.34,3 3,3z")
    val Globe = icon("public", "M12,2C6.48,2 2,6.48 2,12s4.48,10 10,10 10,-4.48 10,-10S17.52,2 12,2zM11,19.93c-3.95,-0.49 -7,-3.85 -7,-7.93 0,-0.62 0.08,-1.21 0.21,-1.79L9,15v1c0,1.1 0.9,2 2,2v1.93zM17.9,17.39c-0.26,-0.81 -1,-1.39 -1.9,-1.39h-1v-3c0,-0.55 -0.45,-1 -1,-1H8v-2h2c0.55,0 1,-0.45 1,-1V7h2c1.1,0 2,-0.9 2,-2v-0.41c2.93,1.19 5,4.06 5,7.41 0,2.08 -0.8,3.97 -2.1,5.39z")
    val Shield = icon("shield", "M12,1L3,5v6c0,5.55 3.84,10.74 9,12 5.16,-1.26 9,-6.45 9,-12V5l-9,-4z")
    val Star = icon("star", "M12,17.27L18.18,21l-1.64,-7.03L22,9.24l-7.19,-0.61L12,2 9.19,8.63 2,9.24l5.46,4.73L5.82,21z")
    val Museum = icon("account_balance", "M4,10v7h3v-7H4zM10,10v7h3v-7h-3zM2,22h19v-3H2v3zM16,10v7h3v-7h-3zM11.5,1L2,6v2h19V6l-9.5,-5z")
    val Science = icon("science", "M19.8,18.4L14,10.67V6.5l1.35,-1.69C15.61,4.48 15.38,4 14.96,4H9.04C8.62,4 8.39,4.48 8.65,4.81L10,6.5v4.17L4.2,18.4C3.71,19.06 4.18,20 5,20h14c0.82,0 1.29,-0.94 0.8,-1.6z")
    val Restaurant = icon("restaurant", "M11,9H9V2H7v7H5V2H3v7c0,2.12 1.66,3.84 3.75,3.97V22h2.5v-9.03C11.34,12.84 13,11.12 13,9V2h-2v7zM16,6v8h2.5v8H21V2c-2.76,0 -5,2.24 -5,4z")
    val Flight = icon("flight", "M21,16v-2l-8,-5V3.5c0,-0.83 -0.67,-1.5 -1.5,-1.5S10,2.67 10,3.5V9l-8,5v2l8,-2.5V19l-2,1.5V22l3.5,-1 3.5,1v-1.5L13,19v-5.5l8,2.5z")
    val Child = icon("child", "M12,2c1.1,0 2,0.9 2,2s-0.9,2 -2,2 -2,-0.9 -2,-2 0.9,-2 2,-2zM15.89,8.11C15.5,7.72 14.83,7 13.53,7h-2.54C8.24,6.99 6,4.75 6,2H4c0,3.16 2.11,5.84 5,6.71V22h2v-6h2v6h2V10.05L18.95,14l1.41,-1.41 -4.47,-4.48z")
    val Theater = icon("theater", "M2,16v2h20v-2H2zM20,6H4c-1.1,0 -2,0.9 -2,2v6h20V8c0,-1.1 -0.9,-2 -2,-2zM4,12V8h16v4H4z")
    val Music = icon("music", "M12,3v10.55c-0.59,-0.34 -1.27,-0.55 -2,-0.55 -2.21,0 -4,1.79 -4,4s1.79,4 4,4 4,-1.79 4,-4V7h4V3h-6z")
    val Heart = icon("heart", "M12,21.35l-1.45,-1.32C5.4,15.36 2,12.28 2,8.5 2,5.42 4.42,3 7.5,3c1.74,0 3.41,0.81 4.5,2.09C13.09,3.81 14.76,3 16.5,3 19.58,3 22,5.42 22,8.5c0,3.78 -3.4,6.86 -8.55,11.54L12,21.35z")

    /** Icon for a Home genre tile by `Genre.key` (§6); unknown keys get a book. */
    fun genre(key: String): ImageVector = when (key.lowercase()) {
        "biography", "biographies", "memoir" -> Person
        "fiction", "novels", "classics" -> BookFilled
        "history" -> Museum
        "science", "technology", "engineering" -> Science
        "children", "kids", "juvenile", "young-adult", "ya" -> Child
        "cooking", "food", "cookery" -> Restaurant
        "travel" -> Flight
        "crime", "mystery", "thriller" -> Search
        "romance" -> Heart
        "music" -> Music
        "film", "movies", "dvd", "drama" -> Theater
        "fantasy", "science-fiction", "scifi" -> Star
        else -> BookFilled
    }

    private fun icon(name: String, path: String, autoMirror: Boolean = false): ImageVector =
        ImageVector.Builder(
            name = name,
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
            autoMirror = autoMirror,
        ).addPath(pathData = addPathNodes(path), fill = SolidColor(Color.Black)).build()
}
