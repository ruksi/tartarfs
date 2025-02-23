use std::ops::{Add, AddAssign};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Inode(pub u64);

impl Add<u64> for Inode {
    type Output = Self;

    fn add(self, other: u64) -> Self {
        Self(self.0 + other)
    }
}

impl AddAssign<u64> for Inode {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}
