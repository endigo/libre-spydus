//! One public search through `CoreBackend`, with the debug timing log on: prints the
//! `⏱ core …` and `⏱ app …` lines (a public GET; no account, no POST).
//!
//!     cargo run --example public_search -- [origin] [query]

use librespydus_desktop::backend::{CoreBackend, LibraryBackend};

fn main() {
    let mut args = std::env::args().skip(1);
    let origin = args.next().unwrap_or_else(|| "https://canning.spydus.com".into());
    let query = args.next().unwrap_or_else(|| "harry potter".into());
    librespydus_desktop::timing::install();
    let backend = CoreBackend::new(Some(&origin), None);
    let first = futures::executor::block_on(backend.search(query.clone())).expect("search");
    println!("{} records, {:?} in all", first.records.len(), first.total_results);
    // The same search again comes from the core's cache.
    futures::executor::block_on(backend.search(query)).expect("search");
}
