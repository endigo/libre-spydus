package com.librespydus.app.feature.onboarding

import com.librespydus.app.core.Formatting
import uniffi.librespydus_core.LibraryInfo
import java.net.URI
import java.text.Collator
import java.text.Normalizer
import java.util.Locale

/** Library picker regions in display order (docs/native-apps.md §6). Unknown codes are [OTHER]. */
enum class PickerRegion(val code: String) {
    AU("AU"),
    NZ("NZ"),
    US("US"),
    UK("UK"),
    SG("SG"),
    OTHER("OTHER"),
    ;

    /** Section header ("Australia", …). */
    val title: String get() = Formatting.regionLabel(code)

    /** What a row in this region warns about (core/README.md "Current regional support"). */
    val caveat: String?
        get() = when (this) {
            UK -> "Asks you to verify you're human"
            SG -> "May be unreachable outside school networks"
            AU, NZ, US, OTHER -> null
        }

    companion object {
        fun of(code: String?): PickerRegion =
            entries.firstOrNull { it.code.equals(code?.trim(), ignoreCase = true) } ?: OTHER
    }
}

/** One picker section: a region and its libraries, sorted by name. */
data class PickerSection(val region: PickerRegion, val libraries: List<LibraryInfo>)

/** The library picker's pure logic: regions, search and sections (no I/O; unit-tested). */
object LibraryPickerLogic {
    /**
     * [libraries] matching [query], grouped by region in [PickerRegion] order and sorted by name;
     * empty regions are left out.
     */
    fun sections(libraries: List<LibraryInfo>, query: String = ""): List<PickerSection> {
        val grouped = filter(libraries, query).groupBy { PickerRegion.of(it.region) }
        return PickerRegion.entries.mapNotNull { region ->
            grouped[region]?.takeIf { it.isNotEmpty() }?.let { PickerSection(region, sorted(it)) }
        }
    }

    /**
     * Every whitespace-separated word of [query] must appear in the name, the country or the
     * region's name (case- and diacritic-insensitive), or be the region code: "city aus",
     * "wellington", "kingdom", "uk".
     */
    fun filter(libraries: List<LibraryInfo>, query: String): List<LibraryInfo> {
        val words = query.split(Regex("\\s+")).filter { it.isNotEmpty() }
        if (words.isEmpty()) return libraries
        return libraries.filter { library ->
            val haystack = listOf(library.name, library.country.orEmpty(), PickerRegion.of(library.region).title)
                .map(::fold)
            words.all { word ->
                library.region.equals(word, ignoreCase = true) || haystack.any { fold(word) in it }
            }
        }
    }

    /** By name as a person reads it ("Library 2" before "Library 10"), then by origin. */
    fun sorted(libraries: List<LibraryInfo>): List<LibraryInfo> =
        libraries.sortedWith(compareBy<LibraryInfo, String>(NaturalOrder) { it.name }.thenBy { it.origin })

    /** Up to two initials for the logo placeholder: "City of Canning" → "CC". */
    fun monogram(name: String): String {
        val skipped = setOf("of", "the", "and", "&", "-", "–")
        val letters = name.split(Regex("[\\s/]+"))
            .filter { it.isNotEmpty() && it.lowercase() !in skipped }
            .take(2)
            .mapNotNull { word -> word.firstOrNull { it.isLetter() } ?: word.firstOrNull() }
        return if (letters.isEmpty()) "?" else letters.joinToString("").uppercase()
    }

    /** `https://canning.spydus.com` → `canning.spydus.com`. */
    fun host(origin: String): String =
        runCatching { URI(origin).host }.getOrNull()?.takeIf { it.isNotEmpty() } ?: origin

    /** Lower-cased without accents, for matching ("Montréal" ~ "montreal"). */
    internal fun fold(text: String): String =
        Normalizer.normalize(text, Normalizer.Form.NFD).replace(Regex("\\p{M}+"), "").lowercase(Locale.ROOT)

    /** Case- and accent-insensitive, with digit runs compared as numbers. */
    private object NaturalOrder : Comparator<String> {
        private val collator: Collator = Collator.getInstance(Locale.ENGLISH).apply { strength = Collator.PRIMARY }
        private val chunks = Regex("\\d+|\\D+")

        override fun compare(a: String, b: String): Int {
            val left = chunks.findAll(a).map { it.value }.toList()
            val right = chunks.findAll(b).map { it.value }.toList()
            for (i in 0 until minOf(left.size, right.size)) {
                val l = left[i]
                val r = right[i]
                val result = if (l[0].isDigit() && r[0].isDigit()) {
                    l.trimStart('0').length.compareTo(r.trimStart('0').length).takeIf { it != 0 }
                        ?: l.trimStart('0').compareTo(r.trimStart('0'))
                } else {
                    collator.compare(l, r)
                }
                if (result != 0) return result
            }
            return left.size.compareTo(right.size)
        }
    }
}
