//! Deterministic, allocation-only rules for Go and Gomoku.
//!
//! The crate is independent of the Konstruct game SDK. It can be compiled for
//! `wasm32-unknown-unknown` and leaves state encoding, turn ownership, and user interface
//! decisions to its caller.

#![no_std]
#![deny(clippy::float_arithmetic)]

extern crate alloc;

mod bitset;
pub mod go;
pub mod gomoku;

/// The two stone colors used by both games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Color {
    /// Black stones.
    Black,
    /// White stones.
    White,
}

impl Color {
    /// Returns the other color.
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

/// A board intersection, numbered from the lower-left corner in row-major order.
///
/// An index is not tied to a board size. Game methods reject a point whose index is outside
/// their board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point(u16);

impl Point {
    /// Creates a point from its row-major index.
    #[must_use]
    pub const fn new(index: u16) -> Self {
        Self(index)
    }

    /// Creates a point from coordinates, or returns `None` when they are outside the board.
    #[must_use]
    pub const fn from_xy(size: u8, x: u8, y: u8) -> Option<Self> {
        if size == 0 || x >= size || y >= size {
            return None;
        }
        Some(Self(y as u16 * size as u16 + x as u16))
    }

    /// Returns the row-major index.
    #[must_use]
    pub const fn index(self) -> u16 {
        self.0
    }

    /// Converts this point to `(x, y)` for the given board size.
    #[must_use]
    pub const fn coordinates(self, size: u8) -> Option<(u8, u8)> {
        if size == 0 || self.0 >= size as u16 * size as u16 {
            return None;
        }
        Some(((self.0 % size as u16) as u8, (self.0 / size as u16) as u8))
    }
}

/// A move rejected by either rules engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Illegal {
    /// The point is already occupied.
    Occupied,
    /// The move leaves the played stone's group without liberties.
    Suicide,
    /// The move recreates a position forbidden by positional superko.
    Superko,
    /// The point lies outside the board.
    OffBoard,
    /// The game has already ended.
    GameOver,
}
