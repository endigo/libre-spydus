package com.librespydus.app.ui.nav

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContentTransitionScope.SlideDirection
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.captionBar
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.BottomSheetDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import com.librespydus.app.BuildConfig
import com.librespydus.app.core.AppState
import com.librespydus.app.core.Route
import com.librespydus.app.core.Tab
import com.librespydus.app.feature.borrowed.BorrowedScreen
import com.librespydus.app.feature.detail.BookDetailScreen
import com.librespydus.app.feature.favourites.FavouritesScreen
import com.librespydus.app.feature.favourites.PicksOverlays
import com.librespydus.app.feature.featured.CollectionScreen
import com.librespydus.app.feature.home.HomeScreen
import com.librespydus.app.feature.onboarding.BiometricOfferDialog
import com.librespydus.app.feature.onboarding.LibraryPickerScreen
import com.librespydus.app.feature.onboarding.LoginSheet
import com.librespydus.app.feature.onboarding.VerificationScreen
import com.librespydus.app.feature.profile.ProfileScreen
import com.librespydus.app.feature.search.ListResultsScreen
import com.librespydus.app.feature.search.SearchResultsScreen
import com.librespydus.app.feature.search.SearchScreen
import com.librespydus.app.ui.components.ComponentGallery
import com.librespydus.app.ui.components.LoadingView
import com.librespydus.app.ui.components.LsSidebar
import com.librespydus.app.ui.components.LsTabBar
import com.librespydus.app.ui.components.tabBarContentPadding
import com.librespydus.app.ui.theme.AppLayout
import com.librespydus.app.ui.theme.LocalAppLayout
import com.librespydus.app.ui.theme.LsColors
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch

/**
 * The app's root: splash until state is restored, onboarding (library picker) without a library,
 * else the five tabs. Sheets and overlays are driven by [AppState] flags: library picker
 * ([AppState.libraryPickerVisible]), login ([AppState.loginNeeded]), verification
 * ([AppState.verificationNeeded]) and the DEBUG components gallery.
 *
 * The window's size here decides the layout (docs/native-apps.md §6.1): wide (a sidebar, grids, a
 * two-column book page) when it is at least 600 × 480 dp, else the compact phone layout — so a
 * narrow split screen or a small freeform window on a tablet is compact too.
 */
@Composable
fun LibreSpydusRoot(appState: AppState) {
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val layout = AppLayout.of(maxWidth, maxHeight)
        CompositionLocalProvider(LocalAppLayout provides layout) {
            RootContent(appState)
        }
    }
}

@Composable
private fun RootContent(appState: AppState) {
    val ready by appState.ready.collectAsStateWithLifecycle()
    val library by appState.library.collectAsStateWithLifecycle()
    val pickerVisible by appState.libraryPickerVisible.collectAsStateWithLifecycle()
    val loginNeeded by appState.loginNeeded.collectAsStateWithLifecycle()
    val verificationNeeded by appState.verificationNeeded.collectAsStateWithLifecycle()
    val galleryVisible by appState.galleryVisible.collectAsStateWithLifecycle()

    CompositionLocalProvider(LocalAppState provides appState) {
        Box(
            Modifier
                .fillMaxSize()
                .background(LsColors.background),
        ) {
            when {
                !ready -> LoadingView()
                library == null -> LibraryPickerScreen(appState, onDismiss = null)
                else -> MainTabs(appState)
            }
            // My Picks (§5.11): save/remove confirmations and the copy offer, over the tabs.
            if (ready && library != null) PicksOverlays(appState)
            // After a typed sign-in: the fingerprint-or-face offer (§5.14), before the copy offer.
            if (ready && library != null) BiometricOfferDialog(appState)

            AnimatedVisibility(
                visible = ready && library != null && pickerVisible,
                enter = slideInVertically { it } + fadeIn(),
                exit = slideOutVertically { it } + fadeOut(),
            ) {
                BackHandler { appState.dismissLibraryPicker() }
                // Surface consumes touches so the tabs underneath stay inert.
                Surface(color = LsColors.background, modifier = Modifier.fillMaxSize()) {
                    LibraryPickerScreen(appState, onDismiss = appState::dismissLibraryPicker)
                }
            }

            if (BuildConfig.DEBUG && galleryVisible) {
                BackHandler { appState.dismissGallery() }
                Surface(color = LsColors.background, modifier = Modifier.fillMaxSize()) {
                    ComponentGallery(onClose = appState::dismissGallery)
                }
            }
        }

        if (ready && library != null && loginNeeded) LoginSheetHost(appState)
        if (ready && library != null && verificationNeeded) VerificationSheetHost(appState)
    }
}

