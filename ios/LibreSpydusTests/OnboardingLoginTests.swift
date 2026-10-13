import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

/// The sign-in form against `FakeBackend` (password "ok" signs in; anything else is
/// `CredentialsRejected`). No network.
@MainActor
struct OnboardingLoginTests {
    func signedOutModel() -> AppModel {
        let model = AppModel(dependencies: .testing())
        model.selectLibrary(.canning)
        model.loginNeeded = true
        return model
    }

    @Test func needsACardNumberAndAPIN() {
        let form = LoginFormModel()
        #expect(!form.canSubmit)
        form.cardNumber = "   "
        form.pin = "1234"
        #expect(!form.canSubmit)
        form.cardNumber = " 2000123 "
        #expect(form.canSubmit)
        form.pin = ""
        #expect(!form.canSubmit)
    }

    @Test func rejectedCredentialsShowInline() async {
        let model = signedOutModel()
        let form = LoginFormModel()
        form.cardNumber = "2000123"
        form.pin = "wrong"
        let signedIn = await form.submit(to: model)
        #expect(!signedIn)
        #expect(form.error?.kind == .credentialsRejected)
        #expect(form.error?.message == "Card number or PIN not recognised.")
        #expect(!form.isSigningIn)
        #expect(!model.isSignedIn)
        #expect(model.rootSheet == .login)
        form.clearError()
        #expect(form.error == nil)
    }

    @Test func successSignsInClosesTheSheetAndLoadsTheAccount() async {
        let model = signedOutModel()
        let form = LoginFormModel()
        form.cardNumber = " 2000123\n"
        form.pin = "ok"
        let signedIn = await form.submit(to: model)
        #expect(signedIn)
        #expect(form.error == nil)
        #expect(form.pin.isEmpty)
        #expect(model.isSignedIn)
        #expect(model.rootSheet == nil)
        #expect(model.accountName == "Fake Borrower")
    }

    @Test func emptyFormDoesNothing() async {
        let model = signedOutModel()
        let form = LoginFormModel()
        #expect(await form.submit(to: model) == false)
        #expect(form.error == nil)
    }
}
