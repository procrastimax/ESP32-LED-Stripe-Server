# ESP32C3 Wifi LED API
## Description
This project is a simple rust implementation for the ESP32C3-Devkit-M1 to be used as a HTTP(S) REST-API to control a 3-color LED RGB stripe via PWM/ RMT.
The server can be requested with commands shown [here](#api-documentation).

The server uses RGB channels to control the proportion of each color channel in the light. The fourth channel, the alpha/ brightness channel, scales down/ up to control the light's brightness over all 3 color channels.

**TODO: Link Android REST-API client**

## Setup
Setup rust nightly with the ESP toolchain. You'll need the rust nightly toolchain, ldproxy and setup support for RISC-V.
A short guide for this can be found [here](https://esp-rs.github.io/book/installation/installation.html) (only ldproxy, RISC-V setup).
To properly flash the binary on to the board, you need the espflash tool, can be installed with:
`cargo install cargo-espflash`.

## Usage
1. please make sure that your rust setup is complete (see [here](#setup)).
2. rename `cfg.toml.example` to `cfg.toml` and enter your wifi-credentials in there
3. (optional if you want to use the android client, or want to use a self-signed ssl certificate): Generate a certificate authority certificate, use this certificate to sign a server ssl certificate to use server's HTTPS API. For this please take a look at the scripts at `/certs`. After generating the certificates, copy the byte arrays for the server certificate and server key in the responding fields `cfg.toml`. Sadly I could not figure out a better way, but after this you need to edit the lines in the `struct Settings` that specify the expected length of the struct's `ssl_server_cert` and `ssl_server_key` fields. If you use rust-analyzer as LSP, the linter should tell you the number you have to enter as the expected length for the byte array.
4. run `cargo espflash /dev/ttyUSB0 --speed 921600 -s 4MB --monitor --release --partition-table=partition.csv` to compile and flash the code on your board
5. enjoy controlling your RGB LED stripe with the ESPC3

## API Documentation

| Command  | Description | Returns | Status Codes  |
|---|---|---|---|
| \health | Indicates if the server is running | Returns string "I am alive" | 200 (OK) |
| \help   |  Shows a help page | Returns help text as string | 200 (OK) / 400 (Error)  |
| \setRGBA?r=RED&g=GREEN&b=Blue&a=BRIGHTNESS | Sets the RGBA values according to their values, not all values need to be specified at the same time | all RGBA values in CSV format without header after 'set' request | 200 (Ok) / 400 (Error)
| \getRGBA   | Retrieve current set RBGBA values  | all RGBA values in CSV format without header | 200 (OK) / 400 (Error)


## Schematic
**TODO**
