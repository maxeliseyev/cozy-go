# cozy-go

`cozy-go` is a small, deterministic Rust rules library for Go and Gomoku. It does not depend
on the Konstruct game SDK; the SDK's `games/go` and `games/gomoku` crates can use it as their
rules layer.

The crate is `no_std` and uses `alloc`, with no third-party dependencies, floating-point
arithmetic, runtime randomness, or hash maps. It targets Rust 1.96.0 and
`wasm32-unknown-unknown`.

## Go

- Board sizes: 9×9, 13×13, and 19×19.
- Positional superko, checked by `Game` across all prior board hashes.
- Suicide is illegal. Captures are simultaneous and returned by `Board::play`.
- Two consecutive passes end the move phase.
- `Game::area_score` uses Chinese area scoring and removes the supplied dead stones before
  counting. Komi is supplied in half-points and included in White's score.
- Board hashes cover stones only, use deterministic keys, and do not include whose turn it is.

`Board::play` applies local placement and capture rules. Use `Game::play` when the caller
needs turn management, passes, and superko.

## Gomoku

- Board sizes: 5×5 through 19×19; the default is 15×15.
- `Rule::Freestyle` wins with five or more stones in a row.
- `Rule::Standard` wins with exactly five; an overline does not win.
- Renju restrictions are outside this crate's current scope.

`Color`, `Point`, and `Illegal` are shared types, and are also re-exported from each game
module. Coordinates use `(0, 0)` at the lower-left corner; points iterate in ascending
row-major order.

## Example

```rust
use cozy_go::go::{Game, Move};
use cozy_go::Point;

if let (Some(mut game), Some(point)) = (Game::new(19, 13), Point::from_xy(19, 3, 3)) {
    let _ = game.play(Move::Place(point));
}
```

The library implements rules and position helpers only. State serialization, player IDs,
network protocol, and board rendering belong to the caller.
