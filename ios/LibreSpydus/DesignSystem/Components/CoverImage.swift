import ImageIO
import SwiftUI
import UIKit

/// A book cover: loads `url` asynchronously (cached), shows a placeholder generated from the
/// title meanwhile and when there's no cover. Rounded corners + soft shadow.
struct CoverImage: View {
    let url: String?
    let title: String
    var author: String?
    var width: CGFloat = Theme.Cover.shelfWidth
    var shadow = true

    @State private var loaded: UIImage?

    var body: some View {
        let height = Theme.Cover.height(forWidth: width)
        let shape = RoundedRectangle(cornerRadius: Theme.Radius.cover, style: .continuous)
        ZStack {
            if let image = loaded ?? cached {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFill()
            } else {
                CoverPlaceholder(title: title, author: author)
            }
        }
        .frame(width: width, height: height)
        .clipShape(shape)
        .overlay(shape.strokeBorder(.white.opacity(0.06), lineWidth: 0.5))
        .themeShadow(shadow ? .card : Theme.Shadow(color: .clear, opacity: 0, radius: 0, y: 0))
        .accessibilityElement()
        .accessibilityLabel("Cover of \(title)")
        .task(id: url) {
            loaded = nil
            guard let url else { return }
            loaded = await CoverImageLoader.shared.image(for: url, pixelWidth: width * 3)
        }
    }

    private var cached: UIImage? {
        url.flatMap { CoverImageLoader.shared.cachedImage(for: $0) }
    }
}

/// The generated cover: title (and author) on a token colour picked from the title.
struct CoverPlaceholder: View {
    let title: String
    var author: String?

    var body: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let base = Theme.Colors.genre(for: title)
            ZStack(alignment: .topLeading) {
                LinearGradient(colors: [base.opacity(0.85), base], startPoint: .top, endPoint: .bottom)
                    .background(Theme.Colors.surfaceElevated)
                Rectangle()
                    .fill(.black.opacity(0.14))
                    .frame(width: max(3, width * 0.06))
                VStack(alignment: .leading, spacing: width * 0.04) {
                    Text(title)
                        .font(.system(size: max(9, width * 0.13), weight: .bold, design: .serif))
                        .foregroundStyle(.white)
                        .lineLimit(5)
                        .minimumScaleFactor(0.6)
                    Spacer(minLength: 0)
                    if let name = Formatting.authorName(author) {
                        Text(name.uppercased())
                            .font(.system(size: max(6, width * 0.075), weight: .semibold))
                            .tracking(0.5)
                            .foregroundStyle(.white.opacity(0.85))
                            .lineLimit(2)
                    }
                }
                .padding(.vertical, width * 0.1)
                .padding(.leading, width * 0.14)
                .padding(.trailing, width * 0.08)
            }
        }
        .accessibilityHidden(true)
    }
}

/// Downloads covers once, downsamples them, and keeps them in memory (and URLCache on disk).
/// Syndetics answers a missing cover with a 1×1 image; that counts as no cover.
@MainActor
final class CoverImageLoader {
    static let shared = CoverImageLoader()

    private let cache = NSCache<NSString, UIImage>()
    private var missing: Set<String> = []
    private var inFlight: [String: Task<UIImage?, Never>] = [:]
    private let session: URLSession

    init() {
        let configuration = URLSessionConfiguration.default
        configuration.urlCache = URLCache(memoryCapacity: 8 << 20, diskCapacity: 100 << 20)
        configuration.requestCachePolicy = .returnCacheDataElseLoad
        configuration.timeoutIntervalForRequest = 20
        configuration.httpMaximumConnectionsPerHost = 4
        session = URLSession(configuration: configuration)
        cache.countLimit = 300
    }

    func cachedImage(for url: String) -> UIImage? {
        cache.object(forKey: url as NSString)
    }

    /// The cover at `url`, or nil when there's none (or it failed to load).
    func image(for url: String, pixelWidth: CGFloat) async -> UIImage? {
        if let image = cachedImage(for: url) { return image }
        if missing.contains(url) { return nil }
        if let task = inFlight[url] { return await task.value }
        guard let remote = URL(string: url), ["http", "https"].contains(remote.scheme?.lowercased() ?? "") else {
            missing.insert(url)
            return nil
        }
        let session = self.session
        let task = Task<UIImage?, Never> { [weak self] in
            let outcome = await Self.download(remote, session: session, maxPixel: max(pixelWidth, 64))
            switch outcome {
            case .image(let image):
                self?.cache.setObject(image, forKey: url as NSString)
                return image
            case .noCover:
                self?.missing.insert(url)
                return nil
            case .failed:
                return nil // not remembered: retried next time the cover is shown
            }
        }
        inFlight[url] = task
        let image = await task.value
        inFlight[url] = nil
        return image
    }

    private enum Outcome: Sendable {
        case image(UIImage)
        /// The server answered, but with no usable image (Syndetics' 1×1 "no cover").
        case noCover
        /// Transport error or non-2xx: worth trying again later.
        case failed
    }

    @concurrent
    private nonisolated static func download(_ url: URL, session: URLSession, maxPixel: CGFloat) async -> Outcome {
        guard let (data, response) = try? await session.data(from: url),
              (response as? HTTPURLResponse).map({ (200..<300).contains($0.statusCode) }) ?? true else { return .failed }
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let pixelWidth = properties[kCGImagePropertyPixelWidth] as? Int,
              let pixelHeight = properties[kCGImagePropertyPixelHeight] as? Int,
              pixelWidth > 4, pixelHeight > 4 else { return .noCover }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: Int(maxPixel / Theme.Cover.aspect),
        ]
        guard let thumbnail = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return .noCover }
        return .image(UIImage(cgImage: thumbnail))
    }
}
