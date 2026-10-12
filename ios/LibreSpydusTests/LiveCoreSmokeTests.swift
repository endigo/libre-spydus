import LibreSpydusCore
import Testing

extension Tag {
    /// Needs the network (public GETs to a real library).
    @Tag static var live: Self
}

/// Proves the Rust core works on iOS: FFI, async (tokio ↔ Swift concurrency) and TLS (rustls).
/// One public GET search on Canning; skip with `-skip-testing:LibreSpydusTests/LiveCoreSmokeTests`.
@Suite(.tags(.live))
struct LiveCoreSmokeTests {
    @Test(.timeLimit(.minutes(1)))
    func canningSearchReturnsRecords() async throws {
        let client = SpydusClient(origin: "https://canning.spydus.com")
        let results = try await client.search(query: "harry potter")
        #expect(!results.records.isEmpty)
        #expect(results.records.contains { ($0.title ?? "").localizedCaseInsensitiveContains("harry potter") })
        #expect(client.origin() == "https://canning.spydus.com")
    }
}
