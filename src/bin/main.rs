#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use bt_hci::cmd::le::LeSetTransmitPowerReportingEnable;
use bt_hci::controller::ExternalController;
use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::OutputConfig;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, gpio::Output};
use esp_radio::ble::controller::BleConnector;
use trouble_host::prelude::*;
use {esp_backtrace as _, esp_println as _};

extern crate alloc;

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

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
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(&radio_init, peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 1>::new(transport);
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let _stack = trouble_host::new(ble_controller, &mut resources);

    // Initialize the modem
    let mut modem_power_pin = Output::new(
        peripherals.GPIO4,
        esp_hal::gpio::Level::High,
        OutputConfig::default(),
    );
    modem_power_pin.set_high();

    // Try turning off the led
    let mut led = Output::new(
        peripherals.GPIO12,
        esp_hal::gpio::Level::High,
        OutputConfig::default(),
    );
    led.set_high();

    info!("initializing io...");
    let _io = esp_hal::gpio::Io::new(peripherals.IO_MUX);
    info!("initialized io.");

    info!("setting up uart...");
    let uart_config = esp_hal::uart::Config::default().with_baudrate(115200);
    let mut uart = esp_hal::uart::Uart::new(peripherals.UART1, uart_config)
        .expect("setting up UART1 (AT commands)")
        .with_tx(peripherals.GPIO26)
        .with_rx(peripherals.GPIO27);
    info!("set up uart!");

    // TODO: Spawn some tasks
    let _ = spawner;

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
        break;
    }

    loop {
        Timer::after(Duration::from_secs(1)).await;
        uart.write(b"AT+CSQ\r\n")
            .expect("error writing signal quality AT command");
        Timer::after(Duration::from_secs(1)).await;
        let mut buf: [u8; 128] = [0; 128];
        let amount = uart.read_buffered(&mut buf).expect("reading from uart");
        if amount == 0 {
            warn!("AT still not replying to us :(");
            continue;
        }

        info!(
            "we got a signal quality answer! {}",
            core::str::from_utf8(&buf).expect("parsing at response to utf8")
        );

        uart.write(b"AT+CSCS=\"GSM\"\r\n")
            .expect("setting gsm encoding");
        uart.write(b"AT+CPMS?\r\n")
            .expect("writing sms quantity enquiry");
        uart.write(b"AT+CMGL=\"ALL\"\r\n")
            .expect("reading all the messages");

        Timer::after(Duration::from_secs(1)).await;
        uart.write(b"AT+CUSD=1,\"*133#\",15\r\n")
            .expect("error writing balance AT command");
        Timer::after(Duration::from_secs(1)).await;
        let mut buf: [u8; 128] = [0; 128];
        let amount = uart.read_buffered(&mut buf).expect("reading from uart");
        if amount == 0 {
            warn!("AT still not replying to us :(");
            continue;
        }

        info!(
            "we got a balance answer! {}",
            core::str::from_utf8(&buf).expect("parsing at response to utf8")
        );
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
