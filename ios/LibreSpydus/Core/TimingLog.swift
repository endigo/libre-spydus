import Foundation
import LibreSpydusCore

/// Debug builds only: prints a "⏱" line for every library page the core loads (how long the
/// library took to send it, and how long it took to parse) and for every `CoreBackend` call (its
/// total time as the app saw it, including the hand-off from the core). The lines show in
/// Xcode's console, or with `xcrun devicectl device process launch --console … com.librespydus`.
enum TimingLog {
    static func install() {
        #if DEBUG
        setTimingSink(sink: CorePrinter())
        #endif
    }

    /// `CoreBackend`'s per-call line.
    static func app(_ operation: String, _ elapsed: Duration) {
        emit("⏱ app  \(operation): \(milliseconds(elapsed)) total")
    }

    static func line(for timing: Timing) -> String {
        let name = "⏱ core \(timing.operation)"
        if timing.cached { return "\(name): from cache" }
        if timing.failed { return "\(name): failed after \(milliseconds(timing.networkMs))" }
        let size = ByteCountFormatter.string(fromByteCount: Int64(timing.bytes), countStyle: .file)
        return "\(name): library \(milliseconds(timing.networkMs)) · parse \(milliseconds(timing.parseMs)) (\(size))"
    }

    static func milliseconds(_ elapsed: Duration) -> String {
        let (seconds, attoseconds) = elapsed.components
        return milliseconds(Double(seconds) * 1000 + Double(attoseconds) / 1e15)
    }

    static func milliseconds(_ ms: Double) -> String {
        ms < 10 ? String(format: "%.2f ms", ms) : String(format: "%.0f ms", ms)
    }

    private static func emit(_ line: String) {
        print(line)
        // stdout is block-buffered when it isn't a terminal (devicectl --console).
        fflush(stdout)
    }

    /// Called on the core's worker threads.
    private final class CorePrinter: TimingSink {
        func record(timing: Timing) {
            TimingLog.emit(TimingLog.line(for: timing))
        }
    }
}
