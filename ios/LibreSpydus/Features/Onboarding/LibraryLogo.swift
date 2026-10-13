import LibreSpydusCore
import SwiftUI
import UIKit

/// A library's directory logo on a white rounded tile, or a monogram on a token colour while it
/// loads and when there's none (most directory entries have no logo).
struct LibraryLogo: View {
    let name: String
    let logoUrl: String?
    var size: CGFloat = 44

    @State private var loaded: UIImage?

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: size * 0.26, style: .continuous)
        ZStack {
            if let image = loaded ?? cached {
                LogoTone.isLightOnClear(image, url: logoUrl ?? "") ? Theme.Colors.background : Color.white
                Image(uiImage: image)
                    .resizable()
                    .scaledToFit()
                    .padding(size * 0.1)
            } else {
                LinearGradient(colors: [tint.opacity(0.8), tint], startPoint: .topLeading, endPoint: .bottomTrailing)
                Text(LibraryDirectory.monogram(for: name))
                    .font(.system(size: size * 0.38, weight: .bold, design: .rounded))
                    .foregroundStyle(.white)
                    .minimumScaleFactor(0.5)
            }
        }
        .frame(width: size, height: size)
        .clipShape(shape)
        .overlay(shape.strokeBorder(.white.opacity(0.08), lineWidth: 0.5))
        .accessibilityHidden(true)
        .task(id: logoUrl) {
            loaded = nil
            guard let logoUrl, cached == nil else { return }
            loaded = await LogoLoadLimiter.shared.withPermit {
                await CoverImageLoader.shared.image(for: logoUrl, pixelWidth: size * 3)
            }
        }
    }

    private var tint: Color { Theme.Colors.genre(for: name) }

    private var cached: UIImage? {
        logoUrl.flatMap { CoverImageLoader.shared.cachedImage(for: $0) }
    }
}

extension LibraryLogo {
    init(_ library: LibraryInfo, size: CGFloat = 44) {
        self.init(name: library.name, logoUrl: library.logoUrl, size: size)
    }
}

/// Some logos are white on a transparent background (City of Fremantle) and vanish on the white
/// tile; those go on navy instead. Decided once per image from a 16×16 thumbnail.
@MainActor
enum LogoTone {
    private static var verdicts: [String: Bool] = [:]

    static func isLightOnClear(_ image: UIImage, url: String) -> Bool {
        if let verdict = verdicts[url] { return verdict }
        let verdict = image.cgImage.map(measure) ?? false
        verdicts[url] = verdict
        return verdict
    }

    /// Light (mean luminance of the visible pixels > 0.85) and at least a quarter transparent.
    nonisolated static func measure(_ image: CGImage) -> Bool {
        let side = 16
        var pixels = [UInt8](repeating: 0, count: side * side * 4)
        let drawn = pixels.withUnsafeMutableBytes { buffer -> Bool in
            guard let context = CGContext(data: buffer.baseAddress, width: side, height: side, bitsPerComponent: 8,
                                          bytesPerRow: side * 4, space: CGColorSpaceCreateDeviceRGB(),
                                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: side, height: side))
            return true
        }
        guard drawn else { return false }
        var luminance = 0.0
        var coverage = 0.0
        var clearPixels = 0
        for index in stride(from: 0, to: pixels.count, by: 4) {
            let alpha = Double(pixels[index + 3]) / 255
            if alpha < 0.1 { clearPixels += 1 }
            guard alpha > 0 else { continue }
            // Premultiplied: un-premultiply, then Rec. 709 luma, weighted by alpha.
            let red = Double(pixels[index]) / 255 / alpha
            let green = Double(pixels[index + 1]) / 255 / alpha
            let blue = Double(pixels[index + 2]) / 255 / alpha
            luminance += (0.2126 * red + 0.7152 * green + 0.0722 * blue) * alpha
            coverage += alpha
        }
        guard coverage > 0 else { return false }
        return clearPixels * 4 >= side * side && luminance / coverage > 0.85
    }
}

/// Logos come from each library's own host, so the per-host limit of the image loader doesn't
/// bound them: at most 4 logo downloads run at once (docs/native-apps.md §2: ≤ 6 concurrent).
/// A row scrolled away before its turn gives up its place.
@MainActor
final class LogoLoadLimiter {
    static let shared = LogoLoadLimiter(permits: 4)

    private var available: Int
    private var waiters: [CheckedContinuation<Void, Never>] = []

    init(permits: Int) {
        available = permits
    }

    /// Runs `body` once a permit is free; nil when the task was cancelled while waiting.
    func withPermit<T>(_ body: () async -> T?) async -> T? {
        if available > 0 {
            available -= 1
        } else {
            await withCheckedContinuation { waiters.append($0) }
        }
        defer { release() }
        guard !Task.isCancelled else { return nil }
        return await body()
    }

    private func release() {
        if waiters.isEmpty {
            available += 1
        } else {
            waiters.removeFirst().resume()
        }
    }
}
