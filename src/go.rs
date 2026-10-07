//! Go rules: positional superko, suicide prevention, captures, and Chinese area scoring.

use alloc::vec;
use alloc::vec::Vec;

use crate::bitset::{BitSet, points};
pub use crate::{Color, Illegal, Point};

/// The supported Go board sizes.
pub const BOARD_SIZES: [u8; 3] = [9, 13, 19];

/// A Go move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Place a stone at an intersection.
    Place(Point),
    /// Pass the turn.
    Pass,
}

/// Stones removed by one legal placement.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Captures {
    stones: Vec<Point>,
}

impl Captures {
    /// Returns the captured intersections in deterministic order.
    #[must_use]
    pub fn points(&self) -> &[Point] {
        &self.stones
    }

    /// Returns the number of captured stones.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stones.len()
    }

    /// Returns whether this move captured no stones.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stones.is_empty()
    }
}

/// Go board storage and the rules for one placement, without repetition history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    size: u8,
    black: BitSet,
    white: BitSet,
}

impl Board {
    /// Creates an empty 9×9, 13×13, or 19×19 board.
    #[must_use]
    pub fn new(size: u8) -> Option<Self> {
        BOARD_SIZES.contains(&size).then_some(Self {
            size,
            black: BitSet::default(),
            white: BitSet::default(),
        })
    }

    /// Creates a board from stones, rejecting off-board, duplicate, or overlapping points.
    #[must_use]
    pub fn from_stones(size: u8, black: &[Point], white: &[Point]) -> Option<Self> {
        let mut board = Self::new(size)?;
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
        Some(board)
    }

    /// Returns the board width and height.
    #[must_use]
    pub const fn size(&self) -> u8 {
        self.size
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

    /// Places a stone and removes adjacent opponent groups without liberties.
    ///
    /// This method does not apply superko; [`Game::play`] does.
    ///
    /// # Errors
    ///
    /// Returns [`Illegal::OffBoard`], [`Illegal::Occupied`], or [`Illegal::Suicide`] when the
    /// placement is not legal. The board is unchanged on error.
    pub fn play(&mut self, color: Color, point: Point) -> Result<Captures, Illegal> {
        if !self.is_on_board(point) {
            return Err(Illegal::OffBoard);
        }
        if self.get(point).is_some() {
            return Err(Illegal::Occupied);
        }

        let mut next = self.clone();
        next.set_stone(color, point);

        let opponent = color.opponent();
        let mut examined = BitSet::default();
        let mut captured = Vec::new();
        for neighbor in next.neighbors(point).into_iter().flatten() {
            if next.get(neighbor) != Some(opponent) || examined.contains(neighbor) {
                continue;
            }
            let (group, has_liberty) = next.group_and_liberties(neighbor, opponent);
            for &stone in &group {
                examined.insert(stone);
            }
            if !has_liberty {
                captured.extend(group);
            }
        }

        for &stone in &captured {
            next.remove_stone(opponent, stone);
        }

        let (_, has_liberty) = next.group_and_liberties(point, color);
        if !has_liberty {
            return Err(Illegal::Suicide);
        }

        *self = next;
        Ok(Captures { stones: captured })
    }

    /// Returns the connected group containing `point`; an empty or invalid point yields an
    /// empty iterator.
    pub fn group(&self, point: Point) -> impl Iterator<Item = Point> {
        let stones = self
            .get(point)
            .map(|color| self.group_and_liberties(point, color).0)
            .unwrap_or_default();
        stones.into_iter()
    }

    /// Returns the deterministic Zobrist hash of stones, independent of the player to move.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.points().fold(0, |hash, point| {
            let key = match self.get(point) {
                Some(Color::Black) => zobrist(point, 0),
                Some(Color::White) => zobrist(point, 1),
                None => 0,
            };
            hash ^ key
        })
    }

    fn is_on_board(&self, point: Point) -> bool {
        point.index() < u16::from(self.size) * u16::from(self.size)
    }

    fn set_stone(&mut self, color: Color, point: Point) {
        match color {
            Color::Black => {
                self.black.insert(point);
            }
            Color::White => {
                self.white.insert(point);
            }
        }
    }

    fn remove_stone(&mut self, color: Color, point: Point) {
        match color {
            Color::Black => {
                self.black.remove(point);
            }
            Color::White => {
                self.white.remove(point);
            }
        }
    }

    fn neighbors(&self, point: Point) -> [Option<Point>; 4] {
        let size = u16::from(self.size);
        let index = point.index();
        let x = index % size;
        let y = index / size;
        [
            (x > 0).then(|| Point::new(index - 1)),
            (x + 1 < size).then(|| Point::new(index + 1)),
            (y > 0).then(|| Point::new(index - size)),
            (y + 1 < size).then(|| Point::new(index + size)),
        ]
    }

    fn group_and_liberties(&self, start: Point, color: Color) -> (Vec<Point>, bool) {
        let mut group = Vec::new();
        let mut stack = vec![start];
        let mut visited = BitSet::default();
        let mut liberties = BitSet::default();

        while let Some(point) = stack.pop() {
            if !visited.insert(point) {
                continue;
            }
            group.push(point);
            for neighbor in self.neighbors(point).into_iter().flatten() {
                match self.get(neighbor) {
                    Some(found) if found == color => {
                        if !visited.contains(neighbor) {
                            stack.push(neighbor);
                        }
                    }
                    None => {
                        liberties.insert(neighbor);
                    }
                    Some(_) => {}
                }
            }
        }

        (group, liberties.count() > 0)
    }
}

/// A Go position with turn, positional-superko history, pass count, and komi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    board: Board,
    to_move: Color,
    history: Vec<u64>,
    consecutive_passes: u8,
    komi_half_points: i32,
}

