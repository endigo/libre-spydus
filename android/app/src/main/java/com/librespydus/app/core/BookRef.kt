package com.librespydus.app.core

import kotlinx.serialization.Serializable
import uniffi.librespydus_core.PatronItem
import uniffi.librespydus_core.RecordDetail
import uniffi.librespydus_core.RecordFormat
import uniffi.librespydus_core.RelatedItem
import uniffi.librespydus_core.SearchRecord

/** A format tab of a search record (docs/native-apps.md §4): serializable copy of [RecordFormat]. */
@Serializable
data class FormatRef(
    val code: String,
    val label: String? = null,
    val reserveUrl: String? = null,
    val availabilityUrl: String? = null,
) {
    fun toRecordFormat(): RecordFormat = RecordFormat(code, label, reserveUrl, availabilityUrl)

    companion object {
        fun from(format: RecordFormat) =
            FormatRef(format.code, format.label, format.reserveUrl, format.availabilityUrl)
    }
}

/**
 * A book as the app passes it around: the argument of the `detail` route and the favourites
 * record (§5.3, §5.5). Built from whatever the screen had ([SearchRecord], [PatronItem],
 * [RelatedItem], a stored favourite), so every field but [id] and [title] may be missing.
 *
 * [id] is a stable key, prefixed by its kind so keys from different sources never collide:
 * `bib:<bib IRN>`, `brn:<BRN>`, `work:<work IRN>`, else `url:<url>`.
 * [url] is what `record(url)` loads; prefer a [RecordDetail.bookmarkUrl] once known (see
 * [withDetail]) because search URLs embed a set id that expires. A loan's or reservation's
 * [url] is its session-free record URL (`CoreHelpers.patronItemRecordUrl`).
 */
@Serializable
data class BookRef(
    val id: String,
    val title: String,
    val author: String? = null,
    val coverUrl: String? = null,
    val url: String? = null,
    val isbn: String? = null,
    val year: String? = null,
    val formats: List<FormatRef> = emptyList(),
    /** Epoch millis when saved as a favourite; null elsewhere. */
    val savedAt: Long? = null,
    /**
     * The bib IRN of the loan or reservation this ref was opened from (Your Books, Home's loans
     * shelf): the book page selects the format of that edition. Null elsewhere, and in favourites
     * saved before it existed.
     */
    val bibIrn: String? = null,
) {
    /**
     * What its book page is saved under on the device (§5.13): the [id] when it is an IRN or BRN
     * (`work:`, `bib:`, `brn:`). Null for a `url:` or `title:` id: a URL carries a search's set
     * id, so the same book would be saved again under every search.
     */
    val savedPageKey: String? get() = id.takeIf { key -> SAVED_PAGE_PREFIXES.any(key::startsWith) }

    /**
     * Upgrades this ref with a loaded record: bookmark URL (stable), bib IRN key, cover, ISBN.
     * Keeps [formats] (records opened from a search carry them; a detail has none).
     */
    fun withDetail(detail: RecordDetail): BookRef = copy(
        id = stableId(bibIrn = detail.bibIrn, brn = detail.brn, workIrn = null, url = null) ?: id,
        title = detail.title?.takeIf { it.isNotBlank() } ?: title,
        coverUrl = detail.coverUrls.firstOrNull() ?: coverUrl,
        url = detail.bookmarkUrl ?: url,
        isbn = detail.isbn ?: isbn,
    )

    companion object {
        const val UNTITLED = Copy.Common.untitled

        private val SAVED_PAGE_PREFIXES = listOf("work:", "bib:", "brn:")

        fun from(record: SearchRecord): BookRef = BookRef(
            id = stableId(bibIrn = null, brn = null, workIrn = record.workIrn, url = record.detailUrl)
                ?: "title:${record.title.orEmpty()}",
            title = record.title.cleanTitle(),
            author = record.author,
            coverUrl = record.coverUrls.firstOrNull(),
            url = record.detailUrl,
            isbn = record.isbn,
            year = record.year,
            formats = record.formats.map(FormatRef::from),
        )

        /** A loan or reservation: loads with the session-free record URL, remembers the row's bib IRN. */
        fun from(item: PatronItem): BookRef = BookRef(
            id = stableId(bibIrn = item.bibIrn, brn = null, workIrn = null, url = item.bibUrl)
                ?: "title:${item.title.orEmpty()}",
            title = item.title.cleanTitle(),
            author = item.author,
            coverUrl = item.coverUrls.firstOrNull(),
            url = CoreHelpers.patronItemRecordUrl(item) ?: item.bibUrl,
            isbn = item.isbn,
            year = item.year,
            bibIrn = item.bibIrn?.trim()?.takeIf { it.isNotEmpty() },
        )

        fun from(item: RelatedItem): BookRef = BookRef(
            id = stableId(bibIrn = bibIrnFromUrl(item.bibUrl), brn = item.brn, workIrn = null, url = item.bibUrl)
                ?: "title:${item.title.orEmpty()}",
            title = item.title.cleanTitle(),
            author = item.author,
            coverUrl = item.coverUrls.firstOrNull(),
            url = item.bibUrl,
            isbn = item.isbn,
            year = item.year,
        )

        /** A bare record URL (debug `detail:<url>` launch route, deep links). */
        fun fromUrl(url: String, title: String = ""): BookRef =
            BookRef(id = "url:$url", title = title, url = url)

        internal fun stableId(bibIrn: String?, brn: String?, workIrn: String?, url: String?): String? = when {
            !bibIrn.isNullOrBlank() -> "bib:$bibIrn"
            !brn.isNullOrBlank() -> "brn:$brn"
            !workIrn.isNullOrBlank() -> "work:$workIrn"
            !url.isNullOrBlank() -> "url:$url"
            else -> null
        }

        private val IRN_PARAM = Regex("""(?:[?&]IRN=|QRY=IRN\()(\d+)""")

        /** `…BIBENQ/…?QRY=IRN(123)…` or `…&IRN=123` → `123`. */
        internal fun bibIrnFromUrl(url: String?): String? =
            url?.let { IRN_PARAM.find(it)?.groupValues?.get(1) }

        private fun String?.cleanTitle(): String =
            this?.trim()?.takeIf { it.isNotEmpty() } ?: UNTITLED
    }
}
