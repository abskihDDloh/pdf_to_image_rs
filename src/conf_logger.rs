use env_logger;
use std::env;

pub fn init_logger() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();
}
