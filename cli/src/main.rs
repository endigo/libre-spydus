//! `spydus`: LibreSpydus for the terminal (docs/cli.md).

fn main() {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(librespydus_cli::error::EXIT_FAILED);
        }
    };
    let mut world = librespydus_cli::app::World::real();
    let code = runtime.block_on(librespydus_cli::app::run(std::env::args().collect(), &mut world));
    drop(runtime);
    std::process::exit(code);
}
