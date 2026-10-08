mod app;
mod core;
mod recording;
mod sketches;

fn main() {
    if std::env::args().any(|arg| arg == "--validate-attractors") {
        if !sketches::validate_attractors() {
            std::process::exit(1);
        }
        return;
    }
    app::run();
}
