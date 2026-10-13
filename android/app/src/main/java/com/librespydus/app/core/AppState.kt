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
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.librespydus_core.AccountSnapshot
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
    accountStorage: AccountSnapshotStorage = InMemoryAccountSnapshotStorage(),
    now: () -> Long = System::currentTimeMillis,
    /** "Ready to collect" alerts and due-date reminders (§5.8). */
    val alerts: Alerts = Alerts.inMemory(io),
) {
    private val ops = Mutex()

    /** DEBUG demo options (`ls_demo_persist`, `ls_demo_refresh_fails`, `ls_demo_new_ready`), see [LaunchOptions]. */
    private var demoPersist = false
    private var demoRefreshFails = false
    private var demoNewReady = false

    /** One backend (one `SpydusClient`) per origin, kept for the app's lifetime. */
    private val clients = mutableMapOf<String, LibraryBackend>()

    private val _ready = MutableStateFlow(false)
    private val _library = MutableStateFlow<LibraryInfo?>(null)
    private val _backend = MutableStateFlow<LibraryBackend?>(null)
    private val _session = MutableStateFlow<String?>(null)
    private val _fallbackName = MutableStateFlow<String?>(null)
    private val _isDemo = MutableStateFlow(false)
    private val _selectedTab = MutableStateFlow(Tab.HOME)
    private val _pendingSearch = MutableStateFlow<String?>(null)
    private val _pendingNavigation = MutableStateFlow<PendingNavigation?>(null)
    private val _pendingBorrowedSegment = MutableStateFlow<BorrowedSegment?>(null)
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

    /**
     * The signed-in borrower's account, saved on the device per library and refreshed in the
     * background (§5.2a). Your Books, Home's loans shelf and the account name read it.
     */
    val account = AccountStore(scope, accountStorage, report = ::report, now = now, io = io, onRefreshed = ::accountRefreshed)

    /** The borrower's name, e.g. "Test Borrower": from [account]'s snapshot, else [refreshAccount]. */
    val accountName: StateFlow<String?> =
        combine(account.snapshot, _fallbackName) { snapshot, fallback -> snapshot?.accountName ?: fallback }
            .stateIn(scope, SharingStarted.Eagerly, null)

    /** Demo mode (§5.4, debug `ls_demo`): fixture account data, no account network calls. */
    val isDemo: StateFlow<Boolean> = _isDemo.asStateFlow()

    val selectedTab: StateFlow<Tab> = _selectedTab.asStateFlow()

    /** Text typed into Home's search field, for the Search tab to pick up ([consumePendingSearch]). */
    val pendingSearch: StateFlow<String?> = _pendingSearch.asStateFlow()

    /** A route to push onto a tab's stack (launch routes, cross-tab links). */
    val pendingNavigation: StateFlow<PendingNavigation?> = _pendingNavigation.asStateFlow()

    /** The list Your Books should show next ([openBorrowed]); Your Books consumes it. */
    val pendingBorrowedSegment: StateFlow<BorrowedSegment?> = _pendingBorrowedSegment.asStateFlow()

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

    /** Bumped on sign-in, sign-out, session expiry and library change. */
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
                updateMorningCheck()
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

    /**
     * Builds (or reuses) the origin's backend and loads that origin's session, cookie and saved
     * account, all before the library is shown, so its first frame already has the account. Then
     * refreshes the account in the background.
     */
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
        val backend = if (demo) {
            DemoBackend(core, withContext(io) { loadDemoData(origin) }, refreshFails = demoRefreshFails, newReady = demoNewReady)
        } else {
            core
        }
        val session = if (demo) DemoBackend.SESSION else withContext(io) { secrets.get(sessionKey(origin)) }
        val key = cacheKey(backend)
        val saved = if (session != null && key != null) account.loadSaved(key) else null

        // From here on nothing suspends: the store and the screens switch library together.
        account.bind(backend, session, key, saved)
        _library.value = info
        _backend.value = backend
        _hasVerificationCookie.value = cookie != null
        _verificationNeeded.value = false
        _verificationAttempts.value = 0
        _loginNeeded.value = false
        _fallbackName.value = null
        _session.value = session
        _accountGeneration.update { it + 1 }
        _verificationGeneration.update { it + 1 }
        if (session != null) account.refresh(force = true)
        updateMorningCheck()
    }

    /**
     * Where [backend]'s account is saved: under its origin. Demo data is kept in memory only,
     * unless DEBUG `ls_demo_persist` asks to save it (under a key of its own).
     */
    private fun cacheKey(backend: LibraryBackend): String? = when {
        !backend.isDemo -> backend.origin
        demoPersist -> "demo:${backend.origin}"
        else -> null
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
        // Committed under the ops lock, so a library switch can't interleave with it.
        serial {
            if (backend !== _backend.value) return@serial // the library changed meanwhile
            if (!backend.isDemo) withContext(io) { secrets.put(sessionKey(backend.origin), token) }
            // A new sign-in starts from nothing: the previous session's account stops (it can't
            // save any more) and any copy saved for this library is deleted.
            val key = cacheKey(backend)
            account.bind(null, null, null)
            if (key != null) account.deleteSaved(key)
            account.bind(backend, token, key)
            _fallbackName.value = null
            _session.value = token
            _loginNeeded.value = false
            _accountGeneration.update { it + 1 }
            account.refresh(force = true)
            updateMorningCheck()
        }
    }

    /** Signs out of the selected library and deletes its saved account (other libraries keep theirs). */
    suspend fun signOut() {
        val backend = _backend.value ?: return
        val origin = backend.origin
        _session.value = null
        _fallbackName.value = null
        val forgotten = account.discard()
        updateMorningCheck()
        withContext(io) { secrets.put(sessionKey(origin), null) }
        alerts.forget(alertScope(backend))
        forgotten.join()
        _accountGeneration.update { it + 1 }
    }

    /**
     * Profile's fallback while there is no account snapshot (its refresh failed before one
     * arrived): loads just the borrower's name with `account(session)`. Returns false only when
     * the request failed; a library page without a name is a success with a null [accountName].
     */
    suspend fun refreshAccount(): Boolean {
        val backend = _backend.value ?: return true
        val session = _session.value ?: return true
        return try {
            val links = backend.account(session)
            if (backend === _backend.value && session == _session.value) _fallbackName.value = links.name
            true
        } catch (e: SpydusException) {
            report(e)
            false
        }
    }

    /**
     * A reservation was placed, cancelled or edited: refreshes the account now (replacing a
     * refresh that may have started before the change).
     */
    fun accountDidChange() {
        account.refreshAfterChange()
    }

    /** The app came to the foreground (activity ON_START): refreshes the account if it is stale. */
    fun onForeground() {
        account.refresh()
    }

    fun requestLogin() {
        _loginNeeded.value = true
    }

    fun dismissLogin() {
        _loginNeeded.value = false
    }

    // endregion
    // region Alerts (§5.8)

    /**
     * Profile's "Ready to collect" switch (after the notification permission was granted). On:
     * what the saved account already has ready counts as alerted, so only what becomes ready
     * later is announced.
     */
    suspend fun setReadyAlerts(on: Boolean) {
        val backend = _backend.value
        val snapshot = account.snapshot.value.takeIf { _session.value != null }
        alerts.setReadyEnabled(on, backend?.let(::alertScope), snapshot)
        updateMorningCheck()
    }

    /** Profile's "Due-date reminders" switch (after the notification permission was granted). */
    suspend fun setDueReminders(on: Boolean) {
        alerts.setDueEnabled(on)
        updateMorningCheck()
    }

    /**
     * The 07:45 morning check ([MorningCheckWorker]): once the app has restored its state, fetches
     * the account with the saved session (joining a refresh already running, e.g. the one a cold
     * start begins) and saves it like any refresh, which raises "Ready to collect" alerts. Then
     * posts today's due reminders from the account it has, fresh or saved. Silent on failure;
     * `NotLoggedIn` signs out as anywhere else in the app.
     */
    suspend fun morningCheck() = scope.async {
        ready.first { it }
        val backend = _backend.value ?: return@async
        if (_session.value == null || !alerts.anyEnabled) return@async
        account.refresh(force = true)?.join()
        if (backend !== _backend.value || _session.value == null) return@async
        val snapshot = account.snapshot.value ?: return@async
        alerts.postDueReminders(alertScope(backend), snapshot)
    }.await()

    /** The end of a morning check: schedules the next one (or none, signed out / switches off). */
    fun scheduleNextMorningCheck() {
        alerts.scheduleNext(signedIn = _session.value != null)
    }

    /** A notification was tapped: Your Books on [segment], once the app has restored its state. */
    fun openFromAlert(segment: BorrowedSegment) {
        scope.launch { ops.withLock { openBorrowed(segment) } }
    }

    /** Every fresh account snapshot (foreground, pull to refresh, after a change, morning check). */
    private suspend fun accountRefreshed(backend: LibraryBackend, snapshot: AccountSnapshot) {
        alerts.process(alertScope(backend), snapshot)
    }

    /** The morning check is scheduled while signed in with a switch on (§5.8). */
    private fun updateMorningCheck() {
        alerts.updateSchedule(signedIn = _session.value != null)
    }

    /** The library [Alerts] files [backend]'s keys under; demo data under its own. */
    private fun alertScope(backend: LibraryBackend): String =
        if (backend.isDemo) "demo:${backend.origin}" else backend.origin

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
                val backend = _backend.value
                if (_session.value != null) {
                    _session.value = null
                    _fallbackName.value = null
                    account.discard()
                    _accountGeneration.update { it + 1 }
                    updateMorningCheck()
                    if (backend != null) {
                        scope.launch(io) { secrets.put(sessionKey(backend.origin), null) }
                        scope.launch { alerts.forget(alertScope(backend)) }
                    }
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
        // The verification may have been for a failed account refresh: retry it now.
        account.refresh(force = account.lastRefreshError.value != null)
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

    /** Switches to Your Books, back at its list, showing [segment] (the book page's held-status pill). */
    fun openBorrowed(segment: BorrowedSegment) {
        _pendingBorrowedSegment.value = segment
        navigate(Tab.BORROWED, Route.Borrowed)
    }

    /** Returns and clears the segment asked for by [openBorrowed]. */
    fun consumePendingBorrowedSegment(): BorrowedSegment? =
        _pendingBorrowedSegment.value.also { _pendingBorrowedSegment.value = null }

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
                if (options.demo) {
                    _isDemo.value = true
                    demoPersist = options.demoPersist
                    demoRefreshFails = options.demoRefreshFails
                    demoNewReady = options.demoNewReady
                }
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
                if (options.morningCheckNow) {
                    // Once the launch's own refresh has settled, so the check fetches a new account.
                    val launchRefresh = account.runningRefresh
                    scope.launch {
                        launchRefresh?.join()
                        alerts.runMorningCheckNow()
                    }
                }
            }
        }
    }

    private suspend fun reset() {
        libraryStore.set(null)
        withContext(io) { secrets.clear() }
        favourites.clear()
        account.bind(null, null, null)
        account.deleteAllSaved()
        alerts.reset()
        clients.values.forEach { it.setVerificationCookie(null) }
        _library.value = null
        _backend.value = null
        _session.value = null
        _fallbackName.value = null
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
                // Not backed up: noBackupFilesDir, sealed with the Keystore key of the session store.
                accountStorage = FileAccountSnapshotStorage(
                    File(app.noBackupFilesDir, FileAccountSnapshotStorage.DIR_NAME),
                    AesGcmSealer { KeystoreKeys.aesKey(KeystoreSecretStore.KEY_ALIAS) },
                ),
                loadDemoData = { origin ->
                    DemoData.load(
                        read = { name -> app.assets.open("${DemoData.ASSET_DIR}/$name").bufferedReader().use { it.readText() } },
                        origin = origin,
                        today = LocalDate.now(),
                    )
                },
                alerts = Alerts(
                    store = SharedPreferencesAlertStore(app.getSharedPreferences(SharedPreferencesAlertStore.PREFS_NAME, Context.MODE_PRIVATE)),
                    notifier = AndroidAlertNotifier(app),
                    scheduler = WorkManagerMorningCheck(app),
                ),
            )
        }
    }
}
