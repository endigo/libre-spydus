package com.librespydus.app.core

import android.content.Context
import android.util.Log
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
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

/**
 * A destination to open inside a tab's stack (consumed by that tab's NavHost). [fromRoot]: the
 * stack is popped back to the tab's root first, so Back from [route] lands there.
 */
data class PendingNavigation(val tab: Tab, val route: Route, val fromRoot: Boolean = false)

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
    /**
     * The card number the library account itself gives (§5.9), for [cardNumber] when none is saved
     * on the device: the core reads it from the account's Personal details page.
     */
    private val accountCardNumber: (AccountSnapshot) -> String? = ::accountCardNumberOf,
    /** Which libraries were offered the copy of local favourites into My Picks (§5.11). */
    picksOffers: PicksOfferStore = InMemoryPicksOfferStore(),
    /** Where the core saves pages (§5.13), so `ls_reset` can delete them; null in tests. */
    private val savedPagesDir: File? = null,
    /** Each library's sign-in details behind the fingerprint check (§5.14). */
    private val biometrics: BiometricVault = BiometricVault.Unavailable,
    /** Demo mode's: kept in memory, the real check still runs. */
    private val demoBiometrics: BiometricVault = biometrics,
    /** The libraries whose fingerprint-or-face offer was answered Not now (§5.14). */
    private val biometricOffers: BiometricOfferStore = InMemoryBiometricOfferStore(),
    /**
     * The app has the screen (MainActivity resumed, nothing over its window), so a fingerprint or
     * face check started on its own (sign back in, the offer's save) can show without colliding.
     */
    private val foreground: StateFlow<Boolean> = MutableStateFlow(true),
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
    private val _savedCardNumber = MutableStateFlow<String?>(null)
    private val _biometricAvailable = MutableStateFlow(false)
    private val _biometricOn = MutableStateFlow(false)
    private val _biometricOffer = MutableStateFlow<BiometricOffer?>(null)
    private val _biometricTurnOn = MutableStateFlow(false)
    private val _biometricSigningIn = MutableStateFlow(false)
    private val _biometricSaving = MutableStateFlow(false)
    private val _loginError = MutableStateFlow<UserFacingError?>(null)

    /** False once a check ended without signing in: the sign-in sheet doesn't start another by itself. */
    private var loginAutoCheck = true

    /** The fingerprint sign-in running (or waiting for the foreground), if any. */
    private var biometricJob: Job? = null

    /** Demo card numbers entered or edited on the card, by origin, in memory only (the demo account's own is in its snapshot). */
    private val demoCardNumbers = mutableMapOf<String, String?>()

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

    /**
     * My Picks (§5.11): signed in, the bookmark saves to the reader's Pick list on the library
     * account and Favourites shows it. Bound to the library and session with [account].
     */
    val picks = PicksStore(scope, picksOffers, report = ::report, now = now)

    /** The borrower's name, e.g. "Test Borrower": from [account]'s snapshot, else [refreshAccount]. */
    val accountName: StateFlow<String?> =
        combine(account.snapshot, _fallbackName) { snapshot, fallback -> snapshot?.accountName ?: fallback }
            .stateIn(scope, SharingStarted.Eagerly, null)

    /**
     * The selected library's card number for My card (§5.9): the one saved on this device (entered
     * or edited on the card), else the account's own. Null → the card asks for it.
     */
    val cardNumber: StateFlow<String?> =
        combine(_savedCardNumber, account.snapshot) { saved, snapshot -> saved ?: snapshot?.let(accountCardNumber) }
            .stateIn(scope, SharingStarted.Eagerly, null)

    /**
     * When the card expires ("12 Mar 2027", as the Personal details page writes it), while the card
     * shown is the account's own (not a different number entered on the device).
     */
    val cardExpires: StateFlow<String?> =
        combine(_savedCardNumber, account.snapshot) { saved, snapshot ->
            snapshot?.cardExpires?.takeIf { saved == null || saved == snapshot.let(accountCardNumber) }
        }.stateIn(scope, SharingStarted.Eagerly, null)

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

    /** A verification cookie is stored for the selected library (Profile "Verified on this phone"). */
    val hasVerificationCookie: StateFlow<Boolean> = _hasVerificationCookie.asStateFlow()

    /** Sign-in requested (session expired, or a signed-out action); the root presents the login sheet. */
    val loginNeeded: StateFlow<Boolean> = _loginNeeded.asStateFlow()

    /**
     * The phone can run the fingerprint check (§5.14); none of it shows otherwise. Checked
     * on library change, sign-in and each return to the foreground.
     */
    val biometricAvailable: StateFlow<Boolean> = _biometricAvailable.asStateFlow()

    /** The selected library has sign-in details behind the check (Profile's switch is on). */
    val biometricOn: StateFlow<Boolean> = _biometricOn.asStateFlow()

    /** After a typed sign-in: "Sign in with fingerprint next time?" ([answerBiometricOffer]). */
    val biometricOffer: StateFlow<BiometricOffer?> = _biometricOffer.asStateFlow()

    /** The sign-in sheet was opened by Profile's switch: its next typed sign-in saves the details. */
    val biometricTurnOn: StateFlow<Boolean> = _biometricTurnOn.asStateFlow()

    /** A fingerprint sign-in is running (the check is up, or the library is answering). */
    val biometricSigningIn: StateFlow<Boolean> = _biometricSigningIn.asStateFlow()

    /** Why a fingerprint sign-in didn't sign in, for the sign-in form to show. */
    val loginError: StateFlow<UserFacingError?> = _loginError.asStateFlow()

    /**
     * My Picks' copy offer (§5.11) once it may show: not over the sign-in sheet, and only after the
     * fingerprint-or-face offer (§5.14) has been answered and its check has ended.
     */
    val picksOfferToShow: StateFlow<PicksOffer?> =
        combine(picks.offer, _biometricOffer, _biometricSaving, _loginNeeded) { offer, biometric, saving, login ->
            offer.takeIf { biometric == null && !saving && !login }
        }.stateIn(scope, SharingStarted.Eagerly, null)

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
        val card = if (demo) demoCardNumber(origin) else withContext(io) { secrets.get(cardKey(origin)) }
        val key = cacheKey(backend)
        val saved = if (session != null && key != null) account.loadSaved(key) else null
        val vault = vault(backend)
        val (biometricAvailable, biometricOn) = withContext(io) { vault.isAvailable() to vault.hasDetails(backend.origin) }

        // From here on nothing suspends: the store and the screens switch library together.
        cancelBiometricSignIn()
        account.bind(backend, session, key, saved)
        picks.bind(backend, session)
        _library.value = info
        _backend.value = backend
        _hasVerificationCookie.value = cookie != null
        _verificationNeeded.value = false
        _verificationAttempts.value = 0
        _loginNeeded.value = false
        _fallbackName.value = null
        _session.value = session
        _savedCardNumber.value = card
        _biometricAvailable.value = biometricAvailable
        _biometricOn.value = biometricOn
        _biometricOffer.value = null
        _biometricTurnOn.value = false
        _loginError.value = null
        loginAutoCheck = true
        _accountGeneration.update { it + 1 }
        _verificationGeneration.update { it + 1 }
        if (session != null) {
            account.refresh(force = true)
            picks.reload()
        }
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
     * Signs in to the selected library from the sign-in form and stores the session
     * (Keystore-encrypted, per origin; demo sessions stay in memory). Throws `SpydusException` —
     * show it with [UserFacingError]. Then offers the fingerprint check (§5.14) and the
     * copy of this phone's favourites (§5.11).
     */
    suspend fun signIn(username: String, password: String) = signIn(username, password, typed = true)

    /**
     * [typed]: from the sign-in form. Otherwise the fingerprint check is signing the same
     * reader back in (§5.14): the saved account stays on screen until the fresh one, and nothing is
     * offered.
     */
    private suspend fun signIn(username: String, password: String, typed: Boolean) {
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
            val key = cacheKey(backend)
            if (typed) {
                // Typed while a fingerprint sign-in waited for the foreground: that one is moot.
                cancelBiometricSignIn()
                // A new sign-in starts from nothing: the previous session's account stops (it can't
                // save any more) and any copy saved for this library is deleted.
                account.bind(null, null, null)
                if (key != null) account.deleteSaved(key)
                account.bind(backend, token, key)
            } else {
                account.bind(backend, token, key, saved = account.snapshot.value)
            }
            picks.bind(backend, token)
            _fallbackName.value = null
            _session.value = token
            _loginNeeded.value = false
            _loginError.value = null
            loginAutoCheck = true
            _accountGeneration.update { it + 1 }
            account.refresh(force = true)
            picks.reload()
            if (typed) {
                afterTypedSignIn(backend, SignInDetails(username.trim(), password))
                // An interactive sign-in: offer, once per library, to copy this phone's favourites
                // (shown once the fingerprint-or-face offer is answered).
                picks.offerAfterSignIn(picksScope(backend), favourites.list(_library.value?.origin ?: backend.origin))
            }
            updateMorningCheck()
        }
    }

    /** Signs out of the selected library and deletes its saved account (other libraries keep theirs). */
    suspend fun signOut() {
        val backend = _backend.value ?: return
        val origin = backend.origin
        // Signing out forgets the fingerprint-or-face details and the Not now (§5.14).
        cancelBiometricSignIn()
        _biometricOn.value = false
        _biometricOffer.value = null
        biometricOffers.forget(biometricScope(backend))
        withContext(io) { vault(backend).delete(origin) }
        _session.value = null
        _fallbackName.value = null
        _savedCardNumber.value = null
        val forgotten = account.discard()
        picks.clear()
        updateMorningCheck()
        withContext(io) { secrets.put(sessionKey(origin), null) }
        storeCardNumber(backend, null)
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
        // A fingerprint may have been enrolled (or removed) in Settings meanwhile.
        _backend.value?.let { _biometricAvailable.value = vault(it).isAvailable() }
        account.refresh()
    }

    fun requestLogin() {
        _loginNeeded.value = true
    }

    /** The sign-in sheet closed: the next one starts from scratch (and may start the check again). */
    fun dismissLogin() {
        _loginNeeded.value = false
        _biometricTurnOn.value = false
        _loginError.value = null
        loginAutoCheck = true
    }

    fun clearLoginError() {
        _loginError.value = null
    }

    /**
     * Saves (trimmed) or, when blank or null, deletes the selected library's card number (§5.9):
     * in the secure store next to the session, or in memory for demo mode. Sign-out deletes it;
     * an expired session keeps it.
     */
    suspend fun setCardNumber(number: String?) = serial {
        val backend = _backend.value ?: return@serial
        val value = number?.trim()?.takeIf { it.isNotEmpty() }
        _savedCardNumber.value = value
        storeCardNumber(backend, value)
    }

    private suspend fun storeCardNumber(backend: LibraryBackend, number: String?) {
        if (backend.isDemo) demoCardNumbers[backend.origin] = number
        else withContext(io) { secrets.put(cardKey(backend.origin), number) }
    }

    private fun demoCardNumber(origin: String): String? = demoCardNumbers[origin]

    // endregion
    // region Fingerprint (§5.14)

    /**
     * After a typed sign-in: details kept for another username are deleted (the check must never
     * sign in to the previous account); then, when the check is available and the library has no
     * details, offer to keep them, unless the reader answered Not now for this library. When the
     * sheet came from Profile's switch the offer doesn't ask ([BiometricOffer.asks]): the check runs.
     */
    private suspend fun afterTypedSignIn(backend: LibraryBackend, details: SignInDetails) {
        val vault = vault(backend)
        val origin = backend.origin
        val turnOn = _biometricTurnOn.value
        _biometricTurnOn.value = false
        val kept = withContext(io) { vault.username(origin) }
        if (kept != null && !SignInDetails.sameUsername(kept, details.username)) {
            withContext(io) { vault.delete(origin) }
        }
        val available = vault.isAvailable()
        val on = kept != null && SignInDetails.sameUsername(kept, details.username)
        _biometricAvailable.value = available
        _biometricOn.value = on
        if (!available || on) return
        val name = libraryName()
        // Either way it waits for the sheet to go (BiometricOfferDialog): the check never shows
        // over a closing sheet or the system's offer to save the password.
        if (turnOn || !biometricOffers.wasDeclined(biometricScope(backend))) {
            _biometricOffer.value = BiometricOffer(origin, biometricScope(backend), name, details, asks = !turnOn)
        }
    }

    /**
     * The reader answered the offer. [use]: the check runs and, once it passes, the details are kept
     * (a cancelled or failed check keeps nothing and isn't remembered). Not now is remembered for
     * this library until sign-out. Either way the PIN is dropped from memory.
     */
    fun answerBiometricOffer(use: Boolean): Job? {
        val offer = _biometricOffer.value ?: return null
        _biometricOffer.value = null
        if (!use) {
            biometricOffers.markDeclined(offer.scope)
            return null
        }
        val backend = _backend.value?.takeIf { it.origin == offer.origin } ?: return null
        _biometricSaving.value = true
        return scope.launch {
            try {
                saveBiometricDetails(backend, offer.details)
            } finally {
                _biometricSaving.value = false
            }
        }
    }

    private suspend fun saveBiometricDetails(backend: LibraryBackend, details: SignInDetails) {
        foreground.first { it }
        if (backend !== _backend.value) return
        val saved = vault(backend).save(backend.origin, details, libraryName())
        if (saved && backend === _backend.value && _session.value != null) _biometricOn.value = true
    }

    /**
     * Profile's switch. Off: the details are deleted at once. On: the sign-in sheet opens with
     * `turnOnNote`, and its next typed sign-in saves the details (the check, no question).
     */
    suspend fun setBiometricSignIn(on: Boolean) {
        val backend = _backend.value ?: return
        if (on) {
            _biometricTurnOn.value = true
            _loginNeeded.value = true
            return
        }
        cancelBiometricSignIn()
        _biometricOn.value = false
        withContext(io) { vault(backend).delete(backend.origin) }
    }

    /**
     * The sign-in sheet opened: starts the check at once when the library has details, unless the
     * sheet opened because a check was just cancelled or failed (or it came from Profile's switch).
     */
    fun autoBiometricSignIn(): Job? {
        if (!loginAutoCheck || _biometricTurnOn.value || !_biometricOn.value || !_biometricAvailable.value) return null
        return signInWithBiometrics()
    }

    /**
     * The sign-in sheet's "Sign in with fingerprint": the check, then `login` with the kept
     * details. Success closes the sheet like a typed sign-in; otherwise the form shows why
     * ([loginError]) and the sheet won't start the check again by itself.
     */
    fun signInWithBiometrics(): Job? {
        biometricJob?.takeIf { it.isActive }?.let { return it }
        val backend = _backend.value ?: return null
        return scope.launch {
            val failure = biometricSignIn(backend) ?: return@launch
            if (backend !== _backend.value) return@launch
            _loginError.value = failure.error
            loginAutoCheck = false
        }.also { biometricJob = it }
    }

    /**
     * `NotLoggedIn` while the library has details: signs back in with the check instead of asking
     * for the PIN. The session and saved account stay on screen until it ends; in the background
     * it waits for the foreground. When it doesn't sign in, the session ends as it would have and
     * the sign-in form shows why.
     */
    private fun startBiometricSignIn(backend: LibraryBackend) {
        biometricJob = scope.launch {
            foreground.first { it }
            if (backend !== _backend.value) return@launch
            val failure = biometricSignIn(backend) ?: return@launch
            if (backend !== _backend.value) return@launch
            expireSession()
            _loginError.value = failure.error
            loginAutoCheck = false
            _loginNeeded.value = true
        }
    }

    /** Why a fingerprint sign-in didn't sign in; [error] for the form (null: nothing to say). */
    private class BiometricFailure(val error: UserFacingError?)

    /** The check, then `login` with the kept details; null once signed in. */
    private suspend fun biometricSignIn(backend: LibraryBackend): BiometricFailure? {
        val vault = vault(backend)
        val origin = backend.origin
        _biometricSigningIn.value = true
        try {
            val details = when (val read = vault.read(origin, libraryName())) {
                is BiometricRead.Unlocked -> read.details
                BiometricRead.Declined -> return BiometricFailure(null)
                BiometricRead.Invalidated -> {
                    if (backend === _backend.value) _biometricOn.value = false
                    return BiometricFailure(null)
                }
            }
            return try {
                signIn(details.username, details.pin, typed = false)
                null
            } catch (e: CancellationException) {
                throw e
            } catch (e: SpydusException.CredentialsRejected) {
                // The PIN changed: these details will never work again.
                withContext(io) { vault.delete(origin) }
                if (backend === _backend.value) _biometricOn.value = false
                BiometricFailure(UserFacingError.from(e, _library.value?.name))
            } catch (e: Exception) {
                // signIn has routed it (verification opens its sheet, which says enough).
                BiometricFailure(UserFacingError.from(e, _library.value?.name).takeUnless { it.kind == UserFacingError.Kind.VERIFICATION })
            }
        } finally {
            _biometricSigningIn.value = false
        }
    }

    private fun cancelBiometricSignIn() {
        biometricJob?.cancel()
        biometricJob = null
        _biometricSigningIn.value = false
    }

    private fun vault(backend: LibraryBackend): BiometricVault = if (backend.isDemo) demoBiometrics else biometrics

    /** The library the Not now is remembered under; demo under its own. */
    private fun biometricScope(backend: LibraryBackend): String =
        if (backend.isDemo) "demo:${backend.origin}" else backend.origin

    private fun libraryName(): String = _library.value?.name ?: Copy.Common.yourLibrary

    /**
     * DEBUG Profile › Developer › Expire session (§5.14): an invalid session replaces the real one
     * and the account is refreshed, so the library (or the demo backend) answers `NotLoggedIn`.
     */
    suspend fun expireSessionForDebug() = serial {
        val backend = _backend.value ?: return@serial
        if (_session.value == null) return@serial
        if (!backend.isDemo) withContext(io) { secrets.put(sessionKey(backend.origin), EXPIRED_SESSION) }
        account.bind(backend, EXPIRED_SESSION, cacheKey(backend), saved = account.snapshot.value)
        picks.bind(backend, EXPIRED_SESSION)
        _session.value = EXPIRED_SESSION
        account.refresh(force = true)
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

    /**
     * A notification was tapped (§5.8 "Tap"): once the app has restored its state (on a cold launch
     * the saved account is loaded first), Your Books on the alert's list with the book page of the
     * item it names pushed over it, or just the list ([AlertRouting]).
     */
    fun openFromAlert(tap: AlertTap) {
        scope.launch {
            ready.first { it }
            ops.withLock {
                val snapshot = account.snapshot.value.takeIf { _session.value != null }
                open(AlertRouting.destination(tap, _library.value?.origin, snapshot))
            }
        }
    }

    private fun open(destination: AlertDestination) {
        val book = destination.book
        if (book == null) {
            openBorrowed(destination.segment)
        } else {
            // Your Books' list underneath (on the right segment once it shows), so Back lands there.
            _pendingBorrowedSegment.value = destination.segment
            navigate(Tab.BORROWED, Route.Detail(book), fromRoot = true)
        }
    }

    /** DEBUG `ls_demo_post_alerts`: one of each alert from the demo account, once it has loaded. */
    private suspend fun postDemoAlerts() {
        val backend = _backend.value?.takeIf { it.isDemo } ?: return
        val snapshot = account.snapshot.value ?: return
        alerts.postSamples(alertScope(backend), snapshot)
    }

    /** Every fresh account snapshot (foreground, pull to refresh, after a change, morning check). */
    private suspend fun accountRefreshed(backend: LibraryBackend, snapshot: AccountSnapshot) {
        alerts.process(alertScope(backend), snapshot)
    }

    /** The morning check is scheduled while signed in with a switch on (§5.8). */
    private fun updateMorningCheck() {
        alerts.updateSchedule(signedIn = _session.value != null)
    }

    /** The library My Picks remembers its copy offer under; demo under its own. */
    private fun picksScope(backend: LibraryBackend): String =
        if (backend.isDemo) "demo:${backend.origin}" else backend.origin

    /** The library [Alerts] files [backend]'s keys under; demo data under its own. */
    private fun alertScope(backend: LibraryBackend): String = Alerts.scope(backend.origin, backend.isDemo)

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
                when {
                    // A fingerprint sign-in is already on its way (§5.14).
                    biometricJob?.isActive == true -> Unit
                    backend != null && _session.value != null && _biometricOn.value && vault(backend).isAvailable() ->
                        startBiometricSignIn(backend)
                    else -> {
                        expireSession()
                        _loginNeeded.value = true
                    }
                }
            }
        }
        return UserFacingError.from(error, _library.value?.name)
    }

    /** The session ended: forgets it and this library's saved account (§5.2a); the details stay. */
    private fun expireSession() {
        val backend = _backend.value
        if (_session.value == null) return
        _session.value = null
        _fallbackName.value = null
        account.discard()
        picks.clear()
        _accountGeneration.update { it + 1 }
        updateMorningCheck()
        if (backend != null) {
            scope.launch(io) { secrets.put(sessionKey(backend.origin), null) }
            scope.launch { alerts.forget(alertScope(backend)) }
        }
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

    /** Switches to [tab] and pushes [route] onto its stack ([fromRoot]: over the tab's root). */
    fun navigate(tab: Tab, route: Route, fromRoot: Boolean = false) {
        _pendingNavigation.value = PendingNavigation(tab, route, fromRoot)
        _selectedTab.value = tab
    }

    fun consumePendingNavigation(tab: Tab): Route? = takePendingNavigation(tab)?.route

    /** Returns and clears the navigation pending for [tab]. */
    fun takePendingNavigation(tab: Tab): PendingNavigation? {
        val pending = _pendingNavigation.value?.takeIf { it.tab == tab } ?: return null
        _pendingNavigation.value = null
        return pending
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
                if (options.demoPostAlerts && _isDemo.value) {
                    val launchRefresh = account.runningRefresh
                    scope.launch {
                        launchRefresh?.join()
                        postDemoAlerts()
                    }
                }
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
        cancelBiometricSignIn()
        withContext(io) {
            secrets.clear()
            biometrics.deleteAll()
            demoBiometrics.deleteAll()
        }
        biometricOffers.clear()
        _biometricOn.value = false
        _biometricOffer.value = null
        favourites.clear()
        demoCardNumbers.clear()
        _savedCardNumber.value = null
        account.bind(null, null, null)
        account.deleteAllSaved()
        savedPagesDir?.let { dir -> withContext(io) { dir.deleteRecursively() } }
        picks.clear()
        picks.forgetOffers()
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
        internal fun cardKey(origin: String) = "card.$origin"

        /** DEBUG [expireSessionForDebug]: a session no library knows. */
        internal const val EXPIRED_SESSION = "expired-session"

        /** The account's own card number (from its Personal details page), or null. */
        internal fun accountCardNumberOf(snapshot: AccountSnapshot): String? = snapshot.cardNumber?.trim()?.takeIf(String::isNotEmpty)

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
        fun create(context: Context, scope: CoroutineScope, prompter: BiometricPrompter): AppState {
            val app = context.applicationContext
            // Public catalogue pages, in the cache dir: the system may empty it (§5.13).
            val savedPages = File(app.cacheDir, CoreBackend.SAVED_PAGES_DIR)
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
                picksOffers = SharedPreferencesPicksOfferStore(
                    app.getSharedPreferences(SharedPreferencesPicksOfferStore.PREFS_NAME, Context.MODE_PRIVATE),
                ),
                makeBackend = { origin -> CoreBackend.create(origin, savedPages) },
                savedPagesDir = savedPages,
                biometrics = KeystoreBiometricVault(app, prompter),
                demoBiometrics = PromptedMemoryVault(prompter),
                biometricOffers = SharedPreferencesBiometricOfferStore(
                    app.getSharedPreferences(SharedPreferencesBiometricOfferStore.PREFS_NAME, Context.MODE_PRIVATE),
                ),
                foreground = combine(prompter.resumed, prompter.focused) { resumed, focused -> resumed && focused }
                    .stateIn(scope, SharingStarted.Eagerly, false),
            )
        }
    }
}
