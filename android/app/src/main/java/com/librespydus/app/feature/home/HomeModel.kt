package com.librespydus.app.feature.home

import androidx.lifecycle.ViewModel
import com.librespydus.app.core.BookRef
import com.librespydus.app.core.CoreHelpers
import com.librespydus.app.core.Formatting
import com.librespydus.app.core.Genre
import com.librespydus.app.core.LibraryBackend
import com.librespydus.app.core.ResultsSort
import com.librespydus.app.core.Route
import com.librespydus.app.core.Shelf
import com.librespydus.app.feature.search.BrowseLoadable
import com.librespydus.app.feature.search.BrowseReporter
import com.librespydus.app.feature.search.SearchResultsModel
import com.librespydus.app.feature.search.trimmedOrNull
import com.librespydus.app.feature.search.valueOrNull
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import uniffi.librespydus_core.PatronItem
import java.time.LocalDate
import java.time.chrono.IsoChronology
import java.time.format.DateTimeFormatter
import java.util.Locale

/**
 * What Home's shelves were loaded for. New arrivals are public, so only a new library or a
 * verification reloads them; the loans shelf also follows sign-in, sign-out and account changes.
 */
data class HomeReloadKey(val origin: String, val verification: Int, val account: Int) {
    /** The part New arrivals depends on. */
    val publicPart: HomeReloadKey get() = copy(account = 0)
}

/** Everything a Home load needs from AppState, captured when it starts. */
class HomeLoadContext(
    val backend: LibraryBackend,
    val session: String?,
    val key: HomeReloadKey,
    val report: BrowseReporter,
)

/**
 * Home's shelves: "New arrivals" (the first `homeShelves()` entry) and, signed in, the current
 * loans. Each loads concurrently with its own state, so one slow or failing shelf never blocks
 * the other.
 */
