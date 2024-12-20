#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

fn main() {
    // Check if the `cfg.toml` file exists and has been filled out.
    if !std::path::Path::new("cfg.toml").exists() {
        println!(
            "cargo::warning=You need to create a `cfg.toml` file with your Wi-Fi credentials!"
        );
    }

    embuild::espidf::sysenv::output();
}
