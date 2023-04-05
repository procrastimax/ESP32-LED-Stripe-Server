use esp_idf_sys::{self as _, EspError}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::{
    modem::WifiModemPeripheral, peripheral::Peripheral, peripherals::Peripherals, prelude::*,
};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::{Configuration as HttpConfiguration, EspHttpServer},
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
};

use embedded_svc::{
    http::Method,
    io::Write,
    wifi::{ClientConfiguration, Configuration, Wifi},
};
use rgb::{RGB8, RGBA8};

use std::{net::UdpSocket, sync::RwLock};
use std::{num::NonZeroI32, sync::Arc};
use std::{thread::sleep, time::Duration};

mod rmt_rgb_led;
use crate::{
    rgb_led::RGBABrightnessExt,
    rmt_rgb_led::{show_failure, show_success, WS2812RMT},
};

mod rgb_led;

mod pwm_rgb_led;

mod api_handler;
use api_handler::{GetRGBAHandler, HelpHandler, SetRGBAHandler};

use self::pwm_rgb_led::PwmRgbLed;

use atoi::atoi;

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
    #[default([u8;0])]
    ssl_server_cert: [u8; 0],
    #[default([u8;0])]
    ssl_server_key: [u8; 0],
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

fn update_rgba_from_udp_msg(msg_arr: &[u8], rgba: &mut RGBA8) {
    // Message format is:
    // r=VALUE,g=VALUE,b=VALUE,a=VALUE

    // ASCI decimal representation:
    // \n -> 10
    // '=' -> 61
    // ',' -> 44
    // 'r' -> 114
    // 'g' -> 103
    // 'b' -> 98
    // 'a' -> 97
    let mut last_equal_sign_idx: usize = 0;
    let mut curr_channel_type: u8 = 0;

    for (idx, val) in msg_arr.iter().enumerate() {
        // found '=' -> update channel type
        if *val == 61 {
            // get channel type, as char befor '='
            if idx > 0 {
                if msg_arr[idx - 1] == 114
                    || msg_arr[idx - 1] == 103
                    || msg_arr[idx - 1] == 98
                    || msg_arr[idx - 1] == 97
                {
                    curr_channel_type = msg_arr[idx - 1];
                    last_equal_sign_idx = idx;
                } else {
                    eprintln!("received unknown channel type: {}", msg_arr[idx - 1]);
                }
            } else {
                eprintln!("received invalid data frame format!");
            }
        }
        // found ',' or newline (\n) -> update channel value and set matching rgba field
        else if *val == 44 || *val == 10 {
            // only update channel value, if channel type was set properly
            if last_equal_sign_idx > 0 && curr_channel_type > 0 {
                // calculate curr channel value from last '=' sign position
                if let Some(curr_channel_value) = atoi::<u8>(&msg_arr[last_equal_sign_idx + 1..idx])
                {
                    match curr_channel_type {
                        114 => rgba.r = curr_channel_value,
                        103 => rgba.g = curr_channel_value,
                        98 => rgba.b = curr_channel_value,
                        97 => rgba.a = curr_channel_value,
                        _ => {
                            eprintln!("found non matching channel type: {}", curr_channel_type);
                        }
                    }
                } else {
                    eprintln!(
                        "could not convert {:?} to u8 integer!",
                        &msg_arr[last_equal_sign_idx + 1..idx]
                    );
                };
            }
        }
    }
}

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let peripherals =
        Peripherals::take().expect("could not take esp peripherals, should be available");

    let mut rgb_led = WS2812RMT::new(8).expect("RGB LED should be creatable!");

    let mut pwm_led = PwmRgbLed::new(
        1.kHz().into(),
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.ledc.channel1,
        peripherals.ledc.channel2,
        peripherals.pins.gpio1,
        peripherals.pins.gpio2,
        peripherals.pins.gpio3,
    )
    .expect("could not instantiate PwmRgbLed struct from peripherals!");

    pwm_led.set_off().expect("could not turn pwm LEDs off!");

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

    let mut udp_buf = [0 as u8; 24];
    let listener = UdpSocket::bind("0.0.0.0:80").expect("Could not bin TCP listener!");
    listener
        .set_nonblocking(false)
        .expect("could not set blocking mode for udp socket!");
    listener
        .set_read_timeout(None)
        .expect("setting read timeout feailed!");

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
            let mut response = request.into_ok_response()?;
            response.write_all(b"I am alive")?;
            response.flush()?;
            Ok(())
        })
        .unwrap();
    esp_server
        .handler("/help", Method::Get, HelpHandler::new())
        .unwrap();

    let rgba_udp = rgba_values.clone();
    std::thread::spawn(move || loop {
        let (number_of_bytes, _) = listener.recv_from(&mut udp_buf).unwrap();
        if number_of_bytes < 1 {
            continue;
        }
        let mut rgba_rwlock = match rgba_udp.write() {
            Ok(val) => val,
            Err(e) => {
                eprintln!("could not get write lock for rgba_udp! Error: {}", e);
                continue;
            }
        };
        update_rgba_from_udp_msg(&udp_buf[0..number_of_bytes], &mut rgba_rwlock);
        drop(rgba_rwlock);
    });

    let mut curr_rgb = RGB8 { r: 0, g: 0, b: 0 };
    let mut last_rgb = RGB8 { r: 0, g: 0, b: 0 };

    loop {
        let rgba_read = match rgba_values.read() {
            Ok(val) => val,
            Err(e) => {
                eprintln!("could not get read lock for rgba_read! Error: {}", e);
                continue;
            }
        };
        (*rgba_read).update_channels(&mut curr_rgb);
        drop(rgba_read);

        if curr_rgb != last_rgb {
            pwm_led
                .set_color(&curr_rgb)
                .expect("could not set color for pwm!");
            last_rgb = curr_rgb;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
