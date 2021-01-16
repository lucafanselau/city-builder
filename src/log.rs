use log::LevelFilter;
use simplelog::{ConfigBuilder, TermLogger, TerminalMode};
pub fn init_logger() {
    let config = ConfigBuilder::new()
        .add_filter_ignore_str("gfx_backend_vulkan")
        // .set_ignore_level(LevelFilter::Info)
        .build();

    TermLogger::init(LevelFilter::Info, config, TerminalMode::Mixed).unwrap()
}
