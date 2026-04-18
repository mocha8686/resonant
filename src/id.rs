use std::{
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Default)]
pub struct IdGenerator(AtomicU64);

impl IdGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn starting_at(start: u64) -> Self {
        Self(AtomicU64::new(start))
    }

    pub fn next_id(&mut self) -> Id {
        let id = self.0.fetch_add(1, Ordering::Relaxed);
        Id::new(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u64);

impl Id {
    fn new(id: u64) -> Self {
        Self(id)
    }
}

impl Deref for Id {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
