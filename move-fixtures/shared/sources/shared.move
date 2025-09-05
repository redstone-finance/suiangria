module shared::shared;


// For Move coding conventions, see
// https://docs.sui.io/concepts/sui-move-concepts/conventions


public struct Test has key {
    id: UID,
    value: u8,
}

public fun new(
    ctx: &mut TxContext,
) {
    let test = Test {
        id: object::new(ctx),
        value: 0,
    };

    transfer::share_object(test);
}

public fun set_value(
    test: &mut Test,
    new_value: u8,
) {
  test.value = new_value;
}