impl Game {
    /// Starts an empty game with Black to move and the supplied nonnegative komi in half-points.
    #[must_use]
    pub fn new(size: u8, komi_half_points: i32) -> Option<Self> {
        if komi_half_points < 0 {
            return None;
        }
        let board = Board::new(size)?;
        Some(Self {
            history: vec![board.hash()],
            board,
            to_move: Color::Black,
            consecutive_passes: 0,
            komi_half_points,
        })
    }

    /// Restores a position and its repetition history for a caller-owned serialized state.
    ///
    /// The current board hash must be the last history entry. History entries before the last
    /// are accepted as supplied and are used for positional superko checks.
    #[must_use]
    pub fn from_position(
        board: Board,
        to_move: Color,
        history: &[u64],
        consecutive_passes: u8,
        komi_half_points: i32,
    ) -> Option<Self> {
        if history.last().copied() != Some(board.hash())
            || consecutive_passes > 2
            || komi_half_points < 0
        {
            return None;
        }
        Some(Self {
            board,
            to_move,
            history: Vec::from(history),
            consecutive_passes,
            komi_half_points,
        })
    }

    /// Returns the current board.
    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// Returns the color to move.
    #[must_use]
    pub const fn to_move(&self) -> Color {
        self.to_move
    }

    /// Returns the number of consecutive passes, capped at two.
    #[must_use]
    pub const fn consecutive_passes(&self) -> u8 {
        self.consecutive_passes
    }

    /// Returns komi in half-points.
    #[must_use]
    pub const fn komi_half_points(&self) -> i32 {
        self.komi_half_points
    }

    /// Returns every position hash since the start, with the current position last.
    #[must_use]
    pub fn history(&self) -> &[u64] {
        &self.history
    }

    /// Returns whether `mv` is legal for the current player.
    ///
    /// # Errors
    ///
    /// Returns a rule-specific [`Illegal`] reason, including [`Illegal::Superko`] when a
    /// placement recreates any earlier board position.
    pub fn is_legal(&self, mv: Move) -> Result<(), Illegal> {
        if self.consecutive_passes >= 2 {
            return Err(Illegal::GameOver);
        }
        match mv {
            Move::Pass => Ok(()),
            Move::Place(point) => {
                let mut next = self.board.clone();
                next.play(self.to_move, point)?;
                if self.history.contains(&next.hash()) {
                    Err(Illegal::Superko)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Iterates over legal placements in ascending point order, followed by pass.
    pub fn legal_moves(&self) -> impl Iterator<Item = Move> + '_ {
        let placements = self.board.points().filter_map(move |point| {
            let mv = Move::Place(point);
            self.is_legal(mv).ok().map(|()| mv)
        });
        let pass = core::iter::once(Move::Pass).filter(move |_| self.is_legal(Move::Pass).is_ok());
        placements.chain(pass)
    }

    /// Applies a legal move and advances the turn and repetition history.
    ///
    /// # Errors
    ///
    /// Returns the reason from [`Self::is_legal`] when the move is rejected. The game is
    /// unchanged on error.
    pub fn play(&mut self, mv: Move) -> Result<(), Illegal> {
        self.is_legal(mv)?;
        match mv {
            Move::Pass => {
                self.consecutive_passes = self.consecutive_passes.saturating_add(1).min(2);
            }
            Move::Place(point) => {
                self.board.play(self.to_move, point)?;
                self.consecutive_passes = 0;
            }
        }
        self.to_move = self.to_move.opponent();
        self.history.push(self.board.hash());
        Ok(())
    }

    /// Calculates Chinese area score after removing the supplied dead stones.
    ///
    /// Off-board points and points that are already empty are ignored. Komi is included in
    /// `white_half_points`; Black's score is returned in whole points.
    #[must_use]
    pub fn area_score(&self, dead: &[Point]) -> Score {
        let mut board = self.board.clone();
        for &point in dead {
            if let Some(color) = board.get(point) {
                board.remove_stone(color, point);
            }
        }

        let mut visited = BitSet::default();
        let mut black_territory = 0_u32;
        let mut white_territory = 0_u32;
        for start in board.points() {
            if board.get(start).is_some() || visited.contains(start) {
                continue;
            }
            let (area, touches_black, touches_white) = empty_region(&board, start, &mut visited);
            match (touches_black, touches_white) {
                (true, false) => black_territory += area,
                (false, true) => white_territory += area,
                _ => {}
            }
        }

        Score {
            black: board.black.count() + black_territory,
            white_half_points: (board.white.count() + white_territory) * 2
                + self.komi_half_points as u32,
        }
    }
}

/// Chinese area score; Black is in whole points and White includes komi in half-points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    /// Black stones plus Black territory.
    pub black: u32,
    /// Twice White's area plus komi in half-points.
    pub white_half_points: u32,
}

fn empty_region(board: &Board, start: Point, visited: &mut BitSet) -> (u32, bool, bool) {
    let mut stack = vec![start];
    visited.insert(start);
    let mut area = 0_u32;
    let mut touches_black = false;
    let mut touches_white = false;

    while let Some(point) = stack.pop() {
        area += 1;
        for neighbor in board.neighbors(point).into_iter().flatten() {
            match board.get(neighbor) {
                Some(Color::Black) => touches_black = true,
                Some(Color::White) => touches_white = true,
                None if visited.insert(neighbor) => stack.push(neighbor),
                None => {}
            }
        }
    }

    (area, touches_black, touches_white)
}

const fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

const fn zobrist(point: Point, color: u64) -> u64 {
    splitmix64(
        0x434F_5A59_474F_0001
            ^ ((point.index() as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93))
            ^ color.wrapping_mul(0xA076_1D64_78BD_642F),
    )
}
