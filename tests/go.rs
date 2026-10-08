use cozy_go::go::{Board, Game, Move};
use cozy_go::{Color, Illegal, Point};

fn p(x: u8, y: u8) -> Point {
    Point::from_xy(9, x, y).expect("test coordinates are on board")
}

#[test]
fn connected_group_has_unique_liberties_and_invalid_points_are_empty() {
    let board = Board::from_stones(9, &[p(4, 4), p(4, 5)], &[]).unwrap();
    assert_eq!(board.group(p(4, 4)).count(), 2);
    assert_eq!(board.liberties(p(4, 4)).count(), 6);
    for point in [p(0, 0), Point::new(u16::MAX)] {
        assert_eq!(board.group(point).count(), 0);
        assert_eq!(board.liberties(point).count(), 0);
    }
}

#[test]
fn area_map_counts_territory_and_removes_dead_stones_without_mutating_board() {
    let board = Board::from_stones(9, &[p(0, 1), p(1, 0), p(2, 1), p(1, 2)], &[p(8, 8)]).unwrap();
    let area = board.area(&[]);
    assert_eq!(area.black, 6);
    assert_eq!(area.white, 1);
    assert_eq!(
        area.territory().get(usize::from(p(1, 1).index())),
        Some(&Some(Color::Black))
    );
    assert_eq!(
        area.territory().get(usize::from(p(2, 2).index())),
        Some(&None)
    );
    let empty = board.area(&[p(0, 1), p(1, 0), p(2, 1), p(1, 2), p(8, 8)]);
    assert_eq!((empty.black, empty.white), (0, 0));
    assert_eq!(board.get(p(8, 8)), Some(Color::White));
}

#[test]
fn last_liberty_capture_is_legal_and_suicide_preserves_position() {
    let white = [p(1, 0), p(0, 1), p(2, 1), p(1, 2)];
    let mut suicide = Board::from_stones(9, &[], &white).unwrap();
    let before = suicide.clone();
    assert_eq!(suicide.play(Color::Black, p(1, 1)), Err(Illegal::Suicide));
    assert_eq!(suicide, before);
    let mut capture = Board::from_stones(9, &[p(0, 0), p(2, 0)], &white).unwrap();
    assert_eq!(
        capture.play(Color::Black, p(1, 1)).unwrap().points(),
        &[p(1, 0)]
    );
}

#[test]
fn positional_superko_survives_passes_and_restoration_for_resumed_play() {
    let board = Board::from_stones(
        9,
        &[p(0, 1), p(1, 0), p(1, 2)],
        &[p(1, 1), p(2, 0), p(3, 1), p(2, 2)],
    )
    .unwrap();
    let history = [board.hash()];
    let mut game = Game::from_position(board, Color::Black, &history, 0, 15).unwrap();
    game.play(Move::Place(p(2, 1))).unwrap();
    assert_eq!(game.play(Move::Place(p(1, 1))), Err(Illegal::Superko));
    game.play(Move::Pass).unwrap();
    game.play(Move::Pass).unwrap();
    let mut resumed =
        Game::from_position(game.board().clone(), game.to_move(), game.history(), 0, 15).unwrap();
    assert_eq!(resumed.play(Move::Place(p(1, 1))), Err(Illegal::Superko));
}
