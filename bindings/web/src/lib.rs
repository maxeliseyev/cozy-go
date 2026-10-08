//! Synchronous, binary WebAssembly bridge for browser and Node.js callers.
//! The input and output buffers carry values only; game state is supplied on every call.

use cozy_go::go::{Board, Game, Move};
use cozy_go::{Color, Illegal, Point};

const INPUT_CAPACITY: usize = 128 * 1024;
const OUTPUT_CAPACITY: usize = 4096;
const INVALID_INPUT: u8 = 6;
static mut INPUT: [u8; INPUT_CAPACITY] = [0; INPUT_CAPACITY];
static mut OUTPUT: [u8; OUTPUT_CAPACITY] = [0; OUTPUT_CAPACITY];

/// Version of this bridge's binary contract.
#[unsafe(no_mangle)]
pub extern "C" fn cozy_web_version() -> u32 {
    1
}

/// Address of the input buffer in exported WebAssembly memory.
#[unsafe(no_mangle)]
pub extern "C" fn cozy_web_input_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(INPUT).cast()
}

/// Maximum number of input bytes.
#[unsafe(no_mangle)]
pub extern "C" fn cozy_web_input_capacity() -> u32 {
    INPUT_CAPACITY as u32
}

/// Address of the output buffer in exported WebAssembly memory.
#[unsafe(no_mangle)]
pub extern "C" fn cozy_web_output_ptr() -> *const u8 {
    core::ptr::addr_of!(OUTPUT).cast()
}

/// Executes an operation and returns its output length, or zero for an oversized input.
///
/// Output starts with zero on success, an `Illegal` code (1–5) on a rejected move, or six
/// for malformed input. Operations are hash (1), group (2), area score (3), and move (4).
///
/// # Safety
/// The caller must serialize access to the input and output buffers, and must not write
/// either buffer while this function runs. The supplied JS bridge calls this synchronously
/// in a WebAssembly instance with unshared memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cozy_web_run(operation: u32, input_len: u32) -> u32 {
    let length = input_len as usize;
    if length > INPUT_CAPACITY {
        return 0;
    }
    // SAFETY: length was checked against the static input buffer; caller serializes access.
    let input = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(INPUT).cast(), length) };
    let bytes = execute(operation, input).unwrap_or_else(|code| vec![code]);
    if bytes.len() > OUTPUT_CAPACITY {
        return 0;
    }
    // SAFETY: the slice is within OUTPUT and cannot overlap INPUT; caller serializes access.
    let output = unsafe {
        core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(OUTPUT).cast(), bytes.len())
    };
    output.copy_from_slice(&bytes);
    bytes.len() as u32
}

