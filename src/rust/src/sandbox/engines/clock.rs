use std::ops::{Deref, DerefMut};

use crate::sandbox::{extensions::time_extension::TimeExtension, StorageExtension};

pub struct ClockEngine<S> {
    storage: S,
}

impl<S> ClockEngine<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S> ClockEngine<S>
where
    S: Deref<Target = StorageExtension>,
{
    pub fn get_time(&self) -> u64 {
        self.storage.current_timestamp_ms()
    }
}

impl<S> ClockEngine<S>
where
    S: DerefMut<Target = StorageExtension>,
{
    pub fn advance(&mut self, millis: u64) {
        self.storage.advance_ms(millis);
    }

    pub fn set_time(&mut self, timestamp_ms: u64) {
        self.storage.set_timestamp_ms(timestamp_ms);
    }

    pub fn reset(&mut self) {
        self.storage.set_timestamp_ms(0);
    }
}
