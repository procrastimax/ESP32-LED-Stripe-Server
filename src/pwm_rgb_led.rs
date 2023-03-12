use esp_idf_hal::{
    gpio::OutputPin,
    ledc::{config::TimerConfig, LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver},
};

use crate::rgb_led::RGB8;
use esp_idf_hal::{peripheral::Peripheral, prelude::*};
use esp_idf_sys::EspError;

pub struct PwmRgbLed<'a> {
    red_driver: LedcDriver<'a>,
    green_driver: LedcDriver<'a>,
    blue_driver: LedcDriver<'a>,
}

impl<'a> PwmRgbLed<'a> {
    pub fn new<T, CR, CG, CB, PR, PG, PB>(
        frequency: Hertz,
        timer: impl Peripheral<P = T> + 'a,
        channel_r: impl Peripheral<P = CR> + 'a,
        channel_g: impl Peripheral<P = CG> + 'a,
        channel_b: impl Peripheral<P = CB> + 'a,
        pin_r: impl Peripheral<P = PR> + 'a,
        pin_g: impl Peripheral<P = PG> + 'a,
        pin_b: impl Peripheral<P = PB> + 'a,
    ) -> Result<PwmRgbLed<'a>, EspError>
    where
        T: LedcTimer,
        CR: LedcChannel,
        CG: LedcChannel,
        CB: LedcChannel,
        PR: OutputPin,
        PG: OutputPin,
        PB: OutputPin,
    {
        let timer_config = TimerConfig::default().frequency(frequency);
        let timer_driver = LedcTimerDriver::new(timer, &timer_config)?;

        Ok(PwmRgbLed {
            red_driver: LedcDriver::new(channel_r, &timer_driver, pin_r)?,
            green_driver: LedcDriver::new(channel_g, &timer_driver, pin_g)?,
            blue_driver: LedcDriver::new(channel_b, &timer_driver, pin_b)?,
        })
    }

    pub fn set_color(&mut self, color: &RGB8) -> Result<(), EspError> {
        self.red_driver.set_duty(color.r.into())?;
        self.green_driver.set_duty(color.g.into())?;
        self.blue_driver.set_duty(color.b.into())?;
        Ok(())
    }

    pub fn set_off(&mut self) -> Result<(), EspError> {
        self.red_driver.set_duty(0)?;
        self.green_driver.set_duty(0)?;
        self.blue_driver.set_duty(0)?;
        Ok(())
    }
}
