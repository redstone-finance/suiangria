use sui_types::{clock::Clock, object::Object, SUI_CLOCK_OBJECT_ID};

use crate::sandbox::storage::StorageExtension;

pub trait TimeExtension {
    fn current_timestamp_ms(&self) -> u64;
    fn set_timestamp_ms(&mut self, timestamp_ms: u64);
    fn advance_ms(&mut self, millis: u64) {
        let current = self.current_timestamp_ms();
        self.set_timestamp_ms(current + millis);
    }
}

impl TimeExtension for StorageExtension {
    fn current_timestamp_ms(&self) -> u64 {
        read_clock(self).timestamp_ms
    }

    fn set_timestamp_ms(&mut self, timestamp_ms: u64) {
        let clock = self
            .get_object(&SUI_CLOCK_OBJECT_ID)
            .expect("Clock object must exist")
            .clone();

        let mut inner = clock.into_inner();
        inner
            .data
            .try_as_move_mut()
            .expect("Clock must be Move object")
            .set_clock_timestamp_ms_unsafe(timestamp_ms);

        self.insert_object(Object::from(inner));
    }
}

fn read_clock(storage: &StorageExtension) -> Clock {
    let clock_object = storage
        .get_object(&SUI_CLOCK_OBJECT_ID)
        .expect("Clock object must exist");

    let contents = clock_object
        .data
        .try_as_move()
        .expect("Clock must be Move object")
        .contents();

    bcs::from_bytes(contents).expect("Clock deserialization must succeed")
}
