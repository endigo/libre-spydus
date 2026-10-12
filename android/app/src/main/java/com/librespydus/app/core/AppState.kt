package com.librespydus.app.core

import android.content.Context
import android.util.Log
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.librespydus_core.LibraryInfo
import uniffi.librespydus_core.SpydusException
import java.io.File
import java.net.URI
import java.time.LocalDate
import kotlin.time.Duration.Companion.seconds

/** A destination to open inside a tab's stack (consumed by that tab's NavHost). */
data class PendingNavigation(val tab: Tab, val route: Route)

/**
 * The app's single source of truth (§5.2), one instance held by [com.librespydus.app.LibreSpydusApplication]
 * and read in Compose via `LocalAppState`. Screens observe the StateFlows, call [backend] for data,
 * and send every caught error through [report] so verification/sign-in prompts appear centrally.
 *
 * State-changing operations (library switch, launch options) run serially in the app scope, so a
 * screen leaving composition never cancels one halfway.
 */
class AppState(
    private val scope: CoroutineScope,
    private val libraryStore: LibraryStore,
    private val secrets: SecretStore,
    val favourites: FavouritesStore,
    private val directory: LibraryDirectory = LibraryDirectory.Core,
    private val makeBackend: suspend (origin: String) -> LibraryBackend = { CoreBackend.create(it) },
    private val loadDemoData: (origin: String) -> DemoData = { error("demo data unavailable") },
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    private val ops = Mutex()

    /** One backend (one `SpydusClient`) per origin, kept for the app's lifetime. */
    private val clients = mutableMapOf<String, LibraryBackend>()

    private val _ready = MutableStateFlow(false)
    private val _library = MutableStateFlow<LibraryInfo?>(null)
    private val _backend = MutableStateFlow<LibraryBackend?>(null)
    private val _session = MutableStateFlow<String?>(null)
    private val _accountName = MutableStateFlow<String?>(null)
    private val _isDemo = MutableStateFlow(false)
    private val _selectedTab = MutableStateFlow(Tab.HOME)
    private val _pendingSearch = MutableStateFlow<String?>(null)
    private val _pendingNavigation = MutableStateFlow<PendingNavigation?>(null)
    private val _verificationNeeded = MutableStateFlow(false)
    private val _verificationAttempts = MutableStateFlow(0)
    private val _hasVerificationCookie = MutableStateFlow(false)
    private val _loginNeeded = MutableStateFlow(false)
    private val _libraryPickerVisible = MutableStateFlow(false)
    private val _galleryVisible = MutableStateFlow(false)
    private val _verificationGeneration = MutableStateFlow(0)
    private val _accountGeneration = MutableStateFlow(0)

    /** False until persisted state has been restored (show a splash meanwhile). */
    val ready: StateFlow<Boolean> = _ready.asStateFlow()

    /** The selected library; null → onboarding (library picker). */
    val library: StateFlow<LibraryInfo?> = _library.asStateFlow()

    /** The selected library's backend; non-null whenever [library] is. */
    val backend: StateFlow<LibraryBackend?> = _backend.asStateFlow()

    /** The session token for the selected library, if signed in. Pass it to account calls. */
    val session: StateFlow<String?> = _session.asStateFlow()
    val isSignedIn: StateFlow<Boolean> = session.map { it != null }.stateIn(scope, SharingStarted.Eagerly, false)

    /** From `account(session).name`, e.g. "Test Borrower". */
    val accountName: StateFlow<String?> = _accountName.asStateFlow()

    /** Demo mode (§5.4, debug `ls_demo`): fixture account data, no account network calls. */
    val isDemo: StateFlow<Boolean> = _isDemo.asStateFlow()

    val selectedTab: StateFlow<Tab> = _selectedTab.asStateFlow()

    /** Text typed into Home's search field, for the Search tab to pick up ([consumePendingSearch]). */
    val pendingSearch: StateFlow<String?> = _pendingSearch.asStateFlow()

    /** A route to push onto a tab's stack (launch routes, cross-tab links). */
    val pendingNavigation: StateFlow<PendingNavigation?> = _pendingNavigation.asStateFlow()

    /** The library asked for human verification; the root presents the verification sheet. */
    val verificationNeeded: StateFlow<Boolean> = _verificationNeeded.asStateFlow()

    /** Verifications completed for this library since it was selected (retry once, then give up). */
    val verificationAttempts: StateFlow<Int> = _verificationAttempts.asStateFlow()

    /** A verification cookie is stored for the selected library (Profile "Verified on this device"). */
    val hasVerificationCookie: StateFlow<Boolean> = _hasVerificationCookie.asStateFlow()

    /** Sign-in requested (session expired, or a signed-out action); the root presents the login sheet. */
    val loginNeeded: StateFlow<Boolean> = _loginNeeded.asStateFlow()

    /** The library picker over the tabs (Profile → Change library). */
    val libraryPickerVisible: StateFlow<Boolean> = _libraryPickerVisible.asStateFlow()

    /** DEBUG components gallery (`ls_route gallery`). */
    val galleryVisible: StateFlow<Boolean> = _galleryVisible.asStateFlow()

    /** Bumped after a verification completes or is cleared: screens re-run failed loads. */
    val verificationGeneration: StateFlow<Int> = _verificationGeneration.asStateFlow()

    /** Bumped on sign-in, sign-out, session expiry and library change: account screens reload. */
    val accountGeneration: StateFlow<Int> = _accountGeneration.asStateFlow()

    /** The selected library's favourites, newest first. */
    val currentFavourites: StateFlow<List<BookRef>> =
        combine(library, favourites.all) { lib, all -> lib?.let { all[it.origin] }.orEmpty() }
            .stateIn(scope, SharingStarted.Eagerly, emptyList())

    /** Restores the persisted library, session and favourites. Called once at app start. */
    fun start() {
        scope.launch {
            ops.withLock {
                favourites.load()
                libraryStore.get()?.let { activate(it) }
                _ready.value = true
            }
        }
    }

    // region Library

    suspend fun selectLibrary(info: LibraryInfo) = serial {
        libraryStore.set(info)
        activate(info)
        _libraryPickerVisible.value = false
    }

    /** Civica's directory for the picker (a public GET; throws `SpydusException`). */
    suspend fun listLibraries(includeTest: Boolean = false): List<LibraryInfo> = directory.list(includeTest)

    fun showLibraryPicker() {
        _libraryPickerVisible.value = true
    }

    fun dismissLibraryPicker() {
        _libraryPickerVisible.value = false
    }

    /** Builds (or reuses) the origin's backend and loads that origin's session and cookie. */
    private suspend fun activate(info: LibraryInfo) {
        val origin = info.origin.trimEnd('/')
        val core = clients[origin] ?: try {
            makeBackend(origin).also { clients[origin] = it }
        } catch (e: Exception) {
            Log.w(TAG, "can't create a client for $origin", e)
            return
        }
        val cookie = withContext(io) { secrets.get(verifyKey(origin)) }
        core.setVerificationCookie(cookie)
        val demo = _isDemo.value
        val backend = if (demo) DemoBackend(core, withContext(io) { loadDemoData(origin) }) else core

        _library.value = info
        _backend.value = backend
        _hasVerificationCookie.value = cookie != null
        _verificationNeeded.value = false
        _verificationAttempts.value = 0
        _loginNeeded.value = false
        _accountName.value = null
        _session.value = if (demo) DemoBackend.SESSION else withContext(io) { secrets.get(sessionKey(origin)) }
        _accountGeneration.update { it + 1 }
        _verificationGeneration.update { it + 1 }
        if (_session.value != null) scope.launch { refreshAccount() }
    }

    // endregion
    // region Account

    /**
     * Signs in to the selected library and stores the session (Keystore-encrypted, per origin;
     * demo sessions stay in memory). Throws `SpydusException` — show it with [UserFacingError].
     */
    suspend fun signIn(username: String, password: String) {
        val backend = _backend.value ?: throw IllegalStateException("No library selected")
        val token = try {
            backend.login(username.trim(), password)
        } catch (e: SpydusException) {
            report(e)
            throw e
        }
        if (backend !== _backend.value) return // the library changed meanwhile
        if (!backend.isDemo) withContext(io) { secrets.put(sessionKey(backend.origin), token) }
        _session.value = token
        _loginNeeded.value = false
        _accountGeneration.update { it + 1 }
        refreshAccount()
    }

    suspend fun signOut() {
        val origin = _backend.value?.origin ?: return
        _session.value = null
        _accountName.value = null
        withContext(io) { secrets.put(sessionKey(origin), null) }
        _accountGeneration.update { it + 1 }
    }

    /** Reloads [accountName] (errors are routed through [report] and otherwise ignored). */
    suspend fun refreshAccount() {
        val backend = _backend.value ?: return
        val session = _session.value ?: return
        try {
            val links = backend.account(session)
            if (backend === _backend.value && session == _session.value) _accountName.value = links.name
        } catch (e: SpydusException) {
            report(e)
        }
    }

    fun requestLogin() {
        _loginNeeded.value = true
    }

    fun dismissLogin() {
        _loginNeeded.value = false
    }

    // endregion
    // region Errors & verification

    /**
     * Routes a failed call (§5.2): `HumanVerificationRequired` presents verification, `NotLoggedIn`
     * clears the session and presents login. Returns what the screen should show.
     */
    fun report(error: Throwable): UserFacingError {
        when (error) {
            is SpydusException.HumanVerificationRequired -> _verificationNeeded.value = true
            is SpydusException.NotLoggedIn -> {
                val origin = _backend.value?.origin
                if (_session.value != null) {
                    _session.value = null
                    _accountName.value = null
                    _accountGeneration.update { it + 1 }
                    if (origin != null) scope.launch(io) { secrets.put(sessionKey(origin), null) }
                }
                _loginNeeded.value = true
            }
        }
        return UserFacingError.from(error, _library.value?.name)
    }

    /**
     * The user completed the library's verification in the web view: [cookie] is
     * `aws-waf-token=<value>`. Stored (encrypted) for this origin, applied to the client, and
     * [verificationGeneration] bumped so the failed screen retries.
     */
    suspend fun completeVerification(cookie: String) {
        val backend = _backend.value ?: return
        backend.setVerificationCookie(cookie)
        withContext(io) { secrets.put(verifyKey(backend.origin), cookie) }
        _hasVerificationCookie.value = true
        _verificationNeeded.value = false
        _verificationAttempts.update { it + 1 }
        _verificationGeneration.update { it + 1 }
    }

    suspend fun clearVerification() {
        val backend = _backend.value ?: return
        backend.setVerificationCookie(null)
        withContext(io) { secrets.put(verifyKey(backend.origin), null) }
        _hasVerificationCookie.value = false
        _verificationGeneration.update { it + 1 }
    }

    fun requestVerification() {
        _verificationNeeded.value = true
    }

    fun dismissVerification() {
        _verificationNeeded.value = false
    }

    // endregion
    // region Favourites

    fun isFavourite(id: String): Boolean = currentFavourites.value.any { it.id == id }

    /** Returns true if [book] is now saved. No-op (false) without a library. */
    suspend fun toggleFavourite(book: BookRef): Boolean {
        val origin = _library.value?.origin ?: return false
        return favourites.toggle(origin, book)
    }

    suspend fun removeFavourite(id: String) {
        val origin = _library.value?.origin ?: return
        favourites.remove(origin, id)
    }

    // endregion
    // region Navigation

    fun selectTab(tab: Tab) {
        _selectedTab.value = tab
    }

    /** Home's search field → the Search tab, with [query] prefilled (may be blank: just focus). */
    fun openSearch(query: String = "") {
        _pendingSearch.value = query
        _selectedTab.value = Tab.SEARCH
    }

    /** Returns and clears the pending search (the Search screen calls it once it shows). */
    fun consumePendingSearch(): String? = _pendingSearch.value.also { _pendingSearch.value = null }

    /** Switches to [tab] and pushes [route] onto its stack. */
    fun navigate(tab: Tab, route: Route) {
        _pendingNavigation.value = PendingNavigation(tab, route)
        _selectedTab.value = tab
    }

    fun consumePendingNavigation(tab: Tab): Route? {
        val pending = _pendingNavigation.value?.takeIf { it.tab == tab } ?: return null
        _pendingNavigation.value = null
        return pending.route
    }

    fun dismissGallery() {
        _galleryVisible.value = false
    }

    // endregion
    // region Debug launch options

    /** Applies DEBUG launch hooks (§5.6) after start-up restore; see [LaunchOptions]. */
    fun applyLaunchOptions(options: LaunchOptions) {
        if (options.isEmpty) return
        scope.launch {
            ops.withLock {
                if (options.reset) reset()
                if (options.demo) _isDemo.value = true
                val origin = options.origin
                when {
                    origin != null -> {
                        val info = libraryFor(origin)
                        libraryStore.set(info)
                        activate(info)
                    }
                    options.demo || options.reset -> _library.value?.let { activate(it) }
                }
                options.route?.let(::applyRoute)
            }
        }
    }

    private suspend fun reset() {
        libraryStore.set(null)
        withContext(io) { secrets.clear() }
        favourites.clear()
        clients.values.forEach { it.setVerificationCookie(null) }
        _library.value = null
        _backend.value = null
        _session.value = null
        _accountName.value = null
        _hasVerificationCookie.value = false
        _selectedTab.value = Tab.HOME
    }

    /** The directory entry for [origin] (public GET, bounded), else a minimal stand-in. */
    private suspend fun libraryFor(origin: String): LibraryInfo {
        val normalized = origin.trimEnd('/').lowercase()
        _library.value?.takeIf { it.origin.equals(normalized, ignoreCase = true) }?.let { return it }
        val listed = withTimeoutOrNull(12.seconds) {
            runCatching { directory.list(false) }.getOrNull()
        }?.firstOrNull { it.origin.equals(normalized, ignoreCase = true) }
        return listed ?: fallbackLibrary(normalized)
    }

    private fun applyRoute(route: LaunchRoute) {
        when (route) {
            LaunchRoute.Home -> selectTab(Tab.HOME)
            LaunchRoute.Borrowed -> selectTab(Tab.BORROWED)
            LaunchRoute.Favourites -> selectTab(Tab.FAVOURITES)
            LaunchRoute.Profile -> selectTab(Tab.PROFILE)
            is LaunchRoute.Search -> openSearch(route.query)
            is LaunchRoute.Results -> navigate(Tab.SEARCH, Route.Results(route.query, title = route.query))
            is LaunchRoute.Detail -> navigate(Tab.HOME, Route.Detail(BookRef.fromUrl(route.url)))
            LaunchRoute.Libraries -> showLibraryPicker()
            LaunchRoute.Login -> requestLogin()
            LaunchRoute.Verify -> requestVerification()
            LaunchRoute.Gallery -> _galleryVisible.value = true
        }
    }

    // endregion

    /** Runs [block] in the app scope under the ops lock; the caller's cancellation doesn't abort it. */
    private suspend fun <T> serial(block: suspend () -> T): T =
        scope.async { ops.withLock { block() } }.await()

    companion object {
        private const val TAG = "AppState"

        internal fun sessionKey(origin: String) = "session:$origin"
        internal fun verifyKey(origin: String) = "verify:$origin"

        /** A [LibraryInfo] for an origin that isn't in (or couldn't be read from) the directory. */
        internal fun fallbackLibrary(origin: String): LibraryInfo {
            val host = runCatching { URI(origin).host }.getOrNull() ?: origin.substringAfter("://")
            val region = when {
                host.endsWith(".co.uk") -> "UK"
                host.endsWith(".co.nz") || host.endsWith(".govt.nz") -> "NZ"
                host.endsWith(".com.sg") || host.endsWith(".sg") -> "SG"
                host.endsWith(".com.au") || host.endsWith(".gov.au") -> "AU"
                else -> "OTHER"
            }
            val name = host.substringBefore('.').replaceFirstChar { it.uppercaseChar() }
            return LibraryInfo(id = host, name = name, origin = origin, logoUrl = null, country = null, region = region, isTest = false)
        }

        /** Wires the production stores; demo data is read from `assets/demo/`. */
        fun create(context: Context, scope: CoroutineScope): AppState {
            val app = context.applicationContext
            return AppState(
                scope = scope,
                libraryStore = DataStoreLibraryStore(app),
                secrets = KeystoreSecretStore(app),
                favourites = FavouritesStore(File(app.filesDir, FavouritesStore.FILE_NAME)),
                loadDemoData = { origin ->
                    DemoData.load(
                        read = { name -> app.assets.open("${DemoData.ASSET_DIR}/$name").bufferedReader().use { it.readText() } },
                        origin = origin,
                        today = LocalDate.now(),
                    )
                },
            )
        }
    }
}
