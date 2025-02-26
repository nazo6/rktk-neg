use embassy_nrf::{
    gpio::{Output, Pin},
    Peripheral,
};
use rktk::{
    drivers::interface::{
        dongle::DongleData,
        rgb::{RgbCommand, RgbDriver, RgbMode},
    },
    hooks::{
        channels::rgb_sender,
        empty_hooks::{EmptyCommonHooks, EmptySlaveHooks},
        interface::{rgb::RGB8, MasterHooks, RgbHooks},
        Hooks,
    },
};

pub fn create_hooks(
    led_off_pin: impl Peripheral<P = impl Pin> + 'static,
    ptx: embassy_nrf_esb::ptx::PtxRadio<'static, embassy_nrf::peripherals::RADIO, 64>,
) -> Hooks<EmptyCommonHooks, NegMasterHooks, EmptySlaveHooks, NegRgbHooks> {
    Hooks {
        common: EmptyCommonHooks,
        master: NegMasterHooks {
            latest_led: None,
            ptx,
        },
        slave: EmptySlaveHooks,
        rgb: NegRgbHooks {
            led_off: embassy_nrf::gpio::Output::new(
                led_off_pin,
                embassy_nrf::gpio::Level::Low,
                embassy_nrf::gpio::OutputDrive::Standard,
            ),
        },
    }
}

pub struct NegMasterHooks {
    latest_led: Option<RgbCommand>,
    ptx: embassy_nrf_esb::ptx::PtxRadio<'static, embassy_nrf::peripherals::RADIO, 64>,
}

impl MasterHooks for NegMasterHooks {
    async fn on_state_update(
        &mut self,
        state_report: &mut rktk_keymanager::interface::report::StateReport,
        _usb_reporter: &Option<impl rktk::drivers::interface::reporter::ReporterDriver>,
        _ble_reporter: &Option<impl rktk::drivers::interface::reporter::ReporterDriver>,
    ) -> bool {
        let led = match state_report.highest_layer {
            1 => RgbCommand::Start(RgbMode::SolidColor(0, 0, 10)),
            2 => RgbCommand::Start(RgbMode::SolidColor(10, 0, 0)),
            3 => RgbCommand::Start(RgbMode::SolidColor(0, 10, 0)),
            4 => RgbCommand::Start(RgbMode::SolidColor(10, 10, 0)),
            _ => RgbCommand::Reset,
        };
        if let Some(k) = state_report.keyboard_report {
            let mut buf = [0; 64];
            if let Ok(s) = postcard::to_slice(&DongleData::Keyboard(k.into()), &mut buf) {
                self.ptx.send(0, s, false).await;
            }
        }
        if let Some(m) = state_report.mouse_report {
            let mut buf = [0; 64];
            if let Ok(s) = postcard::to_slice(&DongleData::Mouse(m.into()), &mut buf) {
                self.ptx.send(0, s, false).await;
            }
        }

        if let Some(latest_led) = &self.latest_led {
            if led != *latest_led {
                let rgb_sender = rgb_sender();
                let _ = rgb_sender.try_send(led.clone());
                // let _ = m2s_tx.try_send(MasterToSlave::Rgb(led.clone()));
            }
        }

        self.latest_led = Some(led);

        true
    }
}

pub struct NegRgbHooks {
    pub led_off: Output<'static>,
}

impl RgbHooks for NegRgbHooks {
    async fn on_rgb_init(&mut self, _driver: &mut impl RgbDriver) {
        self.led_off.set_high();
    }
    async fn on_rgb_process<const N: usize>(
        &mut self,
        _driver: &mut impl RgbDriver,
        command: &RgbCommand,
        _rgb_data: &mut Option<[RGB8; N]>,
    ) {
        if *command == RgbCommand::Reset {
            self.led_off.set_high();
        } else {
            self.led_off.set_low();
        }
    }
}
