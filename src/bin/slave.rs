#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_nrf::bind_interrupts;
use negl_nrf52840::*;
use rktk::drivers::{dummy, Drivers};

bind_interrupts!(pub struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<embassy_nrf::peripherals::USBD>;
    SPI2 => embassy_nrf::spim::InterruptHandler<embassy_nrf::peripherals::SPI2>;
    TWISPI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
    UARTE0 => embassy_nrf::buffered_uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = negl_nrf52840::init_peri();

    let _ = negl_nrf52840::init_sd().await;

    let spi = create_spi!(p);

    let drivers = Drivers {
        keyscan: driver_keyscan!(p, spi),
        system: driver_system!(p),
        mouse: Some(driver_mouse!(p, spi)),
        usb_builder: dummy::usb_builder(),
        display: Some(driver_display!(p)),
        split: Some(driver_split!(p)),
        rgb: Some(driver_rgb!(p)),
        storage: dummy::storage(),
        ble_builder: dummy::ble_builder(),
        debounce: Some(driver_debounce!()),
        encoder: Some(driver_encoder!(p)),
    };

    rktk::task::start(drivers, &keymap::KEYMAP, Some(misc::HAND), hooks!(p)).await;
}
