//! The `librespydus` binary: parses the debug launch flags and runs the app.

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let options = match librespydus_desktop::LaunchOptions::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("librespydus: {error}");
            std::process::exit(2);
        }
    };
    librespydus_desktop::app::run(options);
}
