use esp_idf_sys::{self as _, EspError}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::peripherals::Peripherals;

use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};

use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};

use std::num::NonZeroI32;
use std::{thread::sleep, time::Duration};

mod led;

#[toml_cfg::toml_config]
struct Settings {
    #[default("")]
    ssid: &'static str,
    #[default("")]
    passphrase: &'static str,
    #[default(15)]
    wifi_timeout_wait_seconds: u16,
    #[default(5)]
    wifi_connection_attempts: u16,
}
fn create_wifi_driver() -> Result<EspWifi<'static>, EspError> {
    println!("Creating wifi driver");
    let peripherals =
        Peripherals::take().expect("could not take esp peripherals, should be available");
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;

    wifi_driver.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SETTINGS.ssid.into(),
        password: SETTINGS.passphrase.into(),
        ..Default::default()
    }))?;
    return Ok(wifi_driver);
}

fn connect_to_wifi(wifi_driver: &mut EspWifi) -> Result<(), EspError> {
    println!("Connecting to wifi: {:?}", SETTINGS.ssid);
    wifi_driver.start()?;
    wifi_driver.connect()?;

    // wait until connection is established or time has passed
    for _ in 0..SETTINGS.wifi_timeout_wait_seconds {
        if wifi_driver.is_connected()? {
            println!("Connection established");
            return Ok(());
        } else {
            sleep(Duration::from_secs(1));
        }
    }

    // 12295 - ESP_ERR_WIFI_CONN
    return Err(EspError::from_non_zero(NonZeroI32::new(12295).unwrap()));
}

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let mut wifi_driver = match create_wifi_driver() {
        Ok(x) => x,
        Err(e) => {
            // when the wifi driver creation fails, the program should stop
            eprintln!("Could not create esp32 wifi driver! Error: {:?}", e,);
            return Err(e);
        }
    };

    // try multiple times to connect to wifi if first one did not suceed
    for i in 0..SETTINGS.wifi_connection_attempts {
        match connect_to_wifi(&mut wifi_driver) {
            Ok(_) => {
                println!("Successfully connected to wifi!");
                break;
            }
            Err(e) => {
                eprintln!(
                    "Could not yet connect to wifi - trying again ({:?}/{:?})! Error {:?}",
                    i + 1,
                    SETTINGS.wifi_connection_attempts,
                    e
                );
                if i + 1 == SETTINGS.wifi_connection_attempts {
                    eprintln!(
                        "Could not connect to wifi after {:?} attemps, quitting...",
                        SETTINGS.wifi_connection_attempts
                    )
                }
            }
        };
    }

    return Ok(());
}
