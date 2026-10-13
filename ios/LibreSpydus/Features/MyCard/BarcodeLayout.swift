import CoreGraphics

/// Where a card barcode's modules go across the available width (docs/native-apps.md §5.9), in
/// device pixels: a whole number of pixels per module, as wide as fits with at least
/// `quietZone` white modules each side, centred. Scanners need crisp, even bars, so nothing is
/// scaled or smoothed.
struct BarcodeLayout: Equatable, Sendable {
    /// White modules each side of the bars, at least.
    static let quietZone = 10

    /// Device pixels per module: floor(available / (modules + 2 × quietZone)), at least 1.
    let modulePixels: Int
    /// From the leading edge of the available width to the first module.
    let leadingPixels: Int
    /// The bars' total width.
    let barsPixels: Int

    init(moduleCount: Int, availablePixels: Int) {
        modulePixels = max(1, availablePixels / (moduleCount + 2 * Self.quietZone))
        barsPixels = moduleCount * modulePixels
        // Rounded down: an odd pixel left over goes on the trailing side.
        leadingPixels = (availablePixels - barsPixels).dividedRoundingDown(by: 2)
    }

    /// The black bars as pixel ranges from the leading edge, a run of black modules as one bar.
    func bars(_ modules: [Bool]) -> [Range<Int>] {
        var bars: [Range<Int>] = []
        var start: Int?
        for (index, black) in modules.enumerated() {
            if black, start == nil { start = index }
            if !black, let first = start {
                bars.append(pixels(first..<index))
                start = nil
            }
        }
        if let first = start { bars.append(pixels(first..<modules.count)) }
        return bars
    }

    private func pixels(_ modules: Range<Int>) -> Range<Int> {
        (leadingPixels + modules.lowerBound * modulePixels)..<(leadingPixels + modules.upperBound * modulePixels)
    }

    /// Whole device pixels in `points` at `scale`, from `pixelOffset` on (a few thousandths of a
    /// pixel of floating-point error don't lose a pixel).
    static func availablePixels(points: CGFloat, scale: CGFloat, pixelOffset: CGFloat = 0) -> Int {
        max(0, Int((points * scale - pixelOffset + 0.001).rounded(.down)))
    }

    /// How far (in device pixels, 0 ..< 1) a view whose leading edge is at `originX` points must
    /// move right so its drawing starts on a pixel boundary.
    static func pixelOffset(originX: CGFloat, scale: CGFloat) -> CGFloat {
        let pixels = originX * scale
        let offset = pixels.rounded(.up) - pixels
        return offset < 0.001 || offset > 0.999 ? 0 : offset
    }
}

private extension Int {
    /// Floor division (Swift's `/` rounds toward zero).
    func dividedRoundingDown(by divisor: Int) -> Int {
        let quotient = self / divisor
        return (self % divisor != 0 && (self < 0) != (divisor < 0)) ? quotient - 1 : quotient
    }
}
