module dynamic_fields::dynamic_fields;

use sui::table::Table;
use sui::table;

public struct Dynamic has key {
    id: UID,
    values: Table<u8, Value>,
}

public struct Value has store {
    value: u8,
}

public fun new(
    ctx: &mut TxContext,
) {
    let mut values = table::new(ctx);

    table::add(&mut values, 0, Value { value: 0});
    table::add(&mut values, 1, Value { value: 1});
    table::add(&mut values, 2, Value { value: 2});
    table::add(&mut values, 3, Value { value: 3});

    let dynamic = Dynamic {
        id: object::new(ctx),
        values,
    };

    transfer::share_object(dynamic);
}
