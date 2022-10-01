use hal::gpio::{gpioa::*, *};
use hal::i2c::I2c;
use hal::spi::*;
use hal::stm32;
use hal::timer::*;
use klaptik::drivers::st7567::ST7567;
use wii_ext::nunchuk::Nunchuk;

pub type RngTimer = Timer<stm32::TIM3>;
pub type InputTimer = Timer<stm32::TIM17>;
pub type RenderTimer = Timer<stm32::TIM14>;
pub type DisplayController = ST7567<
    Spi<hal::pac::SPI2, (PA0<Analog>, NoMiso, PA4<Analog>)>,
    PA5<Output<PushPull>>,
    PA7<Output<PushPull>>,
    PA3<Output<PushPull>>,
>;
pub type Joystick = Nunchuk<
    I2c<hal::pac::I2C2, PA12<hal::gpio::Output<OpenDrain>>, PA11<hal::gpio::Output<OpenDrain>>>,
>;
