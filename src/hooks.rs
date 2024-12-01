use embassy_nrf::{
    gpio::{Output, Pin},
    Peripheral,
};
use rktk::{
    drivers::interface::backlight::{BacklightCommand, BacklightDriver, BacklightMode},
    hooks::{
        channels::backlight_sender,
        empty_hooks::{EmptyCommonHooks, EmptySlaveHooks},
        interface::{backlight::RGB8, BacklightHooks, MasterHooks},
        Hooks,
    },
};

pub fn create_hooks<'d>(
    led_off_pin: impl Peripheral<P = impl Pin> + 'd,
) -> Hooks<EmptyCommonHooks, NegMasterHooks, EmptySlaveHooks, NegBacklightHooks<'d>> {
    Hooks {
        common: EmptyCommonHooks,
        master: NegMasterHooks { latest_led: None },
        slave: EmptySlaveHooks,
        backlight: NegBacklightHooks {
            led_off: embassy_nrf::gpio::Output::new(
                led_off_pin,
                embassy_nrf::gpio::Level::Low,
                embassy_nrf::gpio::OutputDrive::Standard,
            ),
        },
    }
}

pub struct NegMasterHooks {
    latest_led: Option<BacklightCommand>,
}

impl MasterHooks for NegMasterHooks {
    async fn on_state_update(
        &mut self,
        state_report: &mut rktk_keymanager::state::StateReport,
    ) -> bool {
        let led = match state_report.highest_layer {
            1 => BacklightCommand::Start(BacklightMode::SolidColor(0, 0, 10)),
            2 => BacklightCommand::Start(BacklightMode::SolidColor(10, 0, 0)),
            3 => BacklightCommand::Start(BacklightMode::SolidColor(0, 10, 0)),
            4 => BacklightCommand::Start(BacklightMode::SolidColor(10, 10, 0)),
            _ => BacklightCommand::Reset,
        };

        if let Some(latest_led) = &self.latest_led {
            if led != *latest_led {
                let backlight_sender = backlight_sender();
                let _ = backlight_sender.try_send(led.clone());
                // let _ = m2s_tx.try_send(MasterToSlave::Backlight(led.clone()));
            }
        }

        self.latest_led = Some(led);

        true
    }
}

pub struct NegBacklightHooks<'d> {
    pub led_off: Output<'d>,
}

impl BacklightHooks for NegBacklightHooks<'_> {
    async fn on_backlight_init(&mut self, _driver: &mut impl BacklightDriver) {
        self.led_off.set_high();
    }
    async fn on_backlight_process<const N: usize>(
        &mut self,
        _driver: &mut impl BacklightDriver,
        command: &BacklightCommand,
        _rgb_data: &mut Option<[RGB8; N]>,
    ) {
        if *command == BacklightCommand::Reset {
            self.led_off.set_high();
        } else {
            self.led_off.set_low();
        }
    }
}
