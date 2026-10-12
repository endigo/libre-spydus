package com.librespydus.app.feature.detail

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.librespydus.app.core.AppState
import com.librespydus.app.core.BookRef
import com.librespydus.app.ui.components.EmptyStateView
import com.librespydus.app.ui.components.LsTopBar
import com.librespydus.app.ui.nav.AppNavigator

/** Book detail (`detail(BookRef)`, §6, owned by A3). Wave-1 stub. */
@Composable
fun BookDetailScreen(book: BookRef, appState: AppState, navigator: AppNavigator, modifier: Modifier = Modifier) {
    Column(modifier.fillMaxSize().statusBarsPadding()) {
        LsTopBar(onBack = navigator::back)
        EmptyStateView(
            title = book.title.ifBlank { "Book" },
            message = "Cover, availability, Borrow Now and the synopsis sheet will appear here.",
        )
    }
}
