#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use core::str::from_utf8;

use cyw43::JoinOptions;
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = include_str!("../.ssid");
const WIFI_PASSWORD: &str = include_str!("../.password");

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

async fn html_response(
    socket: &mut TcpSocket<'_>,
    content: &'static [u8],
    status: &'static [u8],
    content_type: &'static [u8],
) {
    macro_rules! try_write_all {
        ($data:expr) => {
            if socket.write_all($data).await.is_err() {
                defmt::warn!("Socket write failed");
                return;
            }
        };
    }
    try_write_all!(b"HTTP/1.1 ");
    try_write_all!(status);
    try_write_all!(b"\r\nContent-Type: ");
    try_write_all!(content_type);
    try_write_all!(b"\r\nContent-Length: ");
    let mut buffer = itoa::Buffer::new();
    let printed = buffer.format(content.len());
    try_write_all!(printed.as_bytes());
    try_write_all!(b"\r\nConnection: close");
    try_write_all!(b"\r\n\r\n");
    try_write_all!(content);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    // And now we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    let mut relay = Output::new(p.PIN_6, Level::Low);

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;
        info!("Listening on TCP:80...");
        if let Err(e) = socket.accept(80).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        let n = match socket.read(&mut buf).await {
            Ok(0) => {
                warn!("read EOF");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                warn!("read error: {:?}", e);
                break;
            }
        };

        let request_str = from_utf8(&buf[..n]).unwrap_or("");
        info!("rxd {}", request_str);

        if request_str.starts_with("POST /on HTTP/1.1") {
            relay.set_high();
            html_response(&mut socket, b"Powered on!", b"200 OK", b"text/plain").await;
        } else if request_str.starts_with("POST /off HTTP/1.1") {
            relay.set_low();
            html_response(&mut socket, b"Powered off!", b"200 OK", b"text/plain").await;
        } else if request_str.starts_with("POST /status HTTP/1.1") {
            let status = if relay.is_set_high() { b"1" } else { b"0" };
            html_response(&mut socket, status, b"200 OK", b"text/plain").await;
        } else if request_str.starts_with("GET / HTTP/1.1") {
            html_response(
                &mut socket,
                include_bytes!("../website/index.html"),
                b"200 OK",
                b"text/html",
            )
            .await;
        } else {
            html_response(
                &mut socket,
                b"POST /on, /off, /status.",
                b"404 Not Found",
                b"text/plain",
            )
            .await;
        }

        socket.flush().await.unwrap_or_else(|e| {
            warn!("flush error: {:?}", e);
        });
        socket.abort();
    }
}