class HomeModel(
    shelves: List<Shelf> = CoreHelpers.homeShelves(),
    val genres: List<Genre> = CoreHelpers.homeGenres(),
) : ViewModel() {
    val arrivalsShelf: Shelf? = shelves.firstOrNull()

    private val _arrivals = MutableStateFlow<BrowseLoadable<List<BookRef>>>(BrowseLoadable.Idle)
    val arrivals: StateFlow<BrowseLoadable<List<BookRef>>> = _arrivals.asStateFlow()

    /** Idle while signed out. */
    private val _loans = MutableStateFlow<BrowseLoadable<List<LoanShelfItem>>>(BrowseLoadable.Idle)
    val loans: StateFlow<BrowseLoadable<List<LoanShelfItem>>> = _loans.asStateFlow()

    /** Per shelf: the key it last settled for, and the key of the load in flight. */
    private var arrivalsLoadedKey: HomeReloadKey? = null
    private var arrivalsLoadingKey: HomeReloadKey? = null
    private var loansLoadedKey: HomeReloadKey? = null
    private var loansLoadingKey: HomeReloadKey? = null

    /** Per shelf, bumped by each load of it: a late answer for an older load is dropped. */
    private var arrivalsGeneration = 0
    private var loansGeneration = 0

    /**
     * Loads each shelf once per key: coming back from a pushed screen or another tab doesn't
     * reload, and a load already in flight for the key isn't started again.
     */
    suspend fun loadIfNeeded(context: HomeLoadContext) {
        val publicKey = context.key.publicPart
        coroutineScope {
            if (publicKey != arrivalsLoadedKey && publicKey != arrivalsLoadingKey) launch { loadArrivals(context, keepContent = false) }
            if (context.key != loansLoadedKey && context.key != loansLoadingKey) launch { loadLoans(context, keepContent = false) }
        }
    }

    /** Pull to refresh: both shelves at once, keeping the current covers on screen while reloading. */
    suspend fun refresh(context: HomeLoadContext) {
        coroutineScope {
            launch { loadArrivals(context, keepContent = true) }
            launch { loadLoans(context, keepContent = true) }
        }
    }

    suspend fun retryArrivals(context: HomeLoadContext) = loadArrivals(context, keepContent = false)

    suspend fun retryLoans(context: HomeLoadContext) = loadLoans(context, keepContent = false)

    private suspend fun loadArrivals(context: HomeLoadContext, keepContent: Boolean) {
        val key = context.key.publicPart
        val shelf = arrivalsShelf ?: run {
            _arrivals.value = BrowseLoadable.Loaded(emptyList())
            arrivalsLoadedKey = key
            return
        }
        val generation = ++arrivalsGeneration
        arrivalsLoadingKey = key
        if (!(keepContent && _arrivals.value.valueOrNull != null)) _arrivals.value = BrowseLoadable.Loading
        try {
            val results = context.backend.searchSorted(shelf.query, shelf.sort)
            if (generation != arrivalsGeneration) return
            arrivalsLoadedKey = key
            if (results.records.isEmpty() && !results.noResults) {
                _arrivals.value = BrowseLoadable.Failed(SearchResultsModel.UNRECOGNISED_PAGE)
                return
            }
            // One cover per book: `ref.id` is the shelf's identity (records without a work IRN or
            // URL fall back to their title).
            val books = SearchResultsModel.merge(emptyList(), results.records).map { it.ref }.distinctBy { it.id }
            _arrivals.value = BrowseLoadable.Loaded(books.take(SHELF_LIMIT))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (generation != arrivalsGeneration) return
            val shown = context.report(e) ?: return
            arrivalsLoadedKey = key
            if (keepContent && _arrivals.value.valueOrNull != null) return
            _arrivals.value = BrowseLoadable.Failed(shown)
        } finally {
            if (generation == arrivalsGeneration) arrivalsLoadingKey = null
        }
    }

    private suspend fun loadLoans(context: HomeLoadContext, keepContent: Boolean) {
        val key = context.key
        val generation = ++loansGeneration
        val session = context.session ?: run {
            _loans.value = BrowseLoadable.Idle
            loansLoadedKey = key
            loansLoadingKey = null
            return
        }
        loansLoadingKey = key
        if (!(keepContent && _loans.value.valueOrNull != null)) _loans.value = BrowseLoadable.Loading
        try {
            val list = context.backend.loans(session)
            if (generation != loansGeneration) return
            loansLoadedKey = key
            _loans.value = BrowseLoadable.Loaded(LoanShelfItem.make(list.items))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (generation != loansGeneration) return
            val shown = context.report(e) ?: return
            loansLoadedKey = key
            if (keepContent && _loans.value.valueOrNull != null) return
            _loans.value = BrowseLoadable.Failed(shown)
        } finally {
            if (generation == loansGeneration) loansLoadingKey = null
        }
    }

    /** "View All" on New arrivals. */
    val arrivalsRoute: Route.Results?
        get() = arrivalsShelf?.let { Route.Results(it.query, it.sort, it.title) }

    companion object {
        /** How many covers the "New arrivals" shelf shows. */
        const val SHELF_LIMIT = 10

        fun route(genre: Genre): Route.Results = Route.Results(genre.query, ResultsSort.RELEVANCE, genre.label)
    }
}

/** "Hello {first name}!" from `AppState.accountName`. */
object HomeGreeting {
    /** "Hello Jane!", or "Hello!" when there's no usable first name (signed out, still loading). */
    fun title(accountName: String?): String = firstName(accountName)?.let { "Hello $it!" } ?: "Hello!"

    /**
     * The given name from an account name as libraries write it: "Smith, Jane" (surname first) or
     * "Jane Smith". Titles ("Mrs", "Dr.") are skipped, ALL-CAPS and all-lowercase names are
     * capitalised ("SMITH, JANE" → "Jane"), and a bare initial ("J. Smith") gives null.
     */
    fun firstName(accountName: String?): String? {
        val name = accountName.trimmedOrNull() ?: return null
        val comma = name.indexOf(',')
        val given = if (comma >= 0) name.substring(comma + 1) else name
        var words = given.split(Regex("\\s+"))
            .map { it.trim { c -> !c.isLetterOrDigit() && c != '-' && c != '\'' } }
            .filter { it.isNotEmpty() && it.lowercase() !in HONORIFICS }
        // After a comma every word is a given name, so "Smith, J. Jane" can skip the initial;
        // without one, the word after an initial is likely the surname.
        if (comma >= 0) words = words.filter { it.length > 1 }
        val first = words.firstOrNull() ?: return null
        if (first.length <= 1 || first.none { it.isLetter() }) return null
        return normalisedCase(first)
    }

