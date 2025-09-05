use napi_derive::napi;

use crate::SharedState;

#[napi]
pub struct ClockApi {
    sandbox: SharedState,
}

#[napi]
impl ClockApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn get_time_ms(&self) -> i64 {
        self.sandbox.borrow().clock().get_time() as i64
    }

    #[napi]
    pub fn set_time_ms(&self, timestamp_ms: i64) {
        self.sandbox
            .borrow_mut()
            .clock_mut()
            .set_time(timestamp_ms as u64);
    }

    #[napi]
    pub fn advance_by_millis(&self, millis: i64) {
        self.sandbox.borrow_mut().clock_mut().advance(millis as u64);
    }
}
