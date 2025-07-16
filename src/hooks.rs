use embassy_nrf::{
    gpio::{Output, Pin},
    Peri,
};
use rktk::{
    drivers::interface::{
        reporter::ReporterDriver,
        rgb::{RgbCommand, RgbDriver, RgbMode, RgbPattern},
    },
    hooks::{
        channels::rgb::rgb_sender,
        empty_hooks::{EmptyCommonHooks, EmptySlaveHooks},
        interface::{master::Report, MasterHooks, RgbHooks},
        Hooks,
    },
};

pub fn create_hooks(
    led_off_pin: Peri<'static, impl Pin>,
) -> Hooks<EmptyCommonHooks, NegMasterHooks, EmptySlaveHooks, NegRgbHooks> {
    Hooks {
        common: EmptyCommonHooks,
        master: NegMasterHooks {
            latest_led: None,
            latest_highest_layer: 100,
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
    latest_highest_layer: u8,
}

impl MasterHooks for NegMasterHooks {
    async fn on_state_update(
        &mut self,
        state_report: &mut Report,
        usb_reporter: &Option<impl ReporterDriver>,
        _ble_reporter: &Option<impl ReporterDriver>,
    ) -> bool {
        if let Some(usb) = usb_reporter {
            if self.latest_highest_layer != state_report.highest_layer {
                let _ = usb
                    .send_raw_hid_data(&[0x01, state_report.highest_layer])
                    .await;
                self.latest_highest_layer = state_report.highest_layer;
            }
        }

        let led = match state_report.highest_layer {
            1 => RgbCommand::Start(RgbMode::SolidColor(0, 0, 10)),
            2 => RgbCommand::Start(RgbMode::SolidColor(0, 10, 0)),
            3 => RgbCommand::Start(RgbMode::Pattern(RgbPattern::Rainbow(0.3 / 1e3, 1.0))),
            4 => RgbCommand::Start(RgbMode::SolidColor(10, 10, 0)),
            _ => RgbCommand::Start(RgbMode::Off),
        };

        if let Some(latest_led) = &self.latest_led {
            if led != *latest_led {
                let rgb_sender = rgb_sender();
                let _ = rgb_sender.try_send(led.clone());
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
    async fn on_rgb_init(&mut self, _driver: &mut impl RgbDriver, _is_master: bool) {
        self.led_off.set_low();
    }
    async fn on_rgb_process(&mut self, _driver: &mut impl RgbDriver, rgb_mode: &mut RgbMode) {
        if *rgb_mode == RgbMode::Off {
            self.led_off.set_high();
        } else {
            self.led_off.set_low();
        }
    }
}
