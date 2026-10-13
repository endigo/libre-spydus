//! Opt-in timing log: for each page a [`crate::SpydusClient`] call loads, how long the library
//! took to send it and how long it took to parse. Off until the app installs a [`TimingSink`]
//! with [`set_timing_sink`] (the apps do so in debug builds); with none installed the cost is two
//! clock reads per page.

use std::future::Future;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use crate::SpydusError;

/// One page a call loaded (or served from the core's cache).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Timing {
    /// What was loaded, e.g. `search "harry potter"`, `availability`, `record`, `loans`.
    pub operation: String,
    /// Request sent → whole page received, in milliseconds (redirects included). For a failed
    /// request, how long it took to fail.
    pub network_ms: f64,
    /// Parsing the page into the result, in milliseconds (0 for a failed request).
    pub parse_ms: f64,
    /// The page's size as text, after decompression (0 for a failed request or a cache hit).
    pub bytes: u64,
    /// Served from the core's cache: no request was sent and both times are 0.
    pub cached: bool,
    /// The request failed; `network_ms` is how long it took.
    pub failed: bool,
}

/// Receives a [`Timing`] for every page loaded while it is installed. Called on the core's
/// worker threads, so an implementation must be thread-safe and should return quickly.
#[uniffi::export(with_foreign)]
pub trait TimingSink: Send + Sync {
    fn record(&self, timing: Timing);
}

static SINK: RwLock<Option<Arc<dyn TimingSink>>> = RwLock::new(None);

/// Installs `sink` for every client (replacing any earlier one), or with `None` turns the
/// timing log off.
#[uniffi::export]
pub fn set_timing_sink(sink: Option<Arc<dyn TimingSink>>) {
    *SINK.write().unwrap_or_else(PoisonError::into_inner) = sink;
}

fn report(timing: impl FnOnce() -> Timing) {
    // Cloned out so the sink runs without the lock held.
    let sink = SINK.read().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(sink) = sink {
        sink.record(timing());
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Awaits `fetch` (one or more requests ending in a page), then parses it with `parse`, and
/// reports both times as `operation`.
pub(crate) async fn fetch_and_parse<T>(
    operation: &str,
    fetch: impl Future<Output = Result<String, SpydusError>>,
    parse: impl FnOnce(&str) -> T,
) -> Result<T, SpydusError> {
    let started = Instant::now();
    let html = match fetch.await {
        Ok(html) => html,
        Err(e) => {
            let network_ms = ms(started.elapsed());
            report(|| Timing {
                operation: operation.to_owned(),
                network_ms,
                parse_ms: 0.0,
                bytes: 0,
                cached: false,
                failed: true,
            });
            return Err(e);
        }
    };
    let network = started.elapsed();
    let started = Instant::now();
    let value = parse(&html);
    let parse_ms = ms(started.elapsed());
    report(|| Timing {
        operation: operation.to_owned(),
        network_ms: ms(network),
        parse_ms,
        bytes: html.len() as u64,
        cached: false,
        failed: false,
    });
    Ok(value)
}

/// Reports a cache hit for `operation`.
pub(crate) fn cache_hit(operation: &str) {
    report(|| Timing {
        operation: operation.to_owned(),
        network_ms: 0.0,
        parse_ms: 0.0,
        bytes: 0,
        cached: true,
        failed: false,
    });
}
