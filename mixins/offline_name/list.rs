// The mod's row in Options > Mods: the mod's identity and what it does.
// Copyright (c) 2026 hakergeniusz. MIT.
fn offline_name_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "offline_name".into(),
        title: "Offline Name 0.1.1".into(),
        description: "Per-server offline names with automatic login on AuthMe-style servers.".into(),
        keys: vec![],
    });
}
