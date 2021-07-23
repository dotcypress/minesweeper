#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

mod board;
mod game;
mod sprites;

use hal::analog::adc::Adc;
use hal::exti::Event;
use hal::gpio::{gpioa::*, *};
use hal::i2c::*;
use hal::prelude::*;
use hal::stm32;
use hal::timer::*;
use klaptik::*;
use ssd1306::{mode::BasicMode, prelude::*, *};

use crate::game::*;

const BOMBS: usize = 8;

pub type Thumb = (Adc, PA1<Analog>, PA0<Analog>);
pub type RngTimer = Timer<stm32::TIM2>;
pub type InputTimer = Timer<stm32::TIM17>;
pub type RenderTimer = Timer<stm32::TIM14>;
pub type DisplayI2c = I2c<stm32::I2C2, PA12<Output<OpenDrain>>, PA11<Output<OpenDrain>>>;
pub type DisplayController = Ssd1306<I2CInterface<DisplayI2c>, DisplaySize128x64, BasicMode>;

#[rtic::app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        ui: GameUI,
        game: Minesweeper,
        canvas: Ssd1306Canvas,
        thumb: Thumb,
        render_timer: RenderTimer,
        input_timer: InputTimer,
        rng_timer: RngTimer,
        exti: stm32::EXTI,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.freeze(hal::rcc::Config::pll());
        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let mut rng_timer = ctx.device.TIM2.timer(&mut rcc);
        rng_timer.resume();

        let mut render_timer = ctx.device.TIM14.timer(&mut rcc);
        render_timer.start(32.hz());
        render_timer.listen();

        let mut input_timer = ctx.device.TIM17.timer(&mut rcc);
        input_timer.start(8.hz());
        input_timer.listen();

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpiob = ctx.device.GPIOB.split(&mut rcc);

        let mut exti = ctx.device.EXTI;
        gpiob.pb4.listen(SignalEdge::Falling, &mut exti);
        gpiob.pb7.listen(SignalEdge::Falling, &mut exti);

        let adc = ctx.device.ADC.constrain(&mut rcc);
        let x_pin = gpioa.pa1.into_analog();
        let y_pin = gpioa.pa0.into_analog();
        let thumb = (adc, x_pin, y_pin);

        let sda = gpioa.pa12.into_open_drain_output();
        let scl = gpioa.pa11.into_open_drain_output();
        let i2c_config = Config::with_timing(0x0010061A);
        let i2c_bus = ctx.device.I2C2.i2c(sda, scl, i2c_config, &mut rcc);

        delay.delay(10.ms());
        let interface = I2CDisplayInterface::new(i2c_bus);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0);
        display.init().expect("failed to init display");
        let canvas = Ssd1306Canvas(display);

        let game = Minesweeper::new(BOMBS);
        let mut ui = GameUI::new();
        ui.set_state(&game);

        init::LateResources {
            exti,
            ui,
            game,
            canvas,
            thumb,
            rng_timer,
            input_timer,
            render_timer,
        }
    }

    #[task(binds = TIM14, resources = [canvas, game, render_timer, ui])]
    fn render_timer_tick(ctx: render_timer_tick::Context) {
        let render_timer = ctx.resources.render_timer;
        let canvas = ctx.resources.canvas;
        let game = ctx.resources.game;
        let ui = ctx.resources.ui;

        ui.set_state(&game);
        ui.render(canvas);

        render_timer.clear_irq();
    }

    #[task(binds = TIM17, resources = [thumb, game, input_timer])]
    fn input_timer_tick(ctx: input_timer_tick::Context) {
        let game = ctx.resources.game;
        let input_timer = ctx.resources.input_timer;
        let (adc, x_pin, y_pin) = ctx.resources.thumb;

        let x: u32 = adc.read(x_pin).unwrap();
        if x > 3000 {
            game.button_click(GameButton::DPad(Dir::Right));
        } else if x < 1000 {
            game.button_click(GameButton::DPad(Dir::Left));
        }

        let y: u32 = adc.read(y_pin).unwrap();
        if y < 1000 {
            game.button_click(GameButton::DPad(Dir::Down));
        } else if y > 3000 {
            game.button_click(GameButton::DPad(Dir::Up));
        }

        input_timer.clear_irq();
    }

    #[task(binds = EXTI4_15, resources = [exti, game, rng_timer])]
    fn button_press(ctx: button_press::Context) {
        let rng_timer = ctx.resources.rng_timer;
        let exti = ctx.resources.exti;

        let game = ctx.resources.game;
        game.seed_random(rng_timer.get_current());

        if exti.is_pending(Event::GPIO7, SignalEdge::Falling) {
            game.button_click(GameButton::A);
            exti.unpend(Event::GPIO7);
        }

        if exti.is_pending(Event::GPIO4, SignalEdge::Falling) {
            game.button_click(GameButton::B);
            exti.unpend(Event::GPIO4);
        }
    }
};

pub struct Ssd1306Canvas(DisplayController);

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
