// Each client tick, on the window's game: once the pending join's player is
// in the world and the delay is up, the stored login goes out through the
// client's own chat path — exactly the line the player would have typed —
// and the pending join counts as handled. The logins file holds the
// passwords; no rule or an empty one sends nothing. Server games never
// speak for the window (they also run this helper, and leave the decision
// alone).
// Copyright (c) 2026 hakergeniusz. MIT.
fn offline_name_login(game: &mut crate::game::Game<'_>) {
    if !game.playing() {
        offline_name::world_tick(false);
        return;
    }
    if offline_name::sent() {
        return;
    }
    if offline_name::world_tick(true) < offline_name::LOGIN_DELAY_TICKS {
        return;
    }
    let Some(address) = offline_name::pending() else {
        offline_name::mark_sent();
        return;
    };
    if let Some(command) = offline_name::login_command(&offline_name::logins(), &address) {
        game.handle_chat_input(&command);
    }
    offline_name::mark_sent();
}
