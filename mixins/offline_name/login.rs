// Each tick, on the window's game: once the pending join's player is in the
// world and the delay is up, the stored login goes out through the client's
// own chat path — exactly the line the player would have typed — and the
// pending join counts as handled. The `logins` setting holds the passwords;
// no rule or an empty one sends nothing. Server games never speak for the
// window (they also run this helper, and leave the decision alone).
// Copyright (c) 2026 hakergeniusz. MIT.
fn offline_name_login(game: &mut crate::game::Game<'_>) {
    use crate::game::offline_name_gate as gate;
    if !game.native_window() {
        return;
    }
    if !game.native_playing() {
        gate::world_tick(false);
        return;
    }
    if gate::sent() {
        return;
    }
    if gate::world_tick(true) < crate::game::offline_name::LOGIN_DELAY_TICKS {
        return;
    }
    let Some(address) = gate::pending() else {
        gate::mark_sent();
        return;
    };
    let rules = crate::game::offline_name::parse_rules(
        &game
            .native_setting(crate::game::offline_name::ID, "logins")
            .unwrap_or_default(),
    );
    if let Some(command) = crate::game::offline_name::login_command(&rules, &address) {
        game.handle_chat_input(&command);
    }
    gate::mark_sent();
}