    private val HONORIFICS = setOf(
        "mr", "mrs", "ms", "miss", "mx", "dr", "prof", "professor", "sir", "dame", "rev", "fr", "master",
    )

    /** "JANE" / "jane" → "Jane", "MARY-JANE" → "Mary-Jane"; mixed case ("McKenzie") is kept. */
    private fun normalisedCase(word: String): String {
        if (word != word.uppercase() && word != word.lowercase()) return word
        return word.lowercase().split('-').joinToString("-") { part -> part.replaceFirstChar { it.titlecase(Locale.ROOT) } }
    }
}

/** A current loan on Home: cover plus "3 days left" / "Due 27 Oct" / "2 days overdue". */
data class LoanShelfItem(
    val ref: BookRef,
    val caption: String?,
    val tone: Tone,
    val dueDate: LocalDate?,
) {
    enum class Tone {
        NORMAL,

        /** Due within [SOON_DAYS] days. */
        SOON,
        OVERDUE,
    }

    val id: String get() = ref.id

    companion object {
        /** Within this many days the caption counts down ("3 days left"); later it shows the date. */
        const val COUNTDOWN_DAYS = 14

        /** Due within this many days (or today): [Tone.SOON]. */
        const val SOON_DAYS = 3

        // Library dates are Gregorian whatever the device calendar is.
        private val SHORT_DATE: DateTimeFormatter =
            DateTimeFormatter.ofPattern("d MMM", Locale.getDefault()).withChronology(IsoChronology.INSTANCE)

        fun from(item: PatronItem, today: LocalDate = LocalDate.now()): LoanShelfItem {
            val ref = BookRef.from(item)
            val dueText = item.fields["Due"].trimmedOrNull()
            val due = Formatting.parseDueDate(dueText)
                ?: return LoanShelfItem(ref, dueText?.let { "Due $it" }, Tone.NORMAL, null)
            val days = Formatting.daysUntil(due, today)
            return when {
                days < 0 -> LoanShelfItem(ref, daysText(days), Tone.OVERDUE, due)
                else -> LoanShelfItem(
                    ref = ref,
                    caption = if (days <= COUNTDOWN_DAYS) daysText(days) else "Due ${SHORT_DATE.format(due)}",
                    tone = if (days <= SOON_DAYS) Tone.SOON else Tone.NORMAL,
                    dueDate = due,
                )
            }
        }

        /** Loans in due order (overdue first); loans without a readable date last, in list order. */
        fun make(items: List<PatronItem>, today: LocalDate = LocalDate.now()): List<LoanShelfItem> =
            items.map { from(it, today) }
                .distinctBy { it.id }
                .sortedWith(compareBy(nullsLast()) { it.dueDate })

        /** "3 days left", "1 day left", "Due today", "1 day overdue", "2 days overdue". */
        fun daysText(days: Long): String = when {
            days < -1 -> "${-days} days overdue"
            days == -1L -> "1 day overdue"
            days == 0L -> "Due today"
            days == 1L -> "1 day left"
            else -> "$days days left"
        }
    }
}

/** The icon for a `homeGenres()` key (an id the UI maps to an [androidx.compose.ui.graphics.vector.ImageVector]). */
enum class HomeGenreIcon {
    PERSON, STORIES, SCIENCE, MUSEUM, WAND, FINGERPRINT, DINING, CHILD,

    /** Unknown keys. */
    BOOKS,
    ;

    companion object {
        fun forKey(key: String): HomeGenreIcon = when (key) {
            "biography" -> PERSON
            "fiction" -> STORIES
            "science" -> SCIENCE
            "history" -> MUSEUM
            "fantasy" -> WAND
            "crime" -> FINGERPRINT
            "cooking" -> DINING
            "children" -> CHILD
            else -> BOOKS
        }
    }
}
