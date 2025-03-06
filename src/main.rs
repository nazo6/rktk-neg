#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use embedded_alloc::LlffHeap as Heap;

#[cfg(feature = "alloc")]
#[global_allocator]
static HEAP: Heap = Heap::empty();

use core::panic::PanicInfo;

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    buffered_uarte::BufferedUarte,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    interrupt::{self, InterruptExt, Priority},
    peripherals::{SPI2, USBD},
    twim::Twim,
    usb::vbus_detect::SoftwareVbusDetect,
    Peripherals,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use once_cell::sync::OnceCell;
use rktk::{
    drivers::{interface::keyscan::Hand, Drivers},
    none_driver, singleton,
};
use rktk_drivers_common::{
    debounce::EagerDebounceDriver,
    display::ssd1306::Ssd1306DisplayBuilder,
    encoder::GeneralEncoder,
    keyscan::{shift_register_matrix::ShiftRegisterMatrix, HandDetector},
    mouse::paw3395::Paw3395Builder,
    panic_utils,
    usb::{CommonUsbDriverBuilder, UsbDriverConfig, UsbOpts},
};
use rktk_drivers_nrf::{
    esb::{
        create_address,
        reporter::{EsbInterruptHandler, EsbReporterDriverBuilder, TimerInterruptHandler},
        Config as EsbConfig,
    },
    mouse::paw3395,
    rgb::ws2812_pwm::Ws2812Pwm,
    split::uart_full_duplex::UartFullDuplexSplitDriver,
    system::NrfSystemDriver,
};

mod hooks;
mod keymap;
mod misc;

bind_interrupts!(pub struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<USBD>;
    SPI2 => embassy_nrf::spim::InterruptHandler<SPI2>;
    TWISPI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
    UARTE0 => embassy_nrf::buffered_uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
    RADIO => EsbInterruptHandler;
    TIMER0 => TimerInterruptHandler;
});

static SOFTWARE_VBUS: OnceCell<SoftwareVbusDetect> = OnceCell::new();

