mod augmentation;
mod config;

use robjetives_log::prepare_loggers;

// use this, then no need to import mod config...
//use crate::load_config;

fn main() {
    if let Err(e) = app_init("./config/default/loggers.toml".to_string()) {
        panic!("app_init error: {}", e);
    }
    println!("Hello, world!");
}

pub fn app_init(config_file: String) -> Result<(), Box<dyn std::error::Error>> {
    // default -> "./config/default/loggers.toml"
    let result = prepare_loggers(config_file);
    if result.is_err() {
        return Err(Box::new(result.err().unwrap()));
    }
    tracing::info!("otel_broccoli application init successfully !!!");

    Ok(())
}
