import CoreGraphics
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// `BarcodeLayout`: whole device pixels per module, quiet zones, centring (docs/native-apps.md §5.9).
struct MyCardLayoutTests {
    @Test func wholePixelsPerModuleAsWideAsFits() {
        // 156 modules ("DEMO0012345" in Code 128) on a 370 pt card at 3×: 1110 px.
        let layout = BarcodeLayout(moduleCount: 156, availablePixels: 1110)
        #expect(layout.modulePixels == 6) // floor(1110 / (156 + 20))
        #expect(layout.barsPixels == 936)
        #expect(layout.leadingPixels == 87)
    }

    @Test func quietZonesOfAtLeastTenModulesEachSide() {
        for modules in [35, 79, 156, 200, 299] {
            for available in [modules + 20, 640, 1110, 1179, 1290] where available >= modules + 20 {
                let layout = BarcodeLayout(moduleCount: modules, availablePixels: available)
                let trailing = available - layout.leadingPixels - layout.barsPixels
                #expect(layout.leadingPixels >= BarcodeLayout.quietZone * layout.modulePixels)
                #expect(trailing >= BarcodeLayout.quietZone * layout.modulePixels)
                // One more pixel per module wouldn't fit with the quiet zones.
                #expect((modules + 20) * (layout.modulePixels + 1) > available)
            }
        }
    }

    @Test func barsAreCentred() {
        let even = BarcodeLayout(moduleCount: 100, availablePixels: 1000)
        #expect(even.modulePixels == 8)
        #expect(even.leadingPixels == 100)
        #expect(1000 - even.leadingPixels - even.barsPixels == 100)
        // An odd pixel left over goes on the trailing side.
        let odd = BarcodeLayout(moduleCount: 100, availablePixels: 1001)
        #expect(odd.leadingPixels == 100)
        #expect(1001 - odd.leadingPixels - odd.barsPixels == 101)
    }

    @Test func atLeastOnePixelPerModule() {
        let exact = BarcodeLayout(moduleCount: 80, availablePixels: 100)
        #expect(exact.modulePixels == 1)
        #expect(exact.leadingPixels == 10)
        // Too narrow even at 1 px: still 1 px, centred, overflowing both sides evenly.
        let narrow = BarcodeLayout(moduleCount: 200, availablePixels: 101)
        #expect(narrow.modulePixels == 1)
        #expect(narrow.barsPixels == 200)
        #expect(narrow.leadingPixels == -50)
        let empty = BarcodeLayout(moduleCount: 0, availablePixels: 0)
        #expect(empty.modulePixels == 1)
    }

    @Test func runsOfBlackModulesBecomeBars() {
        let layout = BarcodeLayout(moduleCount: 7, availablePixels: 54) // 2 px per module, 20 px in
        #expect(layout.modulePixels == 2)
        #expect(layout.leadingPixels == 20)
        let modules = [true, true, false, true, false, false, true]
        #expect(layout.bars(modules) == [20..<24, 26..<28, 32..<34])
        #expect(layout.bars([false, false]).isEmpty)
        #expect(layout.bars([]) == [])
    }

    @Test func pixelGridMaths() {
        #expect(BarcodeLayout.availablePixels(points: 370, scale: 3) == 1110)
        #expect(BarcodeLayout.availablePixels(points: 370, scale: 3, pixelOffset: 0.5) == 1109)
        #expect(BarcodeLayout.availablePixels(points: 123.333333, scale: 3) == 370) // 369.999999
        #expect(BarcodeLayout.availablePixels(points: 0, scale: 3, pixelOffset: 0.5) == 0)
        #expect(BarcodeLayout.pixelOffset(originX: 16, scale: 3) == 0)
        #expect(BarcodeLayout.pixelOffset(originX: 16.5, scale: 3) == 0.5)
        #expect(BarcodeLayout.pixelOffset(originX: 1.0 / 3, scale: 3) == 0)
        #expect(abs(BarcodeLayout.pixelOffset(originX: 16.25, scale: 2) - 0.5) < 0.0001)
    }

    @Test func aRealCardFitsAPhoneWithRoomToSpare() throws {
        // The demo card in Code 128 and a Canning-length number in Code 39, on the narrowest phone
        // (iPhone SE: 375 pt − 2 × 16 pt margins, 2×).
        for (number, symbology) in [("DEMO0012345", BarcodeSymbology.code128), ("CL114D0001234", .code39)] {
            let barcode = try #require(cardBarcode(cardNumber: number, symbology: symbology))
            let available = BarcodeLayout.availablePixels(points: 343, scale: 2)
            let layout = BarcodeLayout(moduleCount: barcode.modules.count, availablePixels: available)
            #expect(layout.modulePixels >= 2, "\(number): \(barcode.modules.count) modules")
            #expect(layout.leadingPixels >= 10 * layout.modulePixels)
        }
    }
}
