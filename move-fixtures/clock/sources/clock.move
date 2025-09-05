module clock::clock;


use sui::clock::Clock;

public struct IncreasingTimestamp has key {
    id: UID,
    value: u64,
}

public fun new(
    ctx: &mut TxContext,
) {
    let timestamp = IncreasingTimestamp {
        id: object::new(ctx),
        value: 0,
    };

    transfer::share_object(timestamp);
}

public fun update(
    timestamp: &mut IncreasingTimestamp,
    clock: &Clock,
) {
  if (timestamp.value >= clock.timestamp_ms()) {
    abort 0
  };

  timestamp.value = clock.timestamp_ms();
}
