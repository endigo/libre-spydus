import Foundation
import Testing
@testable import LibreSpydus

struct SecretStoreTests {
    /// The real Keychain on the simulator, under a throwaway service.
    @Test func keychainRoundTrip() {
        let store = KeychainSecretStore(service: "com.librespydus.tests.\(UUID().uuidString)")
        defer { store.removeAll() }
        let origin = "https://canning.spydus.com"
        #expect(store.value(for: origin) == nil)
        store.setValue("session-1", for: origin)
        #expect(store.value(for: origin) == "session-1")
        store.setValue("session-2", for: origin)
        #expect(store.value(for: origin) == "session-2")
        store.setValue("other", for: "https://hutt.spydus.co.nz")
        store.setValue(nil, for: origin)
        #expect(store.value(for: origin) == nil)
        #expect(store.value(for: "https://hutt.spydus.co.nz") == "other")
        store.removeAll()
        #expect(store.value(for: "https://hutt.spydus.co.nz") == nil)
    }

    @Test func inMemory() {
        let store = InMemorySecretStore(["a": "1"])
        #expect(store.value(for: "a") == "1")
        store.setValue(nil, for: "a")
        #expect(store.value(for: "a") == nil)
    }
}
