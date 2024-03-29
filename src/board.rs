use klaptik::*;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TileStatus {
    Closed,
    Flagged,
    Opened,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TileContent {
    Bomb,
    Hint(u8),
}

#[derive(Copy, Clone)]
pub struct Tile {
    status: TileStatus,
    content: TileContent,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            status: TileStatus::Closed,
            content: TileContent::Hint(0),
        }
    }
}

impl Tile {
    pub fn new(status: TileStatus, content: TileContent) -> Self {
        Self { status, content }
    }

    pub fn status(&self) -> TileStatus {
        self.status
    }

    pub fn content(&self) -> TileContent {
        self.content
    }
}

impl From<&Tile> for Glyph {
    fn from(tile: &Tile) -> Self {
        match tile.status() {
            TileStatus::Closed => b'-',
            TileStatus::Flagged => b'.',
            TileStatus::Opened => match tile.content() {
                TileContent::Bomb => b'/',
                TileContent::Hint(0) => b',',
                TileContent::Hint(hint) => b'0' + hint,
            },
        }
    }
}

pub struct Board {
    cursor: Point,
    tiles: [Tile; Self::TILES],
}

impl Board {
    pub const WIDTH: usize = 16;
    pub const HEIGHT: usize = 6;
    pub const TILES: usize = Self::HEIGHT * Self::WIDTH;

    pub fn new() -> Self {
        Self {
            tiles: [Tile::default(); Self::TILES],
            cursor: Point::new(Self::WIDTH as i32 / 2, Self::HEIGHT as i32 / 2),
        }
    }

    pub fn reset(&mut self) {
        for tile in self.tiles.iter_mut() {
            *tile = Tile::default()
        }
    }

    pub fn set_status_at(&mut self, pos: Point, status: TileStatus) {
        self.tiles[Self::point_offset(pos)].status = status
    }

    pub fn set_content_at(&mut self, pos: Point, content: TileContent) {
        self.tiles[Self::point_offset(pos)].content = content;
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn tile_at(&self, pos: Point) -> Tile {
        self.tiles[Self::point_offset(pos)]
    }

    pub fn cursor(&self) -> Point {
        self.cursor
    }

    pub fn move_cursor(&mut self, target: Point) {
        self.cursor = target;
    }

    pub fn cursor_offset(&self) -> usize {
        Self::point_offset(self.cursor)
    }

    fn point_offset(point: Point) -> usize {
        point.x as usize + point.y as usize * Board::WIDTH
    }
}

pub struct Neighbors {
    origin: Point,
    next: usize,
}

impl Neighbors {
    const NEIGHBORHOOD: [(i32, i32); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
        (1, 0),
        (1, -1),
        (0, -1),
    ];

    pub fn at(origin: Point) -> Self {
        Self { origin, next: 0 }
    }
}

impl Iterator for Neighbors {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next >= Self::NEIGHBORHOOD.len() {
                return None;
            }

            let addr = Self::NEIGHBORHOOD[self.next];
            self.next += 1;

            let x = self.origin.x + addr.0;
            let y = self.origin.y + addr.1;

            if x >= 0 && y >= 0 && x < Board::WIDTH as i32 && y < Board::HEIGHT as i32 {
                return Some(Point::new(x, y));
            }
        }
    }
}
