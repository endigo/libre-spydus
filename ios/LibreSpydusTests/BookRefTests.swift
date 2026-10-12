import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

struct BookRefTests {
    @Test func fromSearchRecord() {
        let ref = BookRef(TestData.searchRecord)
        #expect(ref.id == "work:15974354")
        #expect(ref.title == "Baby-Sitter's Club graphic novels. 18, Jessi Ramsey, pet-sitter")
        #expect(ref.author == "Crenshaw, Ellen T.")
        #expect(ref.url == TestData.searchRecord.detailUrl)
        #expect(ref.coverUrl == TestData.searchRecord.coverUrls.first)
        #expect(ref.isbn == "9781546144717")
        #expect(ref.year == "2025")
        #expect(ref.formats.map(\.code) == ["BK", "EBK"])
        #expect(ref.formats[0].reserveUrl == TestData.reserveUrl)
        #expect(ref.formats[0].record == TestData.searchRecord.formats[0])
        #expect(ref.formats[1].isDigital)
        #expect(!ref.formats[0].isDigital)
    }

    @Test func fromPatronItem() {
        let item = PatronItem(
            irn: "19000001", bibIrn: "8753080", title: "The midnight library",
            bibUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78500000?QRY=IRN(8753080)&SETLVL=SET",
            author: "Haig, Matt, author", year: "2020",
            coverUrls: ["https://secure.syndetics.com/index.aspx?isbn=9780525559474/MC.GIF&client=canning"],
            isbn: "9780525559474", fields: ["Due": "27 Oct 2026"], actions: []
        )
        let ref = BookRef(item)
        #expect(ref.id == "bib:8753080")
        #expect(ref.title == "The midnight library")
        #expect(ref.url == item.bibUrl)
        #expect(ref.coverUrl == item.coverUrls.first)
        #expect(ref.formats.isEmpty)
    }

    @Test func fromRelatedItem() {
        let item = RelatedItem(title: "The long night",
                               bibUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78554992?BRN=467086&SETLVL=",
                               brn: "467086", author: "White, Christian", year: "2026", isbn: "9781399197809", coverUrls: [])
        let ref = BookRef(item)
        #expect(ref.id == "brn:467086")
        #expect(ref.url == item.bibUrl)
        #expect(ref.coverUrl == nil)
    }

    @Test func fallsBackToUrlThenTitleKeys() {
        let noIrn = PatronItem(irn: nil, bibIrn: "  ", title: nil, bibUrl: "https://x.example/bib", author: "",
                               year: nil, coverUrls: [], isbn: nil, fields: [:], actions: [])
        let ref = BookRef(noIrn)
        #expect(ref.id == "url:https://x.example/bib")
        #expect(ref.title == "Untitled")
        #expect(ref.author == nil)

        let bare = RelatedItem(title: "Mire", bibUrl: nil, brn: nil, author: nil, year: nil, isbn: nil, coverUrls: [])
        #expect(BookRef(bare).id == "title:mire")
    }

    @Test func updatedWithDetailPrefersBookmarkAndKeepsId() {
        let ref = BookRef(TestData.searchRecord)
        let detail = RecordDetail(
            bibIrn: nil, brn: "377677", title: "Other title", seriesTitle: nil, isbn: nil, coverUrls: [], summary: nil,
            details: [], bookmarkUrl: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=377677",
            related: RecordRelated(similar: nil, author: nil, series: nil, versions: nil)
        )
        let updated = ref.updated(with: detail)
        #expect(updated.id == ref.id)
        #expect(updated.url == detail.bookmarkUrl)
        #expect(updated.title == ref.title)
    }

    @Test func codableRoundTrip() throws {
        var ref = BookRef(TestData.searchRecord)
        ref.savedAt = Date(timeIntervalSince1970: 1_000)
        let decoded = try JSONDecoder().decode(BookRef.self, from: JSONEncoder().encode(ref))
        #expect(decoded == ref)
    }

    @Test func detailRouteFromUrl() {
        let ref = BookRef(url: "https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1")
        #expect(ref.id == "url:https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=1")
        #expect(Route.detail(ref) == Route.detail(ref))
    }
}
