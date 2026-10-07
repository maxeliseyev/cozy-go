//! Gomoku rules for freestyle and exactly-five play.

use crate::bitset::{BitSet, points};
pub use crate::{Color, Illegal, Point};

/// Gomoku win conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Five or more consecutive stones win.
    Freestyle,
    /// Exactly five consecutive stones win; an overline does not.
    Standard,
}

/// Gomoku board and winner detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    size: u8,
    rule: Rule,
    black: BitSet,
    white: BitSet,
    winner: Option<(Color, [Point; 5])>,
}

impl Board {
    /// The default Gomoku board width and height.
    pub const DEFAULT_SIZE: u8 = 15;

    /// Creates an empty board from 5×5 through 19×19.
    #[must_use]
    pub fn new(size: u8, rule: Rule) -> Option<Self> {
        (5..=19).contains(&size).then_some(Self {
            size,
            rule,
            black: BitSet::default(),
            white: BitSet::default(),
            winner: None,
        })
    }

    /// Creates a board from stones, rejecting invalid, duplicate, or overlapping points.
    ///
    /// A winning line already present in the supplied position is recorded. Positions where
    /// both colors have a winning line are rejected.
    #[must_use]
    pub fn from_stones(size: u8, rule: Rule, black: &[Point], white: &[Point]) -> Option<Self> {
        let mut board = Self::new(size, rule)?;
        for &point in black {
            if !board.is_on_board(point) || board.get(point).is_some() {
                return None;
            }
            board.black.insert(point);
        }
        for &point in white {
            if !board.is_on_board(point) || board.get(point).is_some() {
                return None;
            }
            board.white.insert(point);
        }

        let black_winner = board.find_winner(Color::Black);
        let white_winner = board.find_winner(Color::White);
        if black_winner.is_some() && white_winner.is_some() {
            return None;
        }
        board.winner = black_winner.or(white_winner);
        Some(board)
    }

    /// Returns the board width and height.
    #[must_use]
    pub const fn size(&self) -> u8 {
        self.size
    }

    /// Returns the active win condition.
    #[must_use]
    pub const fn rule(&self) -> Rule {
        self.rule
    }

    /// Returns the color at a point, or `None` for an empty or off-board point.
    #[must_use]
    pub fn get(&self, point: Point) -> Option<Color> {
        if !self.is_on_board(point) {
            return None;
        }
        if self.black.contains(point) {
            Some(Color::Black)
        } else if self.white.contains(point) {
            Some(Color::White)
        } else {
            None
        }
    }

    /// Iterates over all intersections in ascending [`Point`] order.
    pub fn points(&self) -> impl Iterator<Item = Point> + '_ {
        points(self.size)
    }

    /// Places a stone and updates the winner if this move completes a line.
    ///
    /// # Errors
    ///
    /// Returns [`Illegal::GameOver`], [`Illegal::OffBoard`], or [`Illegal::Occupied`] when the
    /// move cannot be played. The board is unchanged on error.
    pub fn play(&mut self, color: Color, point: Point) -> Result<(), Illegal> {
        if self.winner.is_some() || self.is_full() {
            return Err(Illegal::GameOver);
        }
        if !self.is_on_board(point) {
            return Err(Illegal::OffBoard);
        }
        if self.get(point).is_some() {
            return Err(Illegal::Occupied);
        }

        match color {
            Color::Black => {
                self.black.insert(point);
            }
            Color::White => {
                self.white.insert(point);
            }
        }
        self.winner = self.winner_from(point, color);
        Ok(())
    }

    /// Returns the winning color and five consecutive points, if the game has a winner.
    #[must_use]
    pub const fn winner(&self) -> Option<(Color, [Point; 5])> {
        self.winner
    }

    /// Returns whether every intersection is occupied.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.black.count() + self.white.count() >= u32::from(self.size) * u32::from(self.size)
    }

    fn is_on_board(&self, point: Point) -> bool {
        point.index() < u16::from(self.size) * u16::from(self.size)
    }

    fn occupied_at(&self, x: i16, y: i16, color: Color) -> bool {
        if x < 0 || y < 0 || x >= i16::from(self.size) || y >= i16::from(self.size) {
            return false;
        }
        let index = y as u16 * u16::from(self.size) + x as u16;
        let point = Point::new(index);
        match color {
            Color::Black => self.black.contains(point),
            Color::White => self.white.contains(point),
        }
    }

    fn winner_from(&self, point: Point, color: Color) -> Option<(Color, [Point; 5])> {
        if self.get(point) != Some(color) {
            return None;
        }
        let x = i16::from((point.index() % u16::from(self.size)) as u8);
        let y = i16::from((point.index() / u16::from(self.size)) as u8);
        for (dx, dy) in [(1_i16, 0_i16), (0, 1), (1, 1), (1, -1)] {
            let mut before = 0_i16;
            while self.occupied_at(x - (before + 1) * dx, y - (before + 1) * dy, color) {
                before += 1;
            }
            let mut after = 0_i16;
            while self.occupied_at(x + (after + 1) * dx, y + (after + 1) * dy, color) {
                after += 1;
            }
            let length = before + after + 1;
            let wins = match self.rule {
                Rule::Freestyle => length >= 5,
                Rule::Standard => length == 5,
            };
            if !wins {
                continue;
            }

            let first_x = x - before * dx;
            let first_y = y - before * dy;
            let mut line = [Point::new(0); 5];
            for (offset, slot) in line.iter_mut().enumerate() {
                let step = offset as i16;
                let line_x = first_x + step * dx;
                let line_y = first_y + step * dy;
                let index = line_y as u16 * u16::from(self.size) + line_x as u16;
                *slot = Point::new(index);
            }
            return Some((color, line));
        }
        None
    }

    fn find_winner(&self, color: Color) -> Option<(Color, [Point; 5])> {
        self.points()
            .find_map(|point| self.winner_from(point, color))
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            size: Self::DEFAULT_SIZE,
            rule: Rule::Standard,
            black: BitSet::default(),
            white: BitSet::default(),
            winner: None,
        }
    }
}
