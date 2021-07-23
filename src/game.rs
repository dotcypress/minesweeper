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
                    Dir::Right => {
                        cursor = Point(u16::min(cursor.x() + 1, Board::WIDTH - 1), cursor.y());
                    }
                    Dir::Down => {
                        cursor = Point(cursor.x(), u16::min(cursor.y() + 1, Board::HEIGHT - 1));
                    }
                    Dir::Left => {
                        cursor = Point(cursor.x().saturating_sub(1), cursor.y());
                    }
                    Dir::Up => {
                        cursor = Point(cursor.x(), cursor.y().saturating_sub(1));
                    }
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
            let pos = Point(
                self.gen_random(Board::WIDTH),
                self.gen_random(Board::HEIGHT),
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
                let pos = Point(x, y);

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

    fn gen_random(&mut self, up_to: u16) -> u16 {
        self.rng_seed = self.rng_seed * 16_807 % 0x7fff_ffff;
        (self.rng_seed % up_to as u32) as u16
    }
}

widget!(
  view: GameUI<&Minesweeper> {
    bg(Background, Point(0, 0), Size(128, 64), 1)
    logo(Icon<RomSprite>, Point(0, 0), LOGO, b'~')
    game_screen(GameScreen)
  },
  set_state: |view: &mut GameUI, state: &Minesweeper| {
    view.game_screen.set_state(state);
  }
);

widget!(
  view: GameScreen<&Minesweeper> {
    board(GameBoard)
    win(Icon<RomSprite>, Point(24, 24), POPUP, b'W')
    game_over(Icon<RomSprite>, Point(24, 24), POPUP, b'L')
  },
  active_node: board,
  set_state: |view: &mut GameScreen, state: &Minesweeper| {
    let node = match state.status {
      GameStatus::GameOver => GameScreenNode::GameOver,
      GameStatus::Win => GameScreenNode::Win,
      _ => GameScreenNode::Board,
    };
    view.set_active(node);
    view.board.set_state(&state.board);
  }
);

pub type GameWidget = TextBox<RomSprite, { Board::TILES }, { Board::WIDTH as u16 }>;

widget!(
  view: GameBoard<&Board> {
    field(GameWidget, Point(0, 16), GAME_TILES, "")
  },
  set_state: |view: &mut GameBoard, state: &Board| {
    let cursor_idx = state.cursor_offset();
    for (idx, tile) in state.tiles().iter().enumerate() {
      let mut glyph = tile.into();
      if idx == cursor_idx {
        glyph += 13;
      }
      view.field.set_glyph(idx, glyph);
    }
  }
);
