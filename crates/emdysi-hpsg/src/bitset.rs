//! Fixed-width bit sets used to encode the type hierarchy.

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BitSet {
    pub(crate) words: Box<[u64]>,
}

impl BitSet {
    pub fn new(bits: usize) -> Self {
        BitSet {
            words: vec![0; bits.div_ceil(64)].into_boxed_slice(),
        }
    }

    pub fn insert(&mut self, i: usize) {
        self.words[i / 64] |= 1 << (i % 64);
    }

    pub fn contains(&self, i: usize) -> bool {
        self.words
            .get(i / 64)
            .is_some_and(|w| w & (1 << (i % 64)) != 0)
    }

    pub fn union_with(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a |= b;
        }
    }

    pub fn intersect_with(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a &= b;
        }
    }

    pub fn intersection(&self, other: &BitSet) -> BitSet {
        let mut r = self.clone();
        r.intersect_with(other);
        r
    }

    /// `self ⊆ other`
    pub fn is_subset(&self, other: &BitSet) -> bool {
        self.words
            .iter()
            .zip(other.words.iter())
            .all(|(a, b)| a & !b == 0)
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| *w == 0)
    }

    pub fn count(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(i, &w)| {
            let mut w = w;
            std::iter::from_fn(move || {
                if w == 0 {
                    return None;
                }
                let b = w.trailing_zeros() as usize;
                w &= w - 1;
                Some(i * 64 + b)
            })
        })
    }
}
