type RejectReason = String;

#[derive(Default)]
pub struct TransactionControlExtension {
    reject: Option<RejectReason>,
}

impl TransactionControlExtension {
    pub fn reject_with(&mut self, reason: RejectReason) {
        self.reject = Some(reason)
    }

    pub fn reject<T, F: FnOnce(RejectReason) -> T>(&mut self, on_reject: F) -> Option<T> {
        let reason = self.reject.take();

        reason.map(on_reject)
    }
}
