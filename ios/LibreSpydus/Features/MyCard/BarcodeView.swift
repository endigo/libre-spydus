import SwiftUI

/// A card barcode's modules as black bars (docs/native-apps.md §5.9), laid out by `BarcodeLayout`
/// across the view's whole width: whole device pixels per module, quiet zones each side, centred.
/// Drawn in a `Canvas` without antialiasing and snapped to the screen's pixel grid, so every bar
/// edge is sharp. Put it on white; it fills nothing but the bars.
struct BarcodeView: View {
    let modules: [Bool]

    @Environment(\.displayScale) private var displayScale
    /// The view's leading edge on screen, to start the bars on a pixel boundary.
    @State private var originX: CGFloat = 0

    var body: some View {
        Canvas { context, size in
            let offset = BarcodeLayout.pixelOffset(originX: originX, scale: displayScale)
            let available = BarcodeLayout.availablePixels(points: size.width, scale: displayScale, pixelOffset: offset)
            let layout = BarcodeLayout(moduleCount: modules.count, availablePixels: available)
            var path = Path()
            for bar in layout.bars(modules) {
                path.addRect(CGRect(x: (offset + CGFloat(bar.lowerBound)) / displayScale, y: 0,
                                    width: CGFloat(bar.count) / displayScale, height: size.height))
            }
            context.fill(path, with: .color(.black), style: FillStyle(antialiased: false))
        }
        .onGeometryChange(for: CGFloat.self) { proxy in
            proxy.frame(in: .global).minX
        } action: { minX in
            originX = minX
        }
    }
}
