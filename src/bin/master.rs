#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, usb::vbus_detect::SoftwareVbusDetect};
use negl_nrf52840::*;
use once_cell::sync::OnceCell;
use rktk::{
    drivers::{dummy, Drivers},
    singleton,
};
use rktk_drivers_common::usb::{CommonUsbDriverConfig, CommonUsbReporterBuilder, UsbDriverConfig};

#[cfg(not(feature = "trouble"))]
bind_interrupts!(pub struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<embassy_nrf::peripherals::USBD>;
    SPI2 => embassy_nrf::spim::InterruptHandler<embassy_nrf::peripherals::SPI2>;
    TWISPI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
    UARTE0 => embassy_nrf::buffered_uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
});

#[cfg(feature = "trouble")]
bind_interrupts!(pub struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<embassy_nrf::peripherals::USBD>;
    SPI2 => embassy_nrf::spim::InterruptHandler<embassy_nrf::peripherals::SPI2>;
    TWISPI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
    UARTE0 => embassy_nrf::buffered_uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
    RNG => embassy_nrf::rng::InterruptHandler<embassy_nrf::peripherals::RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

static SOFTWARE_VBUS: OnceCell<SoftwareVbusDetect> = OnceCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = negl_nrf52840::init_peri();

    // create shared SPI bus
    // NOTE: This must be done as soon as possible, otherwise the SPI device will start acting strangely.
    let spi = create_spi!(p);

    #[cfg(feature = "trouble")]
    let trouble_ble_reporter = {
        use rand_chacha::{rand_core::SeedableRng as _, ChaCha12Rng};
        use rktk_drivers_common::trouble::reporter::{
            TroubleReporterBuilder, TroubleReporterConfig,
        };
        use rktk_drivers_nrf::init_sdc;

        let mut rng = singleton!(
            embassy_nrf::rng::Rng::new(p.RNG, Irqs),
            embassy_nrf::rng::Rng<embassy_nrf::peripherals::RNG>
        );
        let rng_2 = singleton!(ChaCha12Rng::from_rng(&mut rng).unwrap(), ChaCha12Rng);
        init_sdc!(
            sdc, Irqs, rng,
            mpsl: (p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31),
            sdc: (p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24, p.PPI_CH25, p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29),
            mtu: 72,
            txq: 3,
            rxq: 3
        );
        TroubleReporterBuilder::<_, _, 1, 5, 72>::new(
            sdc.unwrap(),
            rng_2,
            TroubleReporterConfig {
                advertise_name: "negL Trouble",
                peripheral_config: None,
            },
        )
    };

    cfg_if::cfg_if! {
        if #[cfg(feature = "sd")] {
            let ble_builder = Some(init_sd().await.0);
        } else if #[cfg(feature = "trouble")] {
            let ble_builder = Some(trouble_ble_reporter);
        } else {
            let ble_builder = dummy::ble_builder();
        }
    }

    let usb = {
        let vbus = SOFTWARE_VBUS.get_or_init(|| SoftwareVbusDetect::new(true, true));
        let embassy_driver = embassy_nrf::usb::Driver::new(p.USBD, Irqs, vbus);
        let mut driver_config = UsbDriverConfig::new(0xc0de, 0xcafe);
        driver_config.product = Some("negL");
        let opts = CommonUsbDriverConfig::new(embassy_driver, driver_config);
        Some(CommonUsbReporterBuilder::new(opts))
    };

    // let storage = rktk_drivers_nrf::softdevice::flash::create_storage_driver(flash, &cache);

    let drivers = Drivers {
        keyscan: driver_keyscan!(p, spi),
        system: driver_system!(p),
        mouse: Some(driver_mouse!(p, spi)),
        usb_builder: usb,
        display: Some(driver_display!(p)),
        split: Some(driver_split!(p)),
        rgb: Some(driver_rgb!(p)),
        storage: dummy::storage(),
        ble_builder,
        debounce: Some(driver_debounce!()),
        encoder: Some(driver_encoder!(p)),
    };

    rktk::task::start(drivers, &keymap::KEYMAP, Some(HAND), hooks!(p)).await;
}
