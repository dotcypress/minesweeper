use klaptik::*;

use crate::board::*;
use crate::sprites::*;

#[derive(PartialEq, Debug)]
pub enum GameButton {
    DPad(Dir),
    A,
    B,
}

#[derive(PartialEq, Debug)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Copy, Clone)]
pub enum GameStatus {
    Win,
    Bootstrap,
    Playing,
    GameOver,
}

pub struct Minesweeper {
    board: Board,
    status: GameStatus,
    bombs: usize,
    rng_seed: u32,
}

impl Minesweeper {
    pub fn new(bombs: usize) -> Self {
        assert!(bombs < Board::TILES);
        Self {
            bombs,
            board: Board::new(),
            status: GameStatus::Bootstrap,
            rng_seed: 42,
        }
    }

    pub fn seed_random(&mut self, seed: u32) {
        self.rng_seed = seed % 0x7fff_ffff;
    }

    pub fn button_click(&mut self, button: GameButton) {
        let mut cursor = self.board.cursor();
        match button {
            GameButton::A => match self.status {
                GameStatus::Bootstrap => {
                    self.bootstrap();
                    self.open_tile(cursor)
                }
                GameStatus::Playing => {
                    self.open_tile(cursor);
                    self.refresh_game_state()
                }
                _ => self.bootstrap(),
            },
            GameButton::B => {
                match self.board.tile_at(cursor).status() {
                    TileStatus::Closed => self.board.set_status_at(cursor, TileStatus::Flagged),
                    TileStatus::Flagged => self.board.set_status_at(cursor, TileStatus::Closed),
                    _ => {}
                };
                self.refresh_game_state()
            }
            GameButton::DPad(dir) => {
                match dir {
                    Dir::Left if cursor.x > 0 => {
                        cursor = Point::new(cursor.x - 1, cursor.y);
                    }
                    Dir::Right if cursor.x + 1 < Board::WIDTH as i32 => {
                        cursor = Point::new(cursor.x + 1, cursor.y);
                    }
                    Dir::Up if cursor.y > 0 => {
                        cursor = Point::new(cursor.x, cursor.y - 1);
                    }
                    Dir::Down if cursor.y + 1 < Board::HEIGHT as i32 => {
                        cursor = Point::new(cursor.x, cursor.y + 1);
                    }
                    _ => {}
                }
                self.board.move_cursor(cursor);
            }
        };
    }

    fn refresh_game_state(&mut self) {
        if self
            .board
            .tiles()
            .iter()
            .any(|&tile| tile.status() == TileStatus::Opened && tile.content() == TileContent::Bomb)
        {
            self.status = GameStatus::GameOver;
            return;
        }

        let win = self.board.tiles().iter().all(|&tile| {
            matches!(
                (tile.status(), tile.content()),
                (TileStatus::Flagged, TileContent::Bomb) | (TileStatus::Opened, _)
            )
        });

        if win {
            self.status = GameStatus::Win;
        }
    }

    fn open_tile(&mut self, origin: Point) {
        if let TileStatus::Closed = self.board.tile_at(origin).status() {
            match self.board.tile_at(origin).content() {
                TileContent::Hint(0) => {
                    self.board.set_status_at(origin, TileStatus::Opened);
                    for neighbor in Neighbors::at(origin) {
                        self.open_tile(neighbor);
                    }
                }
                _ => self.board.set_status_at(origin, TileStatus::Opened),
            }
        }
    }

    fn bootstrap(&mut self) {
        self.board.reset();

        let mut bombs_planted = 0;
        while bombs_planted < self.bombs {
            let pos = Point::new(
                self.gen_random(Board::WIDTH as u16),
                self.gen_random(Board::HEIGHT as u16),
            );
            match self.board.tile_at(pos).content() {
                TileContent::Hint(_) if pos != self.board.cursor() => {
                    self.board.set_content_at(pos, TileContent::Bomb);
                    bombs_planted += 1;
                }
                _ => {}
            }
        }

        for x in 0..Board::WIDTH {
            for y in 0..Board::HEIGHT {
                let pos = Point::new(x as i32, y as i32);

                if let TileContent::Bomb = self.board.tile_at(pos).content() {
                    continue;
                }

                let mut bombs = 0;
                for neighbor in Neighbors::at(pos) {
                    if let TileContent::Bomb = self.board.tile_at(neighbor).content() {
                        bombs += 1;
                    }
                }

                self.board.set_content_at(pos, TileContent::Hint(bombs));
            }
        }

        self.status = GameStatus::Playing;
    }

    fn gen_random(&mut self, up_to: u16) -> i32 {
        self.rng_seed = self.rng_seed * 16_807 % 0x7fff_ffff;
        (self.rng_seed % up_to as u32) as i32
    }
}

widget! {
    GameUI<&Minesweeper>,
    nodes:  {
        bg: Background, Point::new(0, 0), Size::new(128, 64);
        logo: SpriteIcon, LOGO, b'~', Point::new(0, 0);
        game_screen: GameScreen;
    },
    update: |nodes: &mut GameUI, state: &Minesweeper| {
        nodes.game_screen.update(state);
    }
}

widget!(
    GameScreen<&Minesweeper>,
    nodes: {
        board: GameBoard;
        win: SpriteIcon, POPUP, b'W', Point::new(24, 24);
        game_over: SpriteIcon, POPUP, b'L', Point::new(24, 24);
    },
    active: board,
    update: |nodes: &mut GameScreen, state: &Minesweeper| {
        let node = match state.status {
            GameStatus::GameOver => GameScreenNode::GameOver,
            GameStatus::Win => GameScreenNode::Win,
            _ => GameScreenNode::Board,
        };
        nodes.set_active(Some(node));
        nodes.board.update(&state.board);
    }
);

pub type GameWidget = TextBox<RomSprite, { Board::TILES }, 8, 8, { Board::WIDTH as _ }>;

widget!(
    GameBoard<&Board>,
    nodes: {
        // bg: Background, Point::new(0, 16), Size::new(128, 48);
        field: GameWidget, GAME_TILES, "", Point::new(0, 16);
    },
    update: |nodes: &mut GameBoard, state: &Board| {
        let cursor_idx = state.cursor_offset();
        for (idx, tile) in state.tiles().iter().enumerate() {
            let mut glyph = tile.into();
            if idx == cursor_idx {
                glyph += 13;
            }
            nodes.field.set_glyph(idx, glyph);
        }
    }
);
