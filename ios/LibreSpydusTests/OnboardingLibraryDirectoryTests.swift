import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

struct OnboardingLibraryDirectoryTests {
    static func library(_ name: String, region: String, country: String? = nil, origin: String? = nil) -> LibraryInfo {
        let slug = name.lowercased().filter(\.isLetter)
        return LibraryInfo(id: slug, name: name, origin: origin ?? "https://\(slug).example.com", logoUrl: nil,
                           country: country, region: region, isTest: false)
    }

    static let sample: [LibraryInfo] = [
        library("Southampton City Libraries", region: "UK", country: "United Kingdom"),
        library("City of Canning", region: "AU", country: "Australia"),
        library("Wellington City Libraries", region: "NZ", country: "New Zealand"),
        library("MOE", region: "SG", country: "Singapore"),
        library("Brazoria County Library System", region: "US", country: "United States"),
        library("Somewhere Else", region: "XX", country: "Atlantis"),
        library("albany", region: "AU", country: "Australia"),
        library("Hutt City Libraries", region: "nz", country: "New Zealand"),
    ]

    @Test func sectionsFollowTheRegionOrderAndSkipEmptyRegions() {
        let sections = LibraryDirectory.sections(Self.sample)
        #expect(sections.map(\.region) == [.au, .nz, .us, .uk, .sg, .other])
        #expect(sections.map(\.region.title) ==
            ["Australia", "New Zealand", "United States", "United Kingdom", "Singapore", "Other"])
        let withoutUK = LibraryDirectory.sections(Self.sample.filter { $0.region != "UK" })
        #expect(!withoutUK.map(\.region).contains(.uk))
    }

    @Test func unknownAndLowercaseRegionCodesAreGrouped() {
        #expect(LibraryDirectory.Region(code: "XX") == .other)
        #expect(LibraryDirectory.Region(code: "OTHER") == .other)
        #expect(LibraryDirectory.Region(code: "nz") == .nz)
        let nz = LibraryDirectory.sections(Self.sample).first { $0.region == .nz }
        #expect(nz?.libraries.map(\.name) == ["Hutt City Libraries", "Wellington City Libraries"])
    }

    @Test func librariesAreSortedByNameAsPeopleReadIt() {
        let libraries = [
            Self.library("Library 10", region: "AU"),
            Self.library("library 2", region: "AU"),
            Self.library("Albany", region: "AU", origin: "https://b.example.com"),
            Self.library("albany", region: "AU", origin: "https://a.example.com"),
            Self.library("Ängelholm", region: "AU"),
        ]
        let names = LibraryDirectory.sections(libraries)[0].libraries.map(\.name)
        #expect(names.prefix(2).allSatisfy { $0.lowercased() == "albany" })
        #expect(names.firstIndex(of: "library 2")! < names.firstIndex(of: "Library 10")!)
        // Same name: by origin, so the order is stable.
        let albanies = LibraryDirectory.sorted(libraries).filter { $0.name.lowercased() == "albany" }
        #expect(albanies.map(\.origin) == ["https://a.example.com", "https://b.example.com"])
    }

    @Test func filterMatchesNameCountryAndRegion() {
        func names(_ query: String) -> [String] {
            LibraryDirectory.sections(Self.sample, query: query).flatMap(\.libraries).map(\.name)
        }
        #expect(names("") .count == Self.sample.count)
        #expect(names("   ").count == Self.sample.count)
        #expect(names("canning") == ["City of Canning"])
        #expect(names("CANNING") == ["City of Canning"])
        #expect(names("zealand") == ["Hutt City Libraries", "Wellington City Libraries"])
        #expect(names("kingdom") == ["Southampton City Libraries"])
        #expect(names("uk") == ["Southampton City Libraries"])
        #expect(names("city new") == ["Hutt City Libraries", "Wellington City Libraries"])
        #expect(names("atlantis") == ["Somewhere Else"])
        #expect(names("nowhere-at-all").isEmpty)
        #expect(LibraryDirectory.sections(Self.sample, query: "nowhere-at-all").isEmpty)
    }

    @Test func filterIgnoresDiacritics() {
        let libraries = [Self.library("Bibliothèque Montréal", region: "OTHER", country: "Canada")]
        #expect(LibraryDirectory.filter(libraries, query: "montreal").count == 1)
        #expect(LibraryDirectory.filter(libraries, query: "bibliotheque").count == 1)
    }

    @Test func regionCaveats() {
        #expect(LibraryDirectory.Region.uk.caveat == "Asks you to confirm you’re human")
        #expect(LibraryDirectory.Region.sg.caveat == "May be unreachable outside school networks")
        for region in [LibraryDirectory.Region.au, .nz, .us, .other] {
            #expect(region.caveat == nil)
        }
    }

    @Test func monograms() {
        #expect(LibraryDirectory.monogram(for: "City of Canning") == "CC")
        #expect(LibraryDirectory.monogram(for: "AFTRS") == "A")
        #expect(LibraryDirectory.monogram(for: "Banana Shire Library & Information Services") == "BS")
        #expect(LibraryDirectory.monogram(for: "the library") == "L")
        #expect(LibraryDirectory.monogram(for: "") == "?")
    }

    @Test func hostOfOrigin() {
        #expect(LibraryDirectory.host(of: "https://canning.spydus.com") == "canning.spydus.com")
        #expect(LibraryDirectory.host(of: "https://hutt.spydus.co.nz:8443") == "hutt.spydus.co.nz")
    }
}
