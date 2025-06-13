mod config;

// use this, then no need to import mod config...
//use crate::load_config;

fn main() {
    println!("Hello, world!");
    let result = config::load_config(
        "config/default".to_string(),
        "tests".to_string(),
        "config.toml".to_string(),
        "stdout_test.toml".to_string(),
    );
}