@Composable
private fun MainTabs(appState: AppState) {
    val selected by appState.selectedTab.collectAsStateWithLifecycle()
    val stateHolder = rememberSaveableStateHolder()
    val reselects = remember { MutableSharedFlow<Tab>(extraBufferCapacity = 1) }

    // Back at a tab's root returns to Home (registered first, so a tab's NavHost wins while it
    // has somewhere to pop to).
    BackHandler(enabled = selected != Tab.HOME) { appState.selectTab(Tab.HOME) }

    val onSelect: (Tab) -> Unit = { tab -> if (tab == selected) reselects.tryEmit(tab) else appState.selectTab(tab) }
    val tabs: @Composable () -> Unit = {
        // One NavHost per tab; switching tabs saves the hidden tab's stack and restores it.
        key(selected) {
            stateHolder.SaveableStateProvider(selected.name) {
                TabNavHost(selected, appState, reselects)
            }
        }
    }

    // A freeform window's caption bar (§6.1): the tabs and the sidebar start below it.
    if (LocalAppLayout.current.isWide) {
        Row(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.captionBar),
        ) {
            LsSidebar(appState, selected = selected, onSelect = onSelect)
            Box(
                Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .clipToBounds()
                    .padding(bottom = tabBarContentPadding()),
            ) {
                tabs()
            }
        }
    } else {
        Box(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.captionBar),
        ) {
            Box(
                Modifier
                    .fillMaxSize()
                    .padding(bottom = tabBarContentPadding()),
            ) {
                tabs()
            }
            LsTabBar(
                selected = selected,
                onSelect = onSelect,
                modifier = Modifier.align(Alignment.BottomCenter),
            )
        }
    }
}

@Composable
private fun TabNavHost(tab: Tab, appState: AppState, reselects: Flow<Tab>) {
    val navController = rememberNavController()
    val navigator = remember(navController) { TabNavigator(navController, appState) }
    val pending by appState.pendingNavigation.collectAsStateWithLifecycle()

    LaunchedEffect(pending) {
        appState.takePendingNavigation(tab)?.let { (_, route, fromRoot) ->
            // The tab's own root (AppState.openBorrowed) pops back to it instead of pushing another;
            // a tapped alert's book page goes straight over the root, so Back lands on Your Books.
            val root = route == Route.root(tab)
            if (root || fromRoot) navController.popBackStack(navController.graph.startDestinationId, inclusive = false)
            if (!root) navController.navigate(route)
        }
    }
    LaunchedEffect(navController) {
        // Re-selecting the current tab pops it to its root.
        reselects.collect { if (it == tab) navController.popBackStack(navController.graph.startDestinationId, inclusive = false) }
    }

    NavHost(
        navController = navController,
        startDestination = Route.root(tab),
        enterTransition = { slideIntoContainer(SlideDirection.Start, tween(280)) + fadeIn(tween(280)) },
        exitTransition = { fadeOut(tween(200)) },
        popEnterTransition = { fadeIn(tween(200)) },
        popExitTransition = { slideOutOfContainer(SlideDirection.End, tween(280)) + fadeOut(tween(280)) },
    ) {
        when (tab) {
            Tab.HOME -> composable<Route.Home> { HomeScreen(appState, navigator) }
            Tab.BORROWED -> composable<Route.Borrowed> { BorrowedScreen(appState, navigator) }
            Tab.SEARCH -> composable<Route.Search> { SearchScreen(appState, navigator) }
            Tab.FAVOURITES -> composable<Route.Favourites> { FavouritesScreen(appState, navigator) }
            Tab.PROFILE -> composable<Route.Profile> { ProfileScreen(appState, navigator) }
        }
        composable<Route.Detail>(typeMap = routeTypeMap) { entry ->
            BookDetailScreen(entry.toRoute<Route.Detail>().book, appState, navigator)
        }
        composable<Route.Results>(typeMap = routeTypeMap) { entry ->
            SearchResultsScreen(entry.toRoute<Route.Results>(), appState, navigator)
        }
        composable<Route.Collection> { entry ->
            CollectionScreen(entry.toRoute<Route.Collection>(), appState, navigator)
        }
        composable<Route.ListResults> { entry ->
            ListResultsScreen(entry.toRoute<Route.ListResults>(), appState, navigator)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun LoginSheetHost(appState: AppState) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()
    ModalBottomSheet(
        onDismissRequest = appState::dismissLogin,
        sheetState = sheetState,
        containerColor = LsColors.background,
        contentColor = LsColors.textPrimary,
        dragHandle = { BottomSheetDefaults.DragHandle(color = LsColors.textMuted) },
    ) {
        LoginSheet(
            appState = appState,
            onDismiss = { scope.launch { sheetState.hide() }.invokeOnCompletion { appState.dismissLogin() } },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun VerificationSheetHost(appState: AppState) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()
    ModalBottomSheet(
        onDismissRequest = appState::dismissVerification,
        sheetState = sheetState,
        // The sheet hosts a web view: dragging inside it must scroll the page, not the sheet.
        sheetGesturesEnabled = false,
        containerColor = LsColors.background,
        contentColor = LsColors.textPrimary,
        dragHandle = { BottomSheetDefaults.DragHandle(color = LsColors.textMuted) },
    ) {
        VerificationScreen(
            appState = appState,
            onDismiss = { scope.launch { sheetState.hide() }.invokeOnCompletion { appState.dismissVerification() } },
        )
    }
}
