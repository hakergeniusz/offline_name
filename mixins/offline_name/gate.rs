// Offline Name's shared state: the captured default name (taken from the
// game's own choice at the first join), the address of the most recent join,
// and the in-world tick counter the automatic login waits on. The join hook
// (game.rs) writes the first two; the tick hook (native_hooks.rs) counts and
// reads. The rules themselves, with tests, live in the mod's `runtime`
// library; these staged tests prove the shared state's policy.
//
// Copyright (c) 2026 hakergeniusz. MIT.
use crate::game::offline_name as runtime;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

/// The name the game would present itself, captured at the first join so
/// rule-less joins keep it (see [`runtime::decide`]).
static DEFAULT: Mutex<Option<String>> = Mutex::new(None);
/// The address of the most recent join; the login sender matches it.
static PENDING: Mutex<Option<String>> = Mutex::new(None);
/// The automatic login for the pending address went out already.
static SENT: AtomicBool = AtomicBool::new(false);
/// In-world ticks since the pending join's player appeared.
static TICKS: AtomicI32 = AtomicI32::new(0);

fn lock<T>(cell: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    cell.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The name this join presents (the runtime's capture-once decision, applied
/// to the mod's shared default) and remembers `address` as the pending join.
pub fn join(current: String, rules: &[(String, String)], address: &str) -> String {
    let name = {
        let mut default = lock(&DEFAULT);
        runtime::decide(&mut default, current, rules, address)
    };
    *lock(&PENDING) = Some(address.to_owned());
    SENT.store(false, Ordering::Release);
    TICKS.store(0, Ordering::Release);
    name
}

/// The address of the most recent join, if any.
pub fn pending() -> Option<String> {
    lock(&PENDING).clone()
}

/// Whether the pending join's login line went out (or no login was due).
pub fn sent() -> bool {
    SENT.load(Ordering::Acquire)
}

/// Mark the pending join handled.
pub fn mark_sent() {
    SENT.store(true, Ordering::Release);
}

/// One world tick: counts while the player is in the world, resets the
/// moment they are not, and returns the count including this tick (the
/// first in-world tick is 1, so `LOGIN_DELAY_TICKS` ticks are exactly
/// `LOGIN_DELAY_TICKS` calls).
pub fn world_tick(playing: bool) -> i32 {
    if playing {
        TICKS.fetch_add(1, Ordering::AcqRel) + 1
    } else {
        TICKS.store(0, Ordering::Release);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{join, mark_sent, pending, sent, world_tick, DEFAULT, PENDING, SENT, TICKS};
    use crate::game::offline_name::parse_rules;

    fn clear() {
        *DEFAULT.lock().unwrap() = None;
        *PENDING.lock().unwrap() = None;
        SENT.store(false, std::sync::atomic::Ordering::Release);
        TICKS.store(0, std::sync::atomic::Ordering::Release);
    }

    /// The tests share the module's statics: one join's decision must not
    /// leak into the next test. (The suite runs with `--test-threads=1`.)
    struct Clean(());
    impl Clean {
        fn on() -> Self {
            clear();
            Self(())
        }
    }
    impl Drop for Clean {
        fn drop(&mut self) {
            clear();
        }
    }

    #[test]
    fn a_join_captures_the_default_and_becomes_pending() {
        let _clean = Clean::on();
        let rules = parse_rules("server-a=Alt");
        assert_eq!(join("Steve".to_owned(), &rules, "server-a"), "Alt");
        assert_eq!(pending().as_deref(), Some("server-a"));
        assert!(!sent(), "a fresh join is not handled yet");
        assert_eq!(world_tick(true), 1, "world ticks count from the join");
    }

    #[test]
    fn a_ruleless_join_keeps_the_captured_default_and_clears_nothing() {
        let _clean = Clean::on();
        let rules = parse_rules("server-a=Alt");
        join("Steve".to_owned(), &rules, "server-a");
        assert_eq!(join("Steve".to_owned(), &rules, "other"), "Steve");
        assert_eq!(pending().as_deref(), Some("other"), "the newest join is pending");
    }

    #[test]
    fn out_of_the_world_resets_the_wait_and_marking_ends_it() {
        let _clean = Clean::on();
        let rules = parse_rules("");
        join("Steve".to_owned(), &rules, "server-a");
        assert_eq!(world_tick(true), 1);
        assert_eq!(world_tick(false), 0, "leaving the world resets the wait");
        assert_eq!(world_tick(true), 1);
        mark_sent();
        assert!(sent(), "marking ends the wait");
    }

    #[test]
    fn a_fresh_join_rearms_the_wait() {
        let _clean = Clean::on();
        let rules = parse_rules("");
        join("Steve".to_owned(), &rules, "server-a");
        mark_sent();
        join("Steve".to_owned(), &rules, "server-b");
        assert!(!sent(), "the new join needs its own login decision");
        assert_eq!(world_tick(true), 1, "the tick counter restarted");
        assert_eq!(TICKS.load(std::sync::atomic::Ordering::Acquire), 1);
    }
}
