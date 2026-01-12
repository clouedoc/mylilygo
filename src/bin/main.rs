#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::OutputConfig;
use esp_hal::peripherals::{GPIO4, GPIO26, GPIO27, UART1, WIFI};
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, gpio::Output};
use esp_radio::wifi::ClientConfig;
use {esp_backtrace as _, esp_println as _};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn websocket_task(wifi: WIFI<'static>) {
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");

    let (mut controller, interfaces) = esp_radio::wifi::new(&radio_init, wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    controller
        .set_config(&esp_radio::wifi::ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(env!("WIFI_SSID").into())
                .with_password(env!("WIFI_PASSWORD").into())
                .with_auth_method(esp_radio::wifi::AuthMethod::Wpa2Personal),
        ))
        .expect("setting wifi controller configuration");
    info!("[ws] set config");
    info!("[ws] wifi ssid: {}", env!("WIFI_SSID"));

    controller
        .start_async()
        .await
        .expect("starting wifi controller");
    info!("[ws] started wifi controller");

    controller
        .connect_async()
        .await
        .expect("connecting to wifi network");
    info!("[ws] connected wifi controller");
}

#[embassy_executor::task]
async fn modem_task(
    power_pin: GPIO4<'static>,
    tx_pin: GPIO26<'static>,
    rx_pin: GPIO27<'static>,
    uart1: UART1<'static>,
) {
    // Set modem power to on.
    let mut modem_power_pin = Output::new(
        power_pin,
        esp_hal::gpio::Level::High,
        OutputConfig::default(),
    );
    modem_power_pin.set_high();

    info!("[modem] setting up uart...");
    let uart_config = esp_hal::uart::Config::default().with_baudrate(115200);
    let mut uart = esp_hal::uart::Uart::new(uart1, uart_config)
        .expect("setting up modem's UART")
        .with_tx(tx_pin)
        .with_rx(rx_pin);
    info!("[modem] set up uart!");

    // Wait until the modem is ready
    loop {
        uart.write(b"AT\r\n").expect("error writing AT command");
        Timer::after(Duration::from_secs(1)).await;
        let mut buf: [u8; 128] = [0; 128];
        let amount = uart.read_buffered(&mut buf).expect("reading from uart");

        if amount == 0 {
            warn!("AT still not replying to us :(");
            continue;
        }

        info!(
            "looks like the modem is ready! response to AT command: {}",
            core::str::from_utf8(&buf).expect("parsing at response to utf8")
        );
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.1.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    // COEX needs more RAM - so we've added some more
    // NOTE: Coex means that bluetooth and wifi are sharing the same
    // antenna. Since we are not using Bluetooth, I think that it might not be required?
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    // Spawn the task that will handle the modem
    spawner
        .spawn(modem_task(
            peripherals.GPIO4,
            peripherals.GPIO26,
            peripherals.GPIO27,
            peripherals.UART1,
        ))
        .expect("spawning modem task");

    spawner
        .spawn(websocket_task(peripherals.WIFI))
        .expect("spawning websocket task");

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples

    // Infinite loop
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
