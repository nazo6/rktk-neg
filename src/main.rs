#![no_std]
#![no_main]

use core::panic::PanicInfo;

use embassy_executor::Spawner;
use embassy_nrf::{
    gpio::{Input, Output, Pin, Pull},
    interrupt::{self, InterruptExt, Priority},
    peripherals::SPI2,
    ppi::Group,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use hooks::NegBacklightHooks;
use once_cell::sync::OnceCell;

use rktk::{
    hooks::EmptyMainHooks,
    interface::{
        debounce::EagerDebounceDriver, double_tap::DummyDoubleTapResetDriver,
        mouse::DummyMouseDriverBuilder, split::DummySplitDriver, storage::DummyStorageDriver,
    },
    task::Drivers,
};
use rktk_drivers_nrf::{
    backlight::ws2812_pwm::Ws2812Pwm, display::ssd1306::create_ssd1306,
    keyscan::shift_register_matrix::create_shift_register_matrix, mouse::pmw3360, panic_utils,
    softdevice::flash::get_flash, split::uart_half_duplex::UartHalfDuplexSplitDriver, usb::UsbOpts,
};

use defmt_rtt as _;
use nrf_softdevice as _;

mod hooks;
mod keymap;
mod misc;

#[cfg(feature = "ble")]
mod ble {
    pub use rktk_drivers_nrf::softdevice::ble::init_ble_server;
    pub use rktk_drivers_nrf::softdevice::ble::NrfBleDriverBuilder;
}

#[cfg(not(feature = "ble"))]
mod no_ble {
    pub use rktk::interface::ble::DummyBleDriverBuilder;
}

#[cfg(feature = "usb")]
mod usb {
    pub use embassy_nrf::usb::vbus_detect::SoftwareVbusDetect;
    pub use rktk_drivers_nrf::usb::{new_usb, Config as UsbConfig};
}

#[cfg(not(feature = "usb"))]
mod no_usb {
    pub use rktk::interface::usb::DummyUsbDriverBuilder;
}

use embassy_nrf::{bind_interrupts, peripherals::USBD};

bind_interrupts!(pub struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<USBD>;
    SPIM2_SPIS2_SPI2 => embassy_nrf::spim::InterruptHandler<SPI2>;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
    UARTE0_UART0 => embassy_nrf::buffered_uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
});

static SOFTWARE_VBUS: OnceCell<usb::SoftwareVbusDetect> = OnceCell::new();

pub fn map_key(row: usize, col: usize) -> Option<(usize, usize)> {
    Some((row, col))
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // - About limitation of softdevice
    // By enabling softdevice, some interrupt priority level (P0,P1,P4)
    // and peripherals are reserved by softdevice, and using them causes panic.
    //
    // Example reserved peripherals are:
    // - TIMER0
    // - CLOCK
    // - RTC0
    // ... and more
    //
    // ref:
    // List of reserved peripherals: https://docs.nordicsemi.com/bundle/sds_s140/page/SDS/s1xx/sd_resource_reqs/hw_block_interrupt_vector.html
    // Peripheral register addresses: https://docs.nordicsemi.com/bundle/ps_nrf52840/page/memory.html
    //
    // When panic occurs by peripheral conflict, PC address that caused panic is logged.
    // By investigating the address using decompiler tools like ghidra, you can find the peripheral that caused the panic.

    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    let p = embassy_nrf::init(config);

    let led_cutoff = embassy_nrf::gpio::Output::new(
        p.P0_31,
        embassy_nrf::gpio::Level::Low,
        embassy_nrf::gpio::OutputDrive::Standard,
    );

    let enc_a = embassy_nrf::gpio::Input::new(p.P0_29, embassy_nrf::gpio::Pull::Down);
    let enc_b = embassy_nrf::gpio::Input::new(p.P0_02, embassy_nrf::gpio::Pull::Down);

    interrupt::USBD.set_priority(Priority::P2);
    interrupt::SPIM2_SPIS2_SPI2.set_priority(Priority::P2);
    interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0.set_priority(Priority::P2);
    interrupt::UARTE1.set_priority(Priority::P2);

    let display = create_ssd1306(
        p.TWISPI0,
        Irqs,
        p.P1_00,
        p.P0_11,
        ssd1306::size::DisplaySize128x32,
    );

    let Some(display) = panic_utils::display_message_if_panicked(display).await else {
        cortex_m::asm::udf()
    };

    let shared_spi = Mutex::<NoopRawMutex, _>::new(embassy_nrf::spim::Spim::new(
        p.SPI2,
        Irqs,
        p.P0_17,
        p.P0_22,
        p.P0_20,
        pmw3360::recommended_pmw3360_config(),
    ));
    let ball = pmw3360::create_pmw3360(&shared_spi, p.P1_06);

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

    let split = UartHalfDuplexSplitDriver::new(
        p.P0_08.degrade(),
        p.UARTE0,
        Irqs,
        p.TIMER1,
        p.PPI_CH0,
        p.PPI_CH1,
        p.PPI_GROUP0.degrade(),
    );

    let backlight = Ws2812Pwm::new(p.PWM0, p.P0_24);

    let sd = rktk_drivers_nrf::softdevice::init_sd("negL");
    #[cfg(feature = "ble")]
    let (server, sd) = ble::init_ble_server(sd).await;
    rktk_drivers_nrf::softdevice::start_softdevice(sd).await;

    embassy_time::Timer::after_millis(50).await;

    // let rand = rktk_drivers_nrf52::softdevice::rand::SdRand::new(sd);

    let (flash, cache) = get_flash(sd);
    let storage = rktk_drivers_nrf::softdevice::flash::create_storage_driver(flash, &cache);

    let ble_builder = {
        #[cfg(feature = "ble")]
        let ble = Some(ble::NrfBleDriverBuilder::new(sd, server, "negL", flash).await);

        #[cfg(not(feature = "ble"))]
        let ble = Option::<no_ble::DummyBleDriverBuilder>::None;

        ble
    };

    let drivers = Drivers {
        keyscan,
        double_tap_reset: Option::<DummyDoubleTapResetDriver>::None,
        mouse_builder: Option::<DummyMouseDriverBuilder>::None,
        usb_builder: {
            #[cfg(feature = "usb")]
            let usb = {
                let vbus = SOFTWARE_VBUS.get_or_init(|| usb::SoftwareVbusDetect::new(true, true));
                let driver = embassy_nrf::usb::Driver::new(p.USBD, Irqs, vbus);
                let opts = UsbOpts {
                    config: {
                        let mut config = usb::UsbConfig::new(0xc0de, 0xcafe);

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
                Some(usb::new_usb(opts))
            };

            #[cfg(not(feature = "usb"))]
            let usb = Option::<no_usb::DummyUsbDriverBuilder>::None;

            usb
        },
        display_builder: Some(display),
        split: DummySplitDriver,
        backlight: Some(backlight),
        storage: Option::<DummyStorageDriver>::None,
        ble_builder,
        debounce: EagerDebounceDriver::new(embassy_time::Duration::from_millis(20)),
    };

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
