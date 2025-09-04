// === Imports ===

module admin::admin;

// === Structs ===

public struct AdminCap has key, store {
    id: UID,
}

// === Public Functions ===
public fun only_admin_can_call_it(_: &AdminCap) {
    
}

// === Private Functions ===

fun init(ctx: &mut TxContext) {
    let admin = AdminCap { id: object::new(ctx) };

    transfer::transfer(admin, ctx.sender());
}
