// A join is starting: the title screen dispatched an `Action::Connect`. The
// game is about to connect with `options.player_name()`, re-read inside the
// arm below — so substituting the option here, before the body runs, is the
// whole name override. The mod's names file holds the per-server rules
// (parsed by config_loading into the mod crate); a rule for the address
// wins, a rule-less join keeps the game's own choice.
// Copyright (c) 2026 hakergeniusz. MIT.
fn offline_name_join(game: &mut crate::game::Game<'_>, action: &crate::title::Action) {
    let crate::title::Action::Connect { address, .. } = action else {
        return;
    };
    let name = offline_name::join(game.options.player_name(), address);
    game.options.player_name = Some(name);
}
