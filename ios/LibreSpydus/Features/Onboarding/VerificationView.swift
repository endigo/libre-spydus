import SwiftUI
import WebKit

/// Human-verification sheet (docs/native-apps.md §6 "Verification", core/README.md "Human
/// verification"): the library's page in a web view for the **user** to complete the AWS WAF
/// check. When the library's `aws-waf-token` cookie appears, `AppModel.completeVerification`
/// stores and applies it, the sheet closes and the failed screen retries
/// (`verificationGeneration`). The app never injects scripts into or interacts with the check.
struct VerificationView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var session: VerificationSession?

    var body: some View {
        VStack(spacing: 0) {
            header
            ZStack {
                Theme.Colors.sheet
                if let session {
                    VerificationWebView(session: session) { cookie in
                        model.completeVerification(cookie: cookie)
                    }
                    overlay(for: session)
                }
            }
            .clipShape(UnevenRoundedRectangle(topLeadingRadius: Theme.Radius.card, topTrailingRadius: Theme.Radius.card))
            .ignoresSafeArea(edges: .bottom)
        }
        .screenBackground()
        .presentationBackground(Theme.Colors.background)
        .presentationDragIndicator(.visible)
        .onAppear {
            if session == nil { session = VerificationSession(origin: model.backend.origin, libraryName: model.libraryName) }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Theme.Spacing.s) {
            HStack {
                Button(Copy.Common.cancel) { dismiss() }
                    .themeFont(.bodyStrong)
                    .foregroundStyle(Theme.Colors.primary)
                Spacer()
                Button {
                    session?.reload()
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 17, weight: .semibold))
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .frame(width: 44, height: 44)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel(Copy.Verification.reload)
            }
            Text(Copy.Verification.title)
                .themeFont(.title)
                .foregroundStyle(Theme.Colors.textPrimary)
                .accessibilityAddTraits(.isHeader)
            Text(Copy.Verification.message(library: model.libraryName).capitalisedFirst)
                .themeFont(.body)
                .foregroundStyle(Theme.Colors.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
            Label(Copy.Verification.closesByItself, systemImage: "checkmark.shield")
                .themeFont(.caption)
                .foregroundStyle(Theme.Colors.textMuted)
        }
        .padding(.horizontal, Theme.Spacing.l)
        .padding(.top, Theme.Spacing.m)
        .padding(.bottom, Theme.Spacing.l)
    }

    @ViewBuilder
    private func overlay(for session: VerificationSession) -> some View {
        if let error = session.loadError {
            ErrorStateView(error: error) { session.reload() }
                .background(Theme.Colors.background)
        } else if session.isLoading, session.showingChallenge == nil {
            LoadingView(message: Copy.Verification.opening(library: model.libraryName))
                .background(Theme.Colors.background)
        } else if session.finishedWithoutCheck {
            VStack {
                Spacer()
                Banner(message: Copy.Verification.noCheck(library: model.libraryName).capitalisedFirst, tone: .info)
                    .background(RoundedRectangle(cornerRadius: Theme.Radius.field).fill(Theme.Colors.background))
                    .padding(Theme.Spacing.l)
            }
        }
    }
}

/// The verification `WKWebView`: persistent default data store, the core's User-Agent, and a
/// cookie-store observer. It only reads cookies and response headers.
private struct VerificationWebView: UIViewRepresentable {
    let session: VerificationSession
    let onVerified: (String) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(session: session, onVerified: onVerified)
    }

    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .default()
        let webView = WKWebView(frame: .zero, configuration: configuration)
        // The WAF token may be tied to the User-Agent, so the web view sends the core's.
        webView.customUserAgent = AppModel.webViewUserAgent
        webView.navigationDelegate = context.coordinator
        webView.allowsBackForwardNavigationGestures = true
        webView.backgroundColor = .white
        configuration.websiteDataStore.httpCookieStore.add(context.coordinator)
        session.webView = webView
        if let url = session.pageURL {
            webView.load(URLRequest(url: url))
        }
        return webView
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        context.coordinator.onVerified = onVerified
    }

    static func dismantleUIView(_ webView: WKWebView, coordinator: Coordinator) {
        webView.configuration.websiteDataStore.httpCookieStore.remove(coordinator)
        webView.stopLoading()
        webView.navigationDelegate = nil
    }

    @MainActor
    final class Coordinator: NSObject, WKNavigationDelegate, WKHTTPCookieStoreObserver {
        let session: VerificationSession
        var onVerified: (String) -> Void

        init(session: VerificationSession, onVerified: @escaping (String) -> Void) {
            self.session = session
            self.onVerified = onVerified
        }

        // MARK: WKHTTPCookieStoreObserver

        func cookiesDidChange(in cookieStore: WKHTTPCookieStore) {
            check(cookieStore)
        }

        // MARK: WKNavigationDelegate

        func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
            session.didStartLoading()
        }

        func webView(_ webView: WKWebView, decidePolicyFor navigationResponse: WKNavigationResponse)
            async -> WKNavigationResponsePolicy {
            if navigationResponse.isForMainFrame {
                session.didReceiveMainFrameResponse(navigationResponse.response)
            }
            return .allow
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            session.didFinishLoading()
            check(webView.configuration.websiteDataStore.httpCookieStore)
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: any Error) {
            session.didFailLoading(error)
        }

        func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!,
                     withError error: any Error) {
            session.didFailLoading(error)
        }

        private func check(_ store: WKHTTPCookieStore) {
            Task { @MainActor [session, weak self] in
                let cookies = await store.allCookies().map(VerificationCookie.Candidate.init)
                if let cookie = session.verifiedCookie(from: cookies) {
                    self?.onVerified(cookie)
                }
            }
        }
    }
}
