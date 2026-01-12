# mylilygo

I am just playing around with a Lilygo dev board that contains a modem.

I am wondering if it's possible to do some basic stuff (reading/sending SMS and USSD codes) with [Embassy](https://embassy.dev/)

My end goal is to have a system that can send and receive texts using these dev boards, with other communications going through WiFi.

Note that I am a total noob when it comes to Embassy, I don't know 1% of the features (I was reading about [RTIC](https://rtic.rs/) and I was really enjoying it, until I learned that my ESP32 wasn't compatible)

## Pre-requisites

ESP32 require a specific Rust fork from Espressif, that you will need to install.

Instructions on how to install the toolchain can be found in [the Rust on ESP Book](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html).

## Configuration

You will need to set your Wi-Fi SSID and password before compiling this software.

```bash
cp .cargo/config.example.toml .cargo/config.toml
vim .cargo/config.toml # set values in the "env" section
```

I had to make it a separate file because it's not possible currently to have a file that
just contains my secrets as of now. Well, [it's possible in nightly Rust](https://github.com/rust-lang/cargo/issues/7723)
but nightly rust-analyzer doesn't seem to work well with [Helix](https://github.com/helix-editor/helix) so I
decided to do it the old way, by creating an example file and keeping it in sync manually :).

## TODO

- [ ] Setup probe-rs for faster flashing(?)
- [ ] Put AT Modem handling in a separate task & make a nice function to emit&send commands
- [ ] Make a nice and easy function to send and receive AT commands
- [ ] Implement USSD commands
  - [ ] list SMS
  - [ ] get carrier
  - [ ] query and parse balance
  - [ ] query&parse 
- [ ] get a websocket connection to the control server
- [ ] business logic
  - [ ] parse phone number from SMS
  - [ ] parse phone number from USSD code
  - [ ] parse balance from USSD code
  - [ ] status led
