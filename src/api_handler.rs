use std::borrow::Borrow;
use std::sync::{Arc, RwLock};

use embedded_svc::http::server::{Handler, Request};
use embedded_svc::http::Query;
use embedded_svc::io::Write;
use esp_idf_svc::http::server::EspHttpConnection;
use rgb::RGBA8;
use url::Url;

// TODO: proper error handling when receiving request, parsing URLs, etc. -> respond with error codes

pub struct GetRGBAHandler {
    pub rgba: Arc<RwLock<RGBA8>>,
}

impl GetRGBAHandler {
    pub fn new(rgba: Arc<RwLock<RGBA8>>) -> GetRGBAHandler {
        return GetRGBAHandler { rgba };
    }
}

impl Handler<EspHttpConnection<'_>> for GetRGBAHandler {
    fn handle(&self, c: &mut EspHttpConnection<'_>) -> embedded_svc::http::server::HandlerResult {
        println!("Handling get RGBA request");
        let req = Request::wrap(c);
        let mut response = req.into_ok_response()?;
        let rgba = self.rgba.read()?;
        response.write_fmt(format_args!("{},{},{},{}", rgba.r, rgba.g, rgba.b, rgba.a))?;
        response.flush()?;
        Ok(())
    }
}

pub struct SetRGBAHandler {
    pub rgba: Arc<RwLock<RGBA8>>,
}

impl SetRGBAHandler {
    pub fn new(rgba: Arc<RwLock<RGBA8>>) -> SetRGBAHandler {
        return SetRGBAHandler { rgba };
    }
}

impl Handler<EspHttpConnection<'_>> for SetRGBAHandler {
    fn handle(&self, c: &mut EspHttpConnection<'_>) -> embedded_svc::http::server::HandlerResult {
        println!("Handling set RGBA request");

        let req = Request::wrap(c);

        // create a dummy base url
        let base_url = Url::parse("http://localhost").unwrap();
        let url = base_url.join(req.uri()).unwrap();

        let mut new_rgba = self.rgba.write().unwrap();

        for pair in url.query_pairs() {
            let color = pair.0;
            let value = pair.1;

            match color.borrow() {
                "r" => {
                    new_rgba.r = value.to_string().parse::<u8>().unwrap();
                }
                "g" => {
                    new_rgba.g = value.to_string().parse::<u8>().unwrap();
                }
                "b" => {
                    new_rgba.b = value.to_string().parse::<u8>().unwrap();
                }
                "a" => {
                    new_rgba.a = value.to_string().parse::<u8>().unwrap();
                }
                _ => {
                    println!("Unknown query parameter: {}-{}!", color, value)
                }
            }
        }

        let mut response = req.into_ok_response()?;
        response.write_fmt(format_args!(
            "{},{},{},{}",
            new_rgba.r, new_rgba.g, new_rgba.b, new_rgba.a
        ))?;
        response.flush()?;
        Ok(())
    }
}

pub struct HelpHandler {}

impl HelpHandler {
    pub fn new() -> HelpHandler {
        return HelpHandler {};
    }
}

impl Handler<EspHttpConnection<'_>> for HelpHandler {
    fn handle(&self, c: &mut EspHttpConnection<'_>) -> embedded_svc::http::server::HandlerResult {
        println!("Handling Help request");

        let help_text = "<h1>Help - Supported functions</h1>
            <b>/help</b> - shows this help page</br>
            <b>/setRGBA?r=VALUE&g=VALUE&b=VALUE&a=VALUE</b> - sets the r,g,b and brightness/ alpha values</br>
            <b>/getRGBA</b> - gets the r,g,b and brightness/alpha values (in this order) as CSV without a CSV header</br>";

        let req = Request::wrap(c);
        let mut response = req.into_ok_response()?;
        response.write_all(help_text.as_bytes())?;
        response.flush()?;
        Ok(())
    }
}