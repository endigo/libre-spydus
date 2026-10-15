import Foundation
import LibreSpydusCore
import SwiftUI
import Testing
@testable import LibreSpydus

/// The iPad's wide layout (docs/native-apps.md §6.1): when it applies, how many cards and covers a
/// row holds, when the book page's right column shows, and the iPad's words where an iPhone says
/// "phone".
@MainActor
struct IPadLayoutTests {
    @Test func aRegularWidthWindowIsWideAndAnyOtherIsCompact() {
        #expect(AppLayout(.regular) == .wide)
        #expect(AppLayout(.compact) == .compact)
        #expect(AppLayout(nil) == .compact)
    }

    @Test func cardGridsHoldAsMany300PointCardsAsFitUpToThree() {
        #expect(PageLayout.cardColumns(width: 0) == 1)
        #expect(PageLayout.cardColumns(width: 611) == 1)
        // Two cards and the gap between them.
        #expect(PageLayout.cardColumns(width: 612) == 2)
        #expect(PageLayout.cardColumns(width: 740) == 2)
        #expect(PageLayout.cardColumns(width: 1008) == 3)
        #expect(PageLayout.cardColumns(width: 2000) == 3)
    }

    @Test func coverGridsFillTheirWidthWithTwoToSixCovers() {
        let narrow = PageLayout.coverGrid(width: 200)
        #expect(narrow.columns == 2)
        #expect(narrow.coverWidth == 92)

        let column = PageLayout.coverGrid(width: 528)
        #expect(column.columns == 4)
        #expect(column.coverWidth == 120)

        let wide = PageLayout.coverGrid(width: 2000)
        #expect(wide.columns == 6)
        // The covers and the gaps between them fill the width.
        #expect(wide.coverWidth * 6 + PageLayout.coverSpacing * 5 <= 2000)
        #expect(wide.coverWidth * 6 + PageLayout.coverSpacing * 5 > 2000 - 6)
    }

    @Test func theBookPagesRightColumnShowsWhileLoadingAndWhenTheBookHasOtherBooks() async {
        let noReport: BookDetailModel.Reporter = { UserFacingError($0, libraryName: "Canning") }
        let ref = BookRef(TestData.searchRecord)
        let bk = ref.formats[0].availabilityUrl!

        // Loading: the column holds the place.
        let loading = BookDetailModel(ref: ref)
        #expect(DetailRelatedColumn.isShown(for: loading))

        // Loaded, with related links: shown (the lists load into it).
        let related = DetailBackend(records: [bk: .success(DetailFixtures.midnightLibrary)])
        await loading.load(backend: related, report: noReport)
        #expect(loading.hasRelatedPanels)
        #expect(DetailRelatedColumn.isShown(for: loading))

        // Loaded without any: the details take the page.
        var alone = DetailFixtures.midnightLibrary
        alone.related = RecordRelated(similar: nil, author: nil, series: nil, versions: nil)
        let noRelated = BookDetailModel(ref: ref)
        await noRelated.load(backend: DetailBackend(records: [bk: .success(alone)]), report: noReport)
        #expect(noRelated.record.value != nil)
        #expect(!DetailRelatedColumn.isShown(for: noRelated))
    }

    @Test func anIPadSaysIPadWhereAnIPhoneSaysPhone() {
        CopyDevice.$isPadOverride.withValue(true) {
            #expect(Copy.Picks.subtitlePhone == "Saved on this iPad")
            #expect(Copy.Picks.emptyPhoneMessage == "Tap the bookmark on a book to save it on this iPad.")
            #expect(Copy.Picks.loadFailed == "Couldn’t load My Picks from this iPad.")
            #expect(Copy.Picks.saveFailed == "Couldn’t save My Picks on this iPad.")
            #expect(Copy.Picks.offerTitle == "Add this iPad’s picks to your library account?")
            #expect(Copy.Picks.offerMessage(count: 1) == "1 book saved on this iPad can be added to My Picks on your library account.")
            #expect(Copy.Picks.offerMessage(count: 2) == "2 books saved on this iPad can be added to My Picks on your library account.")
            #expect(Copy.Profile.verified == "Verified on this iPad")
            #expect(Copy.Biometrics.offerMessage(library: "Canning", method: "Touch ID").contains("stay on this iPad, locked by Touch ID"))
            #expect(Copy.Login.privacyNoteBiometrics(library: "Canning", method: "Touch ID").contains("on this iPad only"))
        }
        CopyDevice.$isPadOverride.withValue(false) {
            #expect(Copy.Picks.subtitlePhone == "Saved on this phone")
            #expect(Copy.Profile.verified == "Verified on this phone")
            #expect(Copy.Picks.offerMessage(count: 2) == "2 books saved on this phone can be added to My Picks on your library account.")
        }
    }
}
