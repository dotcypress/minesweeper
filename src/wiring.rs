use hal::analog::adc::Adc;
use hal::gpio::{gpioa::*, gpioc::*, *};
use hal::i2c::*;
use hal::stm32;
use hal::timer::*;
use klaptik::*;
use ssd1306::{mode::BasicMode, prelude::*, *};

pub type VibroMotor = PC15<Output<OpenDrain>>;
pub type Thumb = (Adc, PA1<Analog>, PA0<Analog>);
pub type RngTimer = Timer<stm32::TIM2>;
pub type VibroTimer = Timer<stm32::TIM16>;
pub type InputTimer = Timer<stm32::TIM17>;
pub type RenderTimer = Timer<stm32::TIM14>;
pub type DisplayI2c = I2c<stm32::I2C2, PA12<Output<OpenDrain>>, PA11<Output<OpenDrain>>>;
pub type DisplayController = Ssd1306<I2CInterface<DisplayI2c>, DisplaySize128x64, BasicMode>;

pub struct Ssd1306Canvas(pub DisplayController);

impl Canvas for Ssd1306Canvas {
    fn draw(&mut self, bounds: Rect, buffer: &[u8]) {
        let origin = bounds.origin();
        let size = bounds.size();
        let start = (origin.x() as u8, origin.y() as u8);
        let end = (
            (origin.x() + size.width()) as u8,
            (origin.y() + size.height()) as u8,
        );
        let controller = &mut self.0;
        controller.set_draw_area(start, end).expect("draw failed");
        controller.draw(buffer).expect("draw failed");
    }
}
