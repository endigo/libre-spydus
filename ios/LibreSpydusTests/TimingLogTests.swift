import Foundation
import LibreSpydusCore
import Testing
@testable import LibreSpydus

struct TimingLogTests {
    private func timing(networkMs: Double = 0, parseMs: Double = 0, bytes: UInt64 = 0,
                        cached: Bool = false, failed: Bool = false) -> Timing {
        Timing(operation: "search \"harry potter\"", networkMs: networkMs, parseMs: parseMs, bytes: bytes,
               cached: cached, failed: failed)
    }

    @Test func aLoadedPageShowsTheLibraryWaitAndTheParse() {
        let line = TimingLog.line(for: timing(networkMs: 5303.4, parseMs: 1.468, bytes: 198_000))
        #expect(line.hasPrefix("⏱ core search \"harry potter\": library 5303 ms · parse 1.47 ms ("))
        #expect(line.contains("KB"))
    }

    @Test func aCacheHitAndAFailureSaySo() {
        #expect(TimingLog.line(for: timing(cached: true)) == "⏱ core search \"harry potter\": from cache")
        #expect(TimingLog.line(for: timing(networkMs: 30_012, failed: true))
            == "⏱ core search \"harry potter\": failed after 30012 ms")
    }

    @Test func durationsAreMilliseconds() {
        #expect(TimingLog.milliseconds(.milliseconds(5306)) == "5306 ms")
        #expect(TimingLog.milliseconds(.microseconds(1500)) == "1.50 ms")
    }
}
