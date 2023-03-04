use embedded_svc::io::Write;
use esp_idf_sys::{self as _, EspError}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::peripherals::Peripherals;

use esp_idf_svc::http::server::{Configuration as HttpConfiguration, EspHttpServer};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};

use embedded_svc::{
    http::Method,
    wifi::{ClientConfiguration, Configuration, Wifi},
};
use rgb::RGBA8;

trait RGBABrightnessExt {
    fn scale_rgb_to_brightness(&self) {}
}

impl RGBABrightnessExt for RGBA8 {
    fn scale_rgb_to_brightness(&self) {
        println!("test");
    }
}

use std::sync::RwLock;
use std::{num::NonZeroI32, sync::Arc};
use std::{thread::sleep, time::Duration};

mod led;
use led::{RGB8, WS2812RMT};

mod api_handler;
use api_handler::{GetRGBAHandler, HelpHandler, SetRGBAHandler};

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

#[derive(Debug)]
struct LedStatus;

impl LedStatus {
    const SUCCESS: RGB8 = RGB8::new(0, 10, 0);
    const FAILURE: RGB8 = RGB8::new(10, 0, 0);
    const OFF: RGB8 = RGB8::new(0, 0, 0);
}

const STATUS_LED_DURATION_SECONDS: u64 = 2;

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

fn show_success(led: &mut WS2812RMT) {
    led.set_pixel(LedStatus::SUCCESS)
        .expect("WS2812 LED should be settable to green light");
    sleep(Duration::from_secs(STATUS_LED_DURATION_SECONDS));
    led.set_pixel(LedStatus::OFF)
        .expect("WS2812 LED should be settable");
}

fn show_failure(led: &mut WS2812RMT) {
    led.set_pixel(LedStatus::FAILURE)
        .expect("WS2812 LED should be settable to red light");
    sleep(Duration::from_secs(STATUS_LED_DURATION_SECONDS));
    led.set_pixel(LedStatus::OFF)
        .expect("WS2812 LED should be settable");
}

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let mut rgb_led = WS2812RMT::new(8).expect("RGB LED should be creatable!");

    let mut wifi_driver = match create_wifi_driver() {
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

    let mut esp_server = EspHttpServer::new(&HttpConfiguration::default()).unwrap();

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
            SetRGBAHandler::new(rgba_values.clone()),
        )
        .unwrap();

    esp_server
        .fn_handler("/health", Method::Get, |request| {
            println!("Handling health request");
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
