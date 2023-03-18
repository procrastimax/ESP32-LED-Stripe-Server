use esp_idf_sys::{self as _, EspError}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::{
    modem::WifiModemPeripheral, peripheral::Peripheral, peripherals::Peripherals, prelude::*,
};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::{Configuration as HttpConfiguration, EspHttpServer},
    nvs::EspDefaultNvsPartition,
    tls::X509,
    wifi::EspWifi,
};

use embedded_svc::{
    http::Method,
    io::Write,
    wifi::{ClientConfiguration, Configuration, Wifi},
};
use rgb::RGBA8;

use std::sync::RwLock;
use std::{num::NonZeroI32, sync::Arc};
use std::{thread::sleep, time::Duration};

mod rmt_rgb_led;
use crate::rmt_rgb_led::{show_failure, show_success, WS2812RMT};

mod rgb_led;

mod pwm_rgb_led;

mod api_handler;
use api_handler::{GetRGBAHandler, HelpHandler, SetRGBAHandler};

use self::pwm_rgb_led::PwmRgbLed;

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
    #[default([u8;1])]
    ssl_server_cert: [u8; 939],
    #[default([u8;1])]
    ssl_server_key: [u8; 1192],
}

fn create_wifi_driver<M: WifiModemPeripheral>(
    modem: impl Peripheral<P = M> + 'static,
) -> Result<EspWifi<'static>, EspError> {
    println!("Creating wifi driver");
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi_driver = EspWifi::new(modem, sys_loop, Some(nvs))?;

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

    let peripherals =
        Peripherals::take().expect("could not take esp peripherals, should be available");

    let mut rgb_led = WS2812RMT::new(8).expect("RGB LED should be creatable!");

    let mut wifi_driver = match create_wifi_driver(peripherals.modem) {
        Ok(x) => x,
        Err(e) => {
            // when the wifi driver creation fails, the program should stop
            eprintln!("Could not create esp32 wifi driver! Error: {:?}", e,);
            show_failure(&mut rgb_led);
            return Err(e);
        }
    };

    // try multiple times to connect to wifi if first one did not suceed
    for i in 0..SETTINGS.wifi_connection_attempts {
        match connect_to_wifi(&mut wifi_driver) {
            Ok(_) => {
                println!("Successfully connected to wifi!");
                show_success(&mut rgb_led);
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
                    );
                    show_failure(&mut rgb_led);
                }
            }
        };
    }

    let pwm_led = PwmRgbLed::new(
        1.kHz().into(),
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.ledc.channel1,
        peripherals.ledc.channel2,
        peripherals.pins.gpio1,
        peripherals.pins.gpio2,
        peripherals.pins.gpio3,
    )
    .unwrap();

    let server_certificate = X509::der(&SETTINGS.ssl_server_cert);
    let private_key = X509::der(&SETTINGS.ssl_server_key);

    let mut https_config = HttpConfiguration::default();
    https_config.server_certificate = Some(server_certificate);
    https_config.private_key = Some(private_key);

    let mut esp_server = EspHttpServer::new(&https_config).unwrap();

    let rgba_values = Arc::new(RwLock::new(RGBA8::new(0, 0, 0, 255)));

    esp_server
        .handler(
            "/getRGBA",
            Method::Get,
            GetRGBAHandler::new(rgba_values.clone()),
        )
        .unwrap();

    esp_server
        .handler(
            "/setRGBA",
            Method::Get,
            SetRGBAHandler::new(rgba_values.clone(), RwLock::new(pwm_led)),
        )
        .unwrap();

    esp_server
        .fn_handler("/health", Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(b"I am alive")?;
            response.flush()?;
            Ok(())
        })
        .unwrap();
    esp_server
        .handler("/help", Method::Get, HelpHandler::new())
        .unwrap();

    loop {
        sleep(Duration::from_millis(1000));
    }
}
