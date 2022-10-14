#![no_std]
#![no_main]
// #![deny(warnings)]

extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

mod board;
mod game;
mod sprites;
mod wiring;

use defmt_rtt as _;

use hal::gpio::*;
use hal::i2c;
use hal::prelude::*;
use klaptik::drivers::st7567::*;
use klaptik::*;
use wii_ext::nunchuk::*;

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
        render_timer: RenderTimer,
        #[lock_free]
        input_timer: InputTimer,
        #[lock_free]
        rng_timer: RngTimer,
    }

    #[local]
    struct Local {
        display: DisplayController,
        ui: GameUI,
        nunchuk: Joystick,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.constrain();
        let port_a = ctx.device.GPIOA.split(&mut rcc);

        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let mut rng_timer = ctx.device.TIM3.timer(&mut rcc);
        rng_timer.resume();

        let mut render_timer = ctx.device.TIM14.timer(&mut rcc);
        render_timer.start(40.millis());
        render_timer.listen();

        let mut input_timer = ctx.device.TIM17.timer(&mut rcc);
        input_timer.start(100.millis());
        input_timer.listen();

        let spi = ctx.device.SPI2.spi(
            (port_a.pa0, hal::spi::NoMiso, port_a.pa4),
            hal::spi::MODE_0,
            4.MHz(),
            &mut rcc,
        );
        let mut display = ST7567::new(
            spi,
            port_a.pa7.into_push_pull_output(),
            port_a.pa3.into_push_pull_output(),
            port_a.pa5.into_push_pull_output(),
        );
        display.set_offset(Point::new(4, 0));

        display.reset(&mut delay);
        display
            .link()
            .command(|tx| {
                tx.write(&[
                    // Command::Bias1_9 as _,
                    Command::SegmentDirectionRev as _,
                    // Command::SetCOMNormal as _,
                ])
            })
            .ok();
        display.on();

        let sda = port_a.pa12.into_open_drain_output();
        let scl = port_a.pa11.into_open_drain_output();
        let i2c_config = i2c::Config::new(100.kHz());
        let i2c = ctx.device.I2C2.i2c(sda, scl, i2c_config, &mut rcc);
        let nunchuk = Nunchuk::new(i2c, &mut delay).unwrap();

        let game = Minesweeper::new(BOMBS);
        let mut ui = GameUI::new();
        ui.update(&game);

        port_a.pa6.into_open_drain_output_in_state(PinState::Low);

        (
            Shared {
                game,
                input_timer,
                render_timer,
                rng_timer,
            },
            Local {
                ui,
                display,
                nunchuk,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM14, local = [display, ui], shared = [game, render_timer])]
    fn render_timer_tick(ctx: render_timer_tick::Context) {
        let render_timer_tick::LocalResources { display, ui } = ctx.local;
        let render_timer_tick::SharedResources { game, render_timer } = ctx.shared;

        ui.update(&game);
        ui.render(display);

        render_timer.clear_irq();
    }

    #[task(binds = TIM17, local = [nunchuk], shared = [game, input_timer, rng_timer])]
    fn input_timer_tick(ctx: input_timer_tick::Context) {
        let input_timer_tick::LocalResources { nunchuk } = ctx.local;
        let input_timer_tick::SharedResources {
            game,
            input_timer,
            rng_timer,
        } = ctx.shared;
        let state = nunchuk.read_no_wait().unwrap();

        if state.button_z {
            game.seed_random(rng_timer.get_current());
            game.button_click(GameButton::A);
        }
        if state.button_c {
            game.button_click(GameButton::B);
        }

        if state.joystick_x > (127 + 64) {
            game.button_click(GameButton::DPad(Dir::Right));
        } else if state.joystick_x < (127 - 64) {
            game.button_click(GameButton::DPad(Dir::Left));
        }

        if state.joystick_y > (127 + 64) {
            game.button_click(GameButton::DPad(Dir::Up));
        } else if state.joystick_y < (127 - 64) {
            game.button_click(GameButton::DPad(Dir::Down));
        }

        input_timer.clear_irq();
    }
}
