mod log;

fn main() {
    log::init_logger();
    let mut app = app::App::new();
    app.add_plugin(window::init_window);
    app.add_plugin(artisan::init_artisan);

    app.run();
}
