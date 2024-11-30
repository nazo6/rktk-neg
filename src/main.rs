#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use core::panic::PanicInfo;
use core::ptr::addr_of_mut;

use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, OutputDrive, Pull},
    interrupt::{self, InterruptExt, Priority},
    peripherals::{SPI2, USBD},
    usb::vbus_detect::SoftwareVbusDetect,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embedded_alloc::LlffHeap as Heap;
use misc::PAW3395_CONFIG;
use once_cell::sync::OnceCell;
use rktk::{
    drivers::Drivers, hooks::EmptyMainHooks, interface::debounce::EagerDebounceDriver, none_driver,
};
use rktk_drivers_common::encoder::GeneralEncoder;
use rktk_drivers_nrf::{
    backlight::ws2812_pwm::Ws2812Pwm,
    display::ssd1306::create_ssd1306,
    keyscan::shift_register_matrix::create_shift_register_matrix,
    mouse::paw3395,
    panic_utils,
    softdevice::{
        ble::{init_ble_server, NrfBleDriverBuilder},
        flash::get_flash,
    },
    usb::{new_usb, Config as UsbConfig, UsbOpts},
};

use hooks::NegBacklightHooks;

use nrf_softdevice as _;

mod hooks;
mod keymap;
mod misc;

extern crate alloc;

#[global_allocator]
static HEAP: Heap = Heap::empty();

bind_interrupts!(pub struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<USBD>;
    SPIM2_SPIS2_SPI2 => embassy_nrf::spim::InterruptHandler<SPI2>;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
    UARTE0_UART0 => embassy_nrf::buffered_uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
});

static SOFTWARE_VBUS: OnceCell<SoftwareVbusDetect> = OnceCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    let p = {
        let config = {
            let mut config = embassy_nrf::config::Config::default();
            config.gpiote_interrupt_priority = Priority::P2;
            config.time_interrupt_priority = Priority::P2;
            config.lfclk_source = embassy_nrf::config::LfclkSource::ExternalXtal;
            config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
            config
        };
        embassy_nrf::init(config)
    };

    let led_cutoff = embassy_nrf::gpio::Output::new(
        p.P0_31,
        embassy_nrf::gpio::Level::Low,
        embassy_nrf::gpio::OutputDrive::Standard,
    );

    interrupt::USBD.set_priority(Priority::P2);
    interrupt::SPIM2_SPIS2_SPI2.set_priority(Priority::P2);
    interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0.set_priority(Priority::P2);
    interrupt::UARTE1.set_priority(Priority::P2);

    let Some(display) = panic_utils::display_message_if_panicked(create_ssd1306(
        p.TWISPI0,
        Irqs,
        p.P1_00,
        p.P0_11,
        ssd1306::size::DisplaySize128x32,
    ))
    .await
    else {
        cortex_m::asm::udf()
    };

    let shared_spi = {
        let mut spi_config = paw3395::recommended_paw3395_config();
        spi_config.sck_drive = OutputDrive::Standard;
        spi_config.mosi_drive = OutputDrive::Standard;
        spi_config.frequency = embassy_nrf::spim::Frequency::K250;
        Mutex::<CriticalSectionRawMutex, _>::new(embassy_nrf::spim::Spim::new(
            p.SPI2, Irqs, p.P0_17, p.P0_22, p.P0_20, spi_config,
        ))
    };

    let ball = paw3395::create_paw3395(&shared_spi, p.P1_06, PAW3395_CONFIG);

    let keyscan = create_shift_register_matrix::<'_, '_, _, _, _, 8, 5, 8, 5>(
        &shared_spi,
        p.P1_04,
        [
            Input::new(p.P1_15, Pull::Down), // ROW0
            Input::new(p.P1_13, Pull::Down), // ROW1
            Input::new(p.P1_11, Pull::Down), // ROW2
            Input::new(p.P0_10, Pull::Down), // ROW3
            Input::new(p.P0_09, Pull::Down), // ROW4
        ],
        (2, 6),
        misc::translate_key_position,
    );

    let encoder = GeneralEncoder::new([(
        Input::new(p.P0_02, Pull::Down),
        Input::new(p.P0_29, Pull::Down),
    )]);

    let backlight = Ws2812Pwm::new(p.PWM0, p.P0_24);
    let usb = {
        let vbus = SOFTWARE_VBUS.get_or_init(|| SoftwareVbusDetect::new(true, true));
        let driver = embassy_nrf::usb::Driver::new(p.USBD, Irqs, vbus);
        let opts = UsbOpts {
            config: {
                let mut config = UsbConfig::new(0xc0de, 0xcafe);

                config.manufacturer = Some("nazo6");
                config.product = Some("negL");
                config.serial_number = Some("12345678");
                config.max_power = 100;
                config.max_packet_size_0 = 64;
                config.supports_remote_wakeup = true;

                config
            },
            mouse_poll_interval: 2,
            kb_poll_interval: 5,
            driver,
        };
        Some(new_usb(opts))
    };

    let sd = rktk_drivers_nrf::softdevice::init_sd("negL");

    let server = init_ble_server(
        sd,
        rktk_drivers_nrf::softdevice::ble::DeviceInformation {
            manufacturer_name: Some("nazo6"),
            model_number: Some("100"),
            serial_number: Some("100"),
            ..Default::default()
        },
    )
    .await;

    rktk_drivers_nrf::softdevice::start_softdevice(sd).await;
    embassy_time::Timer::after_millis(50).await;
    // let rand = rktk_drivers_nrf52::softdevice::rand::SdRand::new(sd);
    let (flash, cache) = get_flash(sd);
    let storage = rktk_drivers_nrf::softdevice::flash::create_storage_driver(flash, &cache);

    let ble_builder = Some(NrfBleDriverBuilder::new(sd, server, "negL", flash).await);

    let drivers = Drivers {
        keyscan,
        double_tap_reset: none_driver!(DoubleTapReset),
        mouse_builder: Some(ball),
        usb_builder: usb,
        display_builder: Some(display),
        split: none_driver!(Split),
        backlight: Some(backlight),
        storage: Some(storage),
        ble_builder,
        debounce: Some(EagerDebounceDriver::new(
            embassy_time::Duration::from_millis(20),
        )),
        encoder: Some(encoder),
    };

    // let vcc_cutoff = embassy_nrf::gpio::Output::new(
    //     p.P0_13,
    //     embassy_nrf::gpio::Level::Low,
    //     embassy_nrf::gpio::OutputDrive::Standard,
    // );

    rktk::task::start(
        drivers,
        keymap::KEY_CONFIG,
        rktk::hooks::Hooks {
            main: EmptyMainHooks,
            backlight: NegBacklightHooks(led_cutoff),
        },
    )
    .await;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    panic_utils::save_panic_info(info);
    cortex_m::peripheral::SCB::sys_reset()
}