fn execute(operation: u32, input: &[u8]) -> Result<Vec<u8>, u8> {
    let mut reader = Reader(input);
    let board = reader.board()?;
    let mut output = vec![0];
    match operation {
        1 => {
            output.extend_from_slice(&board.hash().to_le_bytes());
        }
        2 => {
            let point = Point::new(reader.u16()?);
            let stones: Vec<_> = board.group(point).collect();
            let liberties: Vec<_> = board.liberties(point).collect();
            output.extend_from_slice(&(stones.len() as u16).to_le_bytes());
            output.extend_from_slice(&(liberties.len() as u16).to_le_bytes());
            for point in stones.into_iter().chain(liberties) {
                output.extend_from_slice(&point.index().to_le_bytes());
            }
        }
        3 => {
            let komi = reader.i32()?;
            if komi < 0 {
                return Err(INVALID_INPUT);
            }
            let count = usize::from(reader.u16()?);
            if count > 361 {
                return Err(INVALID_INPUT);
            }
            let mut dead = Vec::with_capacity(count);
            for _ in 0..count {
                dead.push(Point::new(reader.u16()?));
            }
            let area = board.area(&dead);
            output.extend_from_slice(&area.black.to_le_bytes());
            output.extend_from_slice(&(area.white * 2 + komi as u32).to_le_bytes());
            output.extend(area.territory().iter().copied().map(color_byte));
        }
        4 => {
            let color = match reader.u8()? {
                1 => Color::Black,
                2 => Color::White,
                _ => return Err(INVALID_INPUT),
            };
            let passes = reader.u8()?;
            let point = reader.u16()?;
            let komi = reader.i32()?;
            let count = reader.u32()? as usize;
            if count > 10001 {
                return Err(INVALID_INPUT);
            }
            let mut history = Vec::with_capacity(count);
            for _ in 0..count {
                history.push(reader.u64()?);
            }
            let opponent_before = board
                .points()
                .filter(|&p| board.get(p) == Some(color.opponent()))
                .count();
            let mut game =
                Game::from_position(board, color, &history, passes, komi).ok_or(INVALID_INPUT)?;
            let mv = if point == u16::MAX {
                Move::Pass
            } else {
                Move::Place(Point::new(point))
            };
            game.play(mv).map_err(illegal_code)?;
            let opponent_after = game
                .board()
                .points()
                .filter(|&p| game.board().get(p) == Some(color.opponent()))
                .count();
            output.push(color_byte(Some(game.to_move())));
            output.push(game.consecutive_passes());
            output.extend_from_slice(&((opponent_before - opponent_after) as u16).to_le_bytes());
            output.extend_from_slice(&game.board().hash().to_le_bytes());
            output.extend(
                game.board()
                    .points()
                    .map(|p| color_byte(game.board().get(p))),
            );
        }
        _ => return Err(INVALID_INPUT),
    }
    if !reader.0.is_empty() {
        return Err(INVALID_INPUT);
    }
    Ok(output)
}

fn color_byte(color: Option<Color>) -> u8 {
    match color {
        None => 0,
        Some(Color::Black) => 1,
        Some(Color::White) => 2,
    }
}

fn illegal_code(illegal: Illegal) -> u8 {
    match illegal {
        Illegal::Occupied => 1,
        Illegal::Suicide => 2,
        Illegal::Superko => 3,
        Illegal::OffBoard => 4,
        Illegal::GameOver => 5,
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], u8> {
        let part = self.0.get(..length).ok_or(INVALID_INPUT)?;
        self.0 = self.0.get(length..).ok_or(INVALID_INPUT)?;
        Ok(part)
    }

    fn u8(&mut self) -> Result<u8, u8> {
        self.take(1)?.first().copied().ok_or(INVALID_INPUT)
    }

    fn u16(&mut self) -> Result<u16, u8> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| INVALID_INPUT)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, u8> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| INVALID_INPUT)?,
        ))
    }

    fn i32(&mut self) -> Result<i32, u8> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| INVALID_INPUT)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, u8> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| INVALID_INPUT)?,
        ))
    }

    fn board(&mut self) -> Result<Board, u8> {
        let size = self.u8()?;
        if ![9, 13, 19].contains(&size) {
            return Err(INVALID_INPUT);
        }
        let cells = self.take(usize::from(size) * usize::from(size))?;
        let mut black = Vec::new();
        let mut white = Vec::new();
        for (index, cell) in cells.iter().enumerate() {
            match cell {
                0 => {}
                1 => black.push(Point::new(index as u16)),
                2 => white.push(Point::new(index as u16)),
                _ => return Err(INVALID_INPUT),
            }
        }
        Board::from_stones(size, &black, &white).ok_or(INVALID_INPUT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_requests_are_rejected_before_execution() {
        for operation in 0..=5 {
            for length in 0..82 {
                assert!(execute(operation, &vec![0; length]).is_err());
            }
        }
        let mut valid = vec![9];
        valid.extend([0; 81]);
        assert_eq!(execute(1, &valid), Ok(vec![0; 9]));
        valid.push(0);
        assert_eq!(execute(1, &valid), Err(INVALID_INPUT));
    }
}
