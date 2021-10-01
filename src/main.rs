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
mod wiring;

use hal::exti::Event;
use hal::gpio::*;
use hal::i2c::*;
use hal::prelude::*;
use hal::stm32;
use klaptik::*;
use ssd1306::{prelude::*, *};

use crate::game::*;
use crate::wiring::*;

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    const BOMBS: usize = 8;

    #[shared]
    struct Shared {
        #[lock_free]
        game: Minesweeper,
        #[lock_free]
        vibro: VibroMotor,
        #[lock_free]
        render_timer: RenderTimer,
        #[lock_free]
        input_timer: InputTimer,
        #[lock_free]
        vibro_timer: VibroTimer,
    }

    #[local]
    struct Local {
        exti: stm32::EXTI,
        canvas: Ssd1306Canvas,
        ui: GameUI,
        rng_timer: RngTimer,
        thumb: Thumb,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.constrain();

        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let mut rng_timer = ctx.device.TIM2.timer(&mut rcc);
        rng_timer.resume();

        let mut render_timer = ctx.device.TIM14.timer(&mut rcc);
        render_timer.start(24.hz());
        render_timer.listen();

        let mut vibro_timer = ctx.device.TIM16.timer(&mut rcc);
        vibro_timer.start(120.ms());
        vibro_timer.listen();

        let mut input_timer = ctx.device.TIM17.timer(&mut rcc);
        input_timer.start(8.hz());
        input_timer.listen();

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let mut exti = ctx.device.EXTI;
        gpiob.pb4.listen(SignalEdge::Rising, &mut exti);
        gpiob.pb7.listen(SignalEdge::Rising, &mut exti);

        let mut vibro = gpioc.pc15.into_open_drain_output();
        vibro.set_high().unwrap();

        let adc = ctx.device.ADC.constrain(&mut rcc);
        let x_pin = gpioa.pa1.into_analog();
        let y_pin = gpioa.pa0.into_analog();
        let thumb = (adc, x_pin, y_pin);

        let sda = gpioa.pa12.into_open_drain_output();
        let scl = gpioa.pa11.into_open_drain_output();
        let i2c_config = Config::with_timing(0x0010061A);
        let i2c_bus = ctx.device.I2C2.i2c(sda, scl, i2c_config, &mut rcc);

        let interface = I2CDisplayInterface::new(i2c_bus);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0);
        delay.delay(10.ms());
        display.init().expect("failed to init display");
        let canvas = Ssd1306Canvas(display);

        let game = Minesweeper::new(BOMBS);
        let mut ui = GameUI::new();
        ui.set_state(&game);

        (
            Shared {
                game,
                input_timer,
                render_timer,
                vibro,
                vibro_timer,
            },
            Local {
                canvas,
                ui,
                exti,
                thumb,
                rng_timer,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM14, local = [canvas, ui], shared = [game, render_timer])]
    fn render_timer_tick(ctx: render_timer_tick::Context) {
        let render_timer_tick::LocalResources { canvas, ui } = ctx.local;
        let render_timer_tick::SharedResources { game, render_timer } = ctx.shared;

        ui.set_state(&game);
        ui.render(canvas);

        render_timer.clear_irq();
    }

    #[task(binds = TIM16, shared = [vibro_timer, vibro])]
    fn vibro_timer_tick(ctx: vibro_timer_tick::Context) {
        ctx.shared.vibro.set_high().unwrap();
        ctx.shared.vibro_timer.clear_irq();
    }

    #[task(binds = TIM17, local = [thumb], shared = [game, input_timer])]
    fn input_timer_tick(ctx: input_timer_tick::Context) {
        let input_timer_tick::LocalResources {
            thumb: (adc, x_pin, y_pin),
        } = ctx.local;

        let input_timer_tick::SharedResources { game, input_timer } = ctx.shared;

        let x: u32 = adc.read(x_pin).unwrap();
        let y: u32 = adc.read(y_pin).unwrap();

        if x > 3_000 {
            game.button_click(GameButton::DPad(Dir::Right));
        } else if x < 1_000 {
            game.button_click(GameButton::DPad(Dir::Left));
        }

        if y > 3_000 {
            game.button_click(GameButton::DPad(Dir::Up));
        } else if y < 1_000 {
            game.button_click(GameButton::DPad(Dir::Down));
        }

        input_timer.clear_irq();
    }

    #[task(binds = EXTI4_15, local = [exti, rng_timer], shared = [game, vibro, vibro_timer])]
    fn button_press(ctx: button_press::Context) {
        let button_press::LocalResources { exti, rng_timer } = ctx.local;

        let button_press::SharedResources {
            game,
            vibro,
            vibro_timer,
        } = ctx.shared;

        vibro_timer.reset();
        vibro.set_low().unwrap();

        game.seed_random(rng_timer.get_current());

        if exti.is_pending(Event::GPIO7, SignalEdge::Rising) {
            game.button_click(GameButton::A);
            exti.unpend(Event::GPIO7);
        }

        if exti.is_pending(Event::GPIO4, SignalEdge::Rising) {
            game.button_click(GameButton::B);
            exti.unpend(Event::GPIO4);
        }
    }
}
