import SafariServices
import SwiftUI

/// A web page shown in the in-app browser (`SFSafariViewController`): a Featured `.web` link or
/// a collection that lives on the library's website (§5.10).
struct BrowserPage: Identifiable, Hashable {
    let url: URL
    var id: String { url.absoluteString }
}

enum InAppBrowser {
    /// `text` as a URL the browser can open (http or https only; anything else would crash it).
    static func webURL(_ text: String) -> URL? {
        guard let url = URL(string: text.trimmingCharacters(in: .whitespacesAndNewlines)),
              let scheme = url.scheme?.lowercased(), scheme == "https" || scheme == "http",
              url.host() != nil else { return nil }
        return url
    }
}

extension View {
    /// Presents `page` full screen in the in-app browser; its Done button clears `page`. Root
    /// sheets (sign in again, verification) wait until it has gone.
    func inAppBrowser(_ page: Binding<BrowserPage?>) -> some View {
        modifier(InAppBrowserModifier(page: page))
    }
}

private struct InAppBrowserModifier: ViewModifier {
    @Binding var page: BrowserPage?
    @Environment(AppModel.self) private var model

    func body(content: Content) -> some View {
        content
            .onChange(of: page != nil) { _, showing in
                if showing { model.isPresentingFeatureSheet = true }
            }
            .fullScreenCover(item: $page, onDismiss: { model.isPresentingFeatureSheet = false }) { page in
                SafariView(url: page.url) { self.page = nil }
                    .ignoresSafeArea()
            }
    }
}

/// `SFSafariViewController` in SwiftUI, navy bars to match the app.
private struct SafariView: UIViewControllerRepresentable {
    let url: URL
    let onDone: () -> Void

    func makeCoordinator() -> Coordinator { Coordinator(onDone: onDone) }

    func makeUIViewController(context: Context) -> SFSafariViewController {
        let controller = SFSafariViewController(url: url)
        controller.preferredBarTintColor = UIColor(Theme.Colors.background)
        controller.preferredControlTintColor = .white
        controller.dismissButtonStyle = .done
        controller.delegate = context.coordinator
        return controller
    }

    func updateUIViewController(_ controller: SFSafariViewController, context: Context) {}

    final class Coordinator: NSObject, SFSafariViewControllerDelegate {
        let onDone: () -> Void

        init(onDone: @escaping () -> Void) {
            self.onDone = onDone
        }

        func safariViewControllerDidFinish(_ controller: SFSafariViewController) {
            onDone()
        }
    }
}