fn init() -> Peripherals {
    let p = {
        let config = {
            let mut config = embassy_nrf::config::Config::default();
            config.lfclk_source = embassy_nrf::config::LfclkSource::ExternalXtal;
            config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
            config
        };
        embassy_nrf::init(config)
    };

    interrupt::USBD.set_priority(Priority::P2);
    interrupt::SPI2.set_priority(Priority::P2);
    interrupt::SPIM3.set_priority(Priority::P2);
    interrupt::UARTE0.set_priority(Priority::P2);
    interrupt::RADIO.set_priority(Priority::P1);

    #[cfg(feature = "alloc")]
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 16384;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    p
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = init();

    // create shared SPI bus
    // NOTE: This must be done as soon as possible, otherwise the SPI device will start acting strangely.
    let shared_spi = {
        let mut spi_config = paw3395::recommended_spi_config();
        spi_config.sck_drive = OutputDrive::Standard;
        spi_config.mosi_drive = OutputDrive::Standard;
        spi_config.frequency = embassy_nrf::spim::Frequency::K250;

        Mutex::<ThreadModeRawMutex, _>::new(embassy_nrf::spim::Spim::new(
            p.SPI2, Irqs, p.P0_17, p.P0_22, p.P0_20, spi_config,
        ))
    };

    let d = EsbReporterDriverBuilder::new(
        p.TIMER0,
        p.RADIO,
        Irqs,
        EsbConfig {
            addresses: create_address(90).unwrap(),
            ..EsbConfig::default()
        },
    );
    let ble_builder = Some(d);

    embassy_time::Timer::after_millis(200).await;

    let split = {
        let uarte_config = embassy_nrf::uarte::Config::default();
        UartFullDuplexSplitDriver::new(BufferedUarte::new(
            p.UARTE0,
            p.TIMER1,
            p.PPI_CH0,
            p.PPI_CH1,
            p.PPI_GROUP0,
            Irqs,
            p.P0_08,
            p.P0_06,
            uarte_config,
            singleton!([0; 256], [u8; 256]),
            singleton!([0; 256], [u8; 256]),
        ))
    };

    let drivers = {
        let display = Ssd1306DisplayBuilder::new(
            Twim::new(
                p.TWISPI0,
                Irqs,
                p.P1_00,
                p.P0_11,
                rktk_drivers_nrf::display::ssd1306::recommended_i2c_config(),
            ),
            ssd1306::size::DisplaySize128x32,
        );
        let Some(display) = panic_utils::display_message_if_panicked(display).await else {
            cortex_m::asm::udf()
        };

        let ball_cs = Output::new(
            p.P1_06,
            embassy_nrf::gpio::Level::High,
            OutputDrive::Standard,
        );
        let ball_spi_device = SpiDevice::new(&shared_spi, ball_cs);
        let ball = Paw3395Builder::new(ball_spi_device, misc::PAW3395_CONFIG);

        let shift_register_cs = Output::new(
            p.P1_04,
            embassy_nrf::gpio::Level::High,
            OutputDrive::Standard,
        );
        let shift_register_spi_device = SpiDevice::new(&shared_spi, shift_register_cs);
        let keyscan = ShiftRegisterMatrix::<_, _, 8, 5, 8, 5>::new(
            shift_register_spi_device,
            [
                Input::new(p.P1_15, Pull::Down), // ROW0
                Input::new(p.P1_13, Pull::Down), // ROW1
                Input::new(p.P1_11, Pull::Down), // ROW2
                Input::new(p.P0_10, Pull::Down), // ROW3
                Input::new(p.P0_09, Pull::Down), // ROW4
            ],
            HandDetector::Constant({
                #[cfg(feature = "left")]
                {
                    Hand::Left
                }
                #[cfg(feature = "right")]
                {
                    Hand::Right
                }
            }),
            misc::translate_key_position,
            None,
        );

        let encoder = GeneralEncoder::new([(
            Input::new(p.P0_02, Pull::Down),
            Input::new(p.P0_29, Pull::Down),
        )]);

        let rgb = Ws2812Pwm::new(p.PWM0, p.P0_24);
        let usb = {
            let vbus = SOFTWARE_VBUS.get_or_init(|| SoftwareVbusDetect::new(true, true));
            let driver = embassy_nrf::usb::Driver::new(p.USBD, Irqs, vbus);
            let opts = UsbOpts {
                config: {
                    let mut config = UsbDriverConfig::new(0xc0de, 0xcafe);

                    config.manufacturer = Some("nazo6");
                    config.product = Some("negL");
                    config.serial_number = Some("12345678");
                    config.max_power = 100;
                    config.max_packet_size_0 = 64;
                    config.supports_remote_wakeup = true;

                    config
                },
                mouse_poll_interval: 1,
                kb_poll_interval: 5,
                driver,
                defmt_usb_use_dtr: true,
            };
            Some(CommonUsbDriverBuilder::new(opts))
        };

        #[cfg(feature = "force-slave")]
        let usb = none_driver!(UsbBuilder);
        #[cfg(feature = "force-slave")]
        let ble_builder = none_driver!(BleBuilder);

        // let storage = rktk_drivers_nrf::softdevice::flash::create_storage_driver(flash, &cache);

        let vcc_cutoff = (
            Output::new(p.P0_13, Level::High, OutputDrive::Standard),
            Level::Low,
        );

        Drivers {
            keyscan,
            system: NrfSystemDriver::new(Some(vcc_cutoff)),
            mouse_builder: Some(ball),
            usb_builder: usb,
            display_builder: Some(display),
            split: Some(split),
            rgb: Some(rgb),
            storage: none_driver!(Storage),
            ble_builder,
            debounce: Some(EagerDebounceDriver::new(
                embassy_time::Duration::from_millis(10),
                true,
            )),
            encoder: Some(encoder),
        }
    };

    let hooks = hooks::create_hooks(p.P0_31);

    rktk::task::start(drivers, keymap::KEYMAP, hooks).await;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    defmt::info!("panic");
    cortex_m::interrupt::disable();
    panic_utils::save_panic_info(info);
    cortex_m::peripheral::SCB::sys_reset()
}
