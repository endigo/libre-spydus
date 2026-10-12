import Foundation
import Testing
@testable import LibreSpydus

@MainActor
struct ProfileTests {
    @Test func versionTextNamesVersionAndBuild() {
        let text = ProfileView.versionText
        #expect(text.hasPrefix("Version "))
        #expect(text.contains("("))
        #expect(!text.contains("?"))
    }

    @Test func clearingVerificationForgetsTheCookie() {
        let cookies = InMemorySecretStore()
        let model = AppModel(dependencies: .testing(cookies: cookies))
        model.selectLibrary(.canning)
        model.completeVerification(cookie: "aws-waf-token=abc")
        #expect(model.isVerified)
        model.clearVerification()
        #expect(!model.isVerified)
        #expect(cookies.value(for: "https://canning.spydus.com") == nil)
    }
}
