use crate::Point;

pub(crate) const MAX_POINTS: usize = 19 * 19;
const WORDS: usize = MAX_POINTS.div_ceil(u64::BITS as usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BitSet([u64; WORDS]);

impl BitSet {
    pub(crate) fn contains(self, point: Point) -> bool {
        let index = usize::from(point.index());
        let word = index / u64::BITS as usize;
        let bit = index % u64::BITS as usize;
        self.0
            .get(word)
            .is_some_and(|value| value & (1_u64 << bit) != 0)
    }

    pub(crate) fn insert(&mut self, point: Point) -> bool {
        let index = usize::from(point.index());
        let word = index / u64::BITS as usize;
        let bit = index % u64::BITS as usize;
        let Some(value) = self.0.get_mut(word) else {
            return false;
        };
        let mask = 1_u64 << bit;
        let was_new = *value & mask == 0;
        *value |= mask;
        was_new
    }

    pub(crate) fn remove(&mut self, point: Point) -> bool {
        let index = usize::from(point.index());
        let word = index / u64::BITS as usize;
        let bit = index % u64::BITS as usize;
        let Some(value) = self.0.get_mut(word) else {
            return false;
        };
        let mask = 1_u64 << bit;
        let was_present = *value & mask != 0;
        *value &= !mask;
        was_present
    }

    pub(crate) fn count(self) -> u32 {
        self.0.iter().map(|word| word.count_ones()).sum()
    }
}

pub(crate) fn points(size: u8) -> impl Iterator<Item = Point> {
    let count = u16::from(size) * u16::from(size);
    (0..count).map(Point::new)
}
