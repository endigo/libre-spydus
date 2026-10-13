package com.librespydus.app.feature.onboarding

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.librespydus_core.LibraryInfo

class LibraryPickerLogicTest {
    companion object {
        fun library(name: String, region: String, country: String? = null, origin: String? = null): LibraryInfo {
            val slug = name.lowercase().filter { it.isLetter() }
            return LibraryInfo(slug, name, origin ?: "https://$slug.example.com", null, country, region, false)
        }

        val sample = listOf(
            library("Southampton City Libraries", "UK", "United Kingdom"),
            library("City of Canning", "AU", "Australia"),
            library("Wellington City Libraries", "NZ", "New Zealand"),
            library("MOE", "SG", "Singapore"),
            library("Brazoria County Library System", "US", "United States"),
            library("Somewhere Else", "XX", "Atlantis"),
            library("albany", "AU", "Australia"),
            library("Hutt City Libraries", "nz", "New Zealand"),
        )
    }

    @Test
    fun sectionsFollowTheRegionOrderAndSkipEmptyRegions() {
        val sections = LibraryPickerLogic.sections(sample)
        assertEquals(PickerRegion.entries.toList(), sections.map { it.region })
        assertEquals(
            listOf("Australia", "New Zealand", "United States", "United Kingdom", "Singapore", "Other"),
            sections.map { it.region.title },
        )
        val withoutUk = LibraryPickerLogic.sections(sample.filter { it.region != "UK" })
        assertFalse(PickerRegion.UK in withoutUk.map { it.region })
    }

    @Test
    fun unknownAndLowercaseRegionCodesAreGrouped() {
        assertEquals(PickerRegion.OTHER, PickerRegion.of("XX"))
        assertEquals(PickerRegion.OTHER, PickerRegion.of("OTHER"))
        assertEquals(PickerRegion.OTHER, PickerRegion.of(null))
        assertEquals(PickerRegion.NZ, PickerRegion.of("nz"))
        val nz = LibraryPickerLogic.sections(sample).first { it.region == PickerRegion.NZ }
        assertEquals(listOf("Hutt City Libraries", "Wellington City Libraries"), nz.libraries.map { it.name })
    }

    @Test
    fun librariesAreSortedByNameAsPeopleReadIt() {
        val libraries = listOf(
            library("Library 10", "AU"),
            library("library 2", "AU"),
            library("Albany", "AU", origin = "https://b.example.com"),
            library("albany", "AU", origin = "https://a.example.com"),
            library("Ängelholm", "AU"),
        )
        val names = LibraryPickerLogic.sections(libraries)[0].libraries.map { it.name }
        assertTrue(names.take(2).all { it.lowercase() == "albany" })
        assertTrue(names.indexOf("library 2") < names.indexOf("Library 10"))
        // Same name: by origin, so the order is stable.
        val albanies = LibraryPickerLogic.sorted(libraries).filter { it.name.lowercase() == "albany" }
        assertEquals(listOf("https://a.example.com", "https://b.example.com"), albanies.map { it.origin })
    }

    @Test
    fun filterMatchesNameCountryAndRegion() {
        fun names(query: String) = LibraryPickerLogic.sections(sample, query).flatMap { it.libraries }.map { it.name }
        assertEquals(sample.size, names("").size)
        assertEquals(sample.size, names("   ").size)
        assertEquals(listOf("City of Canning"), names("canning"))
        assertEquals(listOf("City of Canning"), names("CANNING"))
        assertEquals(listOf("Hutt City Libraries", "Wellington City Libraries"), names("zealand"))
        assertEquals(listOf("Southampton City Libraries"), names("kingdom"))
        assertEquals(listOf("Southampton City Libraries"), names("uk"))
        assertEquals(listOf("Hutt City Libraries", "Wellington City Libraries"), names("city new"))
        assertEquals(listOf("Somewhere Else"), names("atlantis"))
        assertTrue(names("nowhere-at-all").isEmpty())
        assertTrue(LibraryPickerLogic.sections(sample, "nowhere-at-all").isEmpty())
    }

    @Test
    fun filterIgnoresDiacritics() {
        val libraries = listOf(library("Bibliothèque Montréal", "OTHER", "Canada"))
        assertEquals(1, LibraryPickerLogic.filter(libraries, "montreal").size)
        assertEquals(1, LibraryPickerLogic.filter(libraries, "bibliotheque").size)
        assertEquals(1, LibraryPickerLogic.filter(libraries, "MONTRÉAL").size)
    }

    @Test
    fun regionCaveats() {
        assertEquals("Asks you to confirm you’re human", PickerRegion.UK.caveat)
        assertEquals("May be unreachable outside school networks", PickerRegion.SG.caveat)
        for (region in listOf(PickerRegion.AU, PickerRegion.NZ, PickerRegion.US, PickerRegion.OTHER)) assertNull(region.caveat)
    }

    @Test
    fun monograms() {
        assertEquals("CC", LibraryPickerLogic.monogram("City of Canning"))
        assertEquals("A", LibraryPickerLogic.monogram("AFTRS"))
        assertEquals("BS", LibraryPickerLogic.monogram("Banana Shire Library & Information Services"))
        assertEquals("L", LibraryPickerLogic.monogram("the library"))
        assertEquals("?", LibraryPickerLogic.monogram(""))
    }

    @Test
    fun hostOfOrigin() {
        assertEquals("canning.spydus.com", LibraryPickerLogic.host("https://canning.spydus.com"))
        assertEquals("hutt.spydus.co.nz", LibraryPickerLogic.host("https://hutt.spydus.co.nz:8443"))
        assertEquals("not a url", LibraryPickerLogic.host("not a url"))
    }
}
