#![no_std]

use core::panic::PanicInfo;

use embassy_nrf::Peripherals;
use rktk::config::Hand;
use rktk_drivers_common::panic_utils;

pub mod drivers;
pub mod hooks;
pub mod keymap;
pub mod misc;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use embedded_alloc::LlffHeap as Heap;

#[cfg(feature = "alloc")]
#[global_allocator]
static HEAP: Heap = Heap::empty();

#[cfg(feature = "sd")]
use nrf_softdevice as _;
#[cfg(feature = "sd")]
use rktk_drivers_nrf::softdevice::{ble::init_ble_server, flash::get_flash, init_softdevice};

pub fn init_peri() -> Peripherals {
    let p = {
        let config = {
            let mut config = embassy_nrf::config::Config::default();
            #[cfg(feature = "sd")]
            {
                use embassy_nrf::interrupt::Priority;
                config.gpiote_interrupt_priority = Priority::P2;
                config.time_interrupt_priority = Priority::P2;
            }
            config.lfclk_source = embassy_nrf::config::LfclkSource::ExternalXtal;
            config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;

            config
        };
        embassy_nrf::init(config)
    };

    #[cfg(feature = "sd")]
    {
        use embassy_nrf::interrupt::{self, *};
        interrupt::USBD.set_priority(Priority::P2);
        interrupt::SPI2.set_priority(Priority::P2);
        interrupt::SPIM3.set_priority(Priority::P2);
        interrupt::UARTE0.set_priority(Priority::P2);
    }

    #[cfg(feature = "alloc")]
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 32768;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    p
}

#[cfg(feature = "sd")]
use rktk_drivers_nrf::softdevice::ble::SoftdeviceBleReporterBuilder;
#[cfg(feature = "sd")]
use rktk_drivers_nrf::softdevice::flash::SharedFlash;

#[cfg(feature = "sd")]
pub async fn init_sd() -> (SoftdeviceBleReporterBuilder, &'static SharedFlash) {
    let sd = init_softdevice("negL");

    let server = init_ble_server(
        sd,
        rktk_drivers_nrf::softdevice::ble::DeviceInformation {
            manufacturer_name: Some("nazo6"),
            model_number: Some("100"),
            serial_number: Some("100"),
            ..Default::default()
        },
    );
    let (flash, _cache) = get_flash(sd);

    rktk_drivers_nrf::softdevice::start_softdevice(sd).await;
    embassy_time::Timer::after_millis(200).await;

    (
        SoftdeviceBleReporterBuilder::new(sd, server, "negL", flash),
        flash,
    )
}

pub const HAND: Hand = {
    #[cfg(feature = "left")]
    {
        Hand::Left
    }
    #[cfg(feature = "right")]
    {
        Hand::Right
    }
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    panic_utils::save_panic_info(info);
    cortex_m::peripheral::SCB::sys_reset()
}
