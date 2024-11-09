use embassy_nrf::gpio::Output;
use rktk::{
    hooks::{BacklightHooks, RGB8},
    interface::backlight::{BacklightCommand, BacklightDriver},
};

pub struct NegBacklightHooks<'d>(pub Output<'d>);

impl BacklightHooks for NegBacklightHooks<'_> {
    async fn on_backlight_init(&mut self, _driver: &mut impl BacklightDriver) {
        self.0.set_high();
    }
    async fn on_backlight_process<const N: usize>(
        &mut self,
        _driver: &mut impl BacklightDriver,
        command: &BacklightCommand,
        _rgb_data: &mut Option<[RGB8; N]>,
    ) {
        if *command == BacklightCommand::Reset {
            self.0.set_high();
        } else {
            self.0.set_low();
        }
    }
}
