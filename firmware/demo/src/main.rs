//! This example shows how to send messages between the two cores in the RP2040 chip.

#![no_std]
#![no_main]

mod usb;

use core::future::pending;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{bind_interrupts, dma, gpio, peripherals, Peri};
use embassy_rp::adc::{Adc, Channel, Config, InterruptHandler};
use embassy_rp::gpio::Level::{High, Low};
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_rp::pio::Direction::Out;
use embassy_time::{Duration, Ticker, Timer};
use gpio::{Level, Output};
use {panic_probe as _};
use embassy_rp::peripherals::DMA_CH0;

struct DisplayPeripherals {
    noe: Peri<'static, peripherals::PIN_17>,
    shcp: Peri<'static, peripherals::PIN_18>,
    stcp: Peri<'static, peripherals::PIN_19>,

    r1_r: Peri<'static, peripherals::PIN_26>,
    r1_g: Peri<'static, peripherals::PIN_30>,
    r1_b: Peri<'static, peripherals::PIN_34>,

    c1: Peri<'static, peripherals::PIN_20>,
    c2: Peri<'static, peripherals::PIN_21>,
    c3: Peri<'static, peripherals::PIN_22>,
}

bind_interrupts!(
    /// Binds the ADC interrupts.
    struct Irqs {
        ADC_IRQ_FIFO => InterruptHandler;
        DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    }
);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());
    let mut trigger = Output::new(p.PIN_0, Level::Low);
    trigger.set_low();
    Timer::after_millis(200).await;
    trigger.set_high();
    Timer::after_millis(100).await;

    let mut led = Output::new(p.PIN_1, Level::Low);

    _spawner.spawn(usb::task(p.USB).unwrap());
    _spawner.spawn(led_loop(DisplayPeripherals {
        noe: p.PIN_17,
        shcp: p.PIN_18,
        stcp: p.PIN_19,
        r1_r: p.PIN_26,
        r1_g: p.PIN_30,
        r1_b: p.PIN_34,
        c1: p.PIN_20,
        c2: p.PIN_21,
        c3: p.PIN_22,
    }).unwrap());

    let mut adc = Adc::new(p.ADC, Irqs, Config::default());

    let mut mic = Channel::new_pin(p.PIN_47, Pull::None);
    let mut dma_ch = dma::Channel::new(p.DMA_CH0, Irqs);

    let mut light_in = Channel::new_pin(p.PIN_40, Pull::Down);

    const BLOCK_SIZE: usize = 48000;
    const NUM_CHANNELS: usize = 1;

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        let level = adc.read(&mut light_in).await.unwrap();
        info!("Light sensor: {}", level);

        /*for i in 1..10 {
            led.set_high();
            Timer::after_millis(i).await;
            led.set_low();
            Timer::after_millis(i).await;
        }*/

        // Read 100 samples from a single channel
        let mut buf = [0_u16; BLOCK_SIZE];
        let div = 999; // 48khz sample rate (48Mhz / 48khz - 1)
        adc.read_many(&mut mic, &mut buf, div, &mut dma_ch).await.unwrap();
        info!("{:?}", buf);

        ticker.next().await;
    }

}

#[embassy_executor::task]
async fn led_loop(peripherals: DisplayPeripherals) -> ! {
    let mut noe = Output::new(peripherals.noe, Level::High);
    let mut r1_r = Output::new(peripherals.r1_r, Level::Low);
    let mut r1_g = Output::new(peripherals.r1_g, Level::Low);
    let mut r1_b = Output::new(peripherals.r1_b, Level::Low);

    let mut c1 = Output::new(peripherals.c1, Level::Low);
    let mut c2 = Output::new(peripherals.c2, Level::Low);
    let mut c3 = Output::new(peripherals.c3, Level::Low);

    let mut shcp = Output::new(peripherals.shcp, Level::Low);
    let mut stcp = Output::new(peripherals.stcp, Level::High);

    // its a low side driver, so these should be inverted
    // low = LED on
    let rdata = [Low, High, High, Low];
    let gdata = [High, Low, High, Low];
    let bdata = [High, High, Low, Low];

    Timer::after_millis(10).await;

    for i in 0..4 {
        r1_r.set_level(rdata[i]);
        r1_g.set_level(gdata[i]);
        r1_b.set_level(bdata[i]);
        shcp.set_high();
        Timer::after_nanos(1000).await;
        stcp.set_high();
        Timer::after_nanos(1000).await;
        shcp.set_low();
        Timer::after_nanos(1000).await;
        stcp.set_low();
        Timer::after_nanos(1000).await;
    }
    Timer::after_millis(10).await;

    loop {
        for i in 0..4 {

            for i in 0..4 {
                r1_r.set_level(rdata[i]);
                r1_g.set_level(gdata[i]);
                r1_b.set_level(bdata[i]);
                shcp.set_high();
                Timer::after_nanos(1000).await;
                stcp.set_high();
                Timer::after_nanos(1000).await;
                shcp.set_low();
                Timer::after_nanos(1000).await;
                stcp.set_low();
                Timer::after_nanos(1000).await;
            }
            Timer::after_millis(10).await;


            noe.set_high();
            c1.set_level(if i % 2 == 0 { Low } else { High });
            c2.set_level(if (i >> 1) % 2 == 0 { Low } else { High });
            c3.set_level(if (i >> 2) % 2 == 0 { Low } else { High });
            Timer::after_millis(100).await;
            noe.set_low();
            Timer::after_millis(100).await;
        }
    }
}