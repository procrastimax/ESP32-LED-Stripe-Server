use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

#[toml_cfg::toml_config]
struct WifiCredentials {
    #[default("")]
    ssid: &'static str,
    #[default("")]
    passphrase: &'static str,
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    println!("Hello, world!");
    println!("{:?}", WIFI_CREDENTIALS.passphrase);
}
