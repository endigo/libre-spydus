//! The debug timing log (docs/native-apps.md §5.6, docs/desktop-app.md §5.7): the core's
//! `TimingSink` prints one `⏱ core …` line per page it loads, and `CoreBackend` one `⏱ app …` line
//! per call, as iOS's `TimingLog` does.

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use librespydus_core::timing::{Timing, TimingSink, set_timing_sink};

/// Installs the core's sink (debug builds only; release builds log nothing).
pub fn install() {
    if cfg!(debug_assertions) {
        set_timing_sink(Some(Arc::new(CorePrinter)));
    }
}

/// `CoreBackend`'s per-call line: `⏱ app  search: 812 ms total`.
pub fn app(operation: &str, elapsed: Duration) {
    if cfg!(debug_assertions) {
        emit(&format!("⏱ app  {operation}: {} total", milliseconds(elapsed.as_secs_f64() * 1000.0)));
    }
}

/// `⏱ core search "x": library 640 ms · parse 12 ms (182 KB)`, `… from cache`, `… failed after …`.
pub fn line(timing: &Timing) -> String {
    let name = format!("⏱ core {}", timing.operation);
    if timing.cached {
        return format!("{name}: from cache");
    }
    if timing.failed {
        return format!("{name}: failed after {}", milliseconds(timing.network_ms));
    }
    format!(
        "{name}: library {} · parse {} ({})",
        milliseconds(timing.network_ms),
        milliseconds(timing.parse_ms),
        byte_count(timing.bytes)
    )
}

/// "4.25 ms" under 10 ms, else "812 ms".
pub fn milliseconds(ms: f64) -> String {
    if ms < 10.0 { format!("{ms:.2} ms") } else { format!("{ms:.0} ms") }
}

/// "182 KB", as `ByteCountFormatter`'s file style (1 KB = 1000 bytes).
fn byte_count(bytes: u64) -> String {
    match bytes {
        0..1_000 => format!("{bytes} bytes"),
        1_000..1_000_000 => format!("{} KB", (bytes as f64 / 1e3).round()),
        _ => format!("{:.1} MB", bytes as f64 / 1e6),
    }
}

fn emit(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// Called on the core's worker threads.
struct CorePrinter;

impl TimingSink for CorePrinter {
    fn record(&self, timing: Timing) {
        emit(&line(&timing));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(cached: bool, failed: bool) -> Timing {
        Timing { operation: "search \"x\"".into(), network_ms: 640.4, parse_ms: 4.25, bytes: 182_000, cached, failed }
    }

    #[test]
    fn lines_read_like_the_ios_log() {
        assert_eq!(line(&timing(false, false)), "⏱ core search \"x\": library 640 ms · parse 4.25 ms (182 KB)");
        assert_eq!(line(&timing(true, false)), "⏱ core search \"x\": from cache");
        assert_eq!(line(&timing(false, true)), "⏱ core search \"x\": failed after 640 ms");
    }
}
