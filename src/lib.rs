//! Offline Name, the native-route native mod.
//!
//! The rules are pure standard library — how a `host=name|host2=name2`
//! rules file parses, which name a join presents, and when the automatic
//! login line is due — with standalone tests. The mod itself is thin mixins
//! over Horizon's own join and chat paths: a HEAD mixin on the title
//! screen's `title_action` substitutes the name before the connect arm
//! re-reads it, and a HEAD mixin on the client tick dispatch sends the
//! stored `/login` once the player is in the world and the delay is up.
//!
//! The rules live in config files now (`config/offline_name/names.txt` and
//! `logins.txt`, parsed by the game's config machinery into the statics
//! below); the old per-mod settings folder had no text kind to hold them.
//!
//! Clean-room: written for Horizon from the game's own public behavior; it
//! copies nothing from any mod.
//!
//! Copyright (c) 2026 hakergeniusz. MIT.

use std::sync::Mutex;

use mod_api::{Config, ConfigKind, Hooks};

/// The mod's id: its config folder's name.
pub const ID: &str = "offline_name";

/// How many ticks the player must have been in the world before the login
/// line goes out: long enough for the server's auth plugin to have greeted
/// the join, short enough to feel instant (3 seconds at 20 tps).
pub const LOGIN_DELAY_TICKS: i32 = 60;

/// The config files, under `config/offline_name/`: the per-server names and
/// the stored logins. Both are `host=name|host2=name2` pipe lists, skipped
/// entries ignored.
pub const NAMES_FILE: &str = "offline_name/names.txt";
pub const LOGINS_FILE: &str = "offline_name/logins.txt";

const RULES_HEADER: &str = "# host=name|host2=name2 — one line, | between entries.\n";

/// Parse one rules value: `host=name|host2=name2`. Entries split on the
/// first `=` (values may contain `=`); blank and malformed entries are
/// skipped, and surrounding whitespace is trimmed everywhere.
pub fn parse_rules(value: &str) -> Vec<(String, String)> {
    value
        .split('|')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let (host, rest) = entry.split_once('=')?;
            let host = host.trim();
            if host.is_empty() {
                return None;
            }
            Some((host.to_owned(), rest.trim().to_owned()))
        })
        .collect()
}

/// The rule for `address`, matched exactly.
pub fn lookup<'a>(rules: &'a [(String, String)], address: &str) -> Option<&'a str> {
    rules
        .iter()
        .find(|(host, _)| host == address)
        .map(|(_, value)| value.as_str())
}

/// The name the join presents. The first join captures the game's own choice
/// as the default, so rule-less joins keep the pre-mod name instead of
/// inheriting a previous server's rule; a matching rule wins over it.
pub fn decide(
    default: &mut Option<String>,
    current: String,
    rules: &[(String, String)],
    address: &str,
) -> String {
    let default = default.get_or_insert(current);
    lookup(rules, address)
        .map(str::to_owned)
        .unwrap_or_else(|| default.clone())
}

/// The chat line to send for `address`, if a non-empty login is stored for
/// it. The rest is the game's own command handling (`/login` included).
pub fn login_command(rules: &[(String, String)], address: &str) -> Option<String> {
    let login = lookup(rules, address)?;
    if login.is_empty() {
        return None;
    }
    Some(format!("/login {}", login))
}

// The parsed rules, kept here so the mixins read them without touching the
// game's config machinery: config_loading writes, the join and login mixins
// read.
static NAMES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static LOGINS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

pub fn names() -> Vec<(String, String)> {
    NAMES.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

pub fn logins() -> Vec<(String, String)> {
    LOGINS.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

/// A names file's text, parsed in.
pub fn set_names(text: &str) {
    *NAMES.lock().unwrap_or_else(|p| p.into_inner()) = parse_rules(text);
}

/// A logins file's text, parsed in.
pub fn set_logins(text: &str) {
    *LOGINS.lock().unwrap_or_else(|p| p.into_inner()) = parse_rules(text);
}

// The join/login shared state: the captured default name (taken from the
// game's own choice at the first join), the address of the most recent
// join, and the in-world tick counter the automatic login waits on.
static DEFAULT: Mutex<Option<String>> = Mutex::new(None);
static PENDING: Mutex<Option<String>> = Mutex::new(None);
static SENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static TICKS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// The name this join presents (the capture-once decision, applied to the
/// shared default) and remembers `address` as the pending join.
pub fn join(current: String, address: &str) -> String {
    let name = {
        let mut default = DEFAULT.lock().unwrap_or_else(|p| p.into_inner());
        decide(&mut default, current, &names(), address)
    };
    *PENDING.lock().unwrap_or_else(|p| p.into_inner()) = Some(address.to_owned());
    SENT.store(false, std::sync::atomic::Ordering::Release);
    TICKS.store(0, std::sync::atomic::Ordering::Release);
    name
}

/// The address of the most recent join, if any.
pub fn pending() -> Option<String> {
    PENDING
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

/// Whether the pending join's login line went out (or no login was due).
pub fn sent() -> bool {
    SENT.load(std::sync::atomic::Ordering::Acquire)
}

/// Mark the pending join handled.
pub fn mark_sent() {
    SENT.store(true, std::sync::atomic::Ordering::Release);
}

/// One world tick: counts while the player is in the world, resets the
/// moment they are not, and returns the count including this tick (the
/// first in-world tick is 1, so `LOGIN_DELAY_TICKS` ticks are exactly
/// `LOGIN_DELAY_TICKS` calls).
pub fn world_tick(playing: bool) -> i32 {
    use std::sync::atomic::Ordering;
    if playing {
        TICKS.fetch_add(1, Ordering::AcqRel) + 1
    } else {
        TICKS.store(0, Ordering::Release);
        0
    }
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The two rules files, written empty (with a comment header) when
    /// missing. Client: the window is where joins and logins happen.
    fn register_configs(
        &self,
        _: mod_api::HostSide<'_>,
        event: &mut mod_api::RegisterConfigsEvent<'_>,
    ) {
        event
            .configs
            .push(Config::new(ID, ConfigKind::Client, RULES_HEADER).file(NAMES_FILE));
        event
            .configs
            .push(Config::new(ID, ConfigKind::Client, RULES_HEADER).file(LOGINS_FILE));
    }

    /// A config file was read: parse it into the statics when it is ours.
    fn config_loading(&self, _: mod_api::HostSide<'_>, event: &mut mod_api::ConfigLoadingEvent<'_>) {
        absorb(&event.config.mod_id, &event.config.file, &event.config.text);
    }

    /// A no-op listener so the game always calls the `native_client_tick_pre`
    /// dispatch the login mixin lives in.
    fn client_tick_pre(&self, _: &mut dyn mod_api::ClientHost, _: &mut mod_api::ClientTickPreEvent) {
    }
}

// config_loading hears every mod's configs; the event's ConfigFile says
// whose. Kept out of the Hooks impl so the match stays testable.
pub fn absorb(mod_id: &str, file: &str, text: &str) {
    if mod_id != ID {
        return;
    }
    if file == NAMES_FILE {
        set_names(text);
    } else if file == LOGINS_FILE {
        set_logins(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_parse_into_pairs() {
        let rules = parse_rules(
            " localhost:25565 = Herobrine |play.example.com=Alt| bad | = no host | a=\"x\"",
        );
        assert_eq!(
            rules,
            vec![
                ("localhost:25565".to_owned(), "Herobrine".to_owned()),
                ("play.example.com".to_owned(), "Alt".to_owned()),
                ("a".to_owned(), "\"x\"".to_owned()),
            ]
        );
        assert!(parse_rules("").is_empty());
        assert!(parse_rules("   ").is_empty());
        assert!(parse_rules("no-equals-sign").is_empty());
    }

    #[test]
    fn lookups_match_the_address_exactly() {
        let rules = parse_rules("localhost:25565=One|localhost=Two");
        assert_eq!(lookup(&rules, "localhost:25565"), Some("One"));
        assert_eq!(lookup(&rules, "localhost"), Some("Two"));
        assert_eq!(lookup(&rules, "localhost:25566"), None);
    }

    #[test]
    fn the_first_join_captures_the_default() {
        let rules = parse_rules("server-a=Alt");
        let mut default = None;
        assert_eq!(decide(&mut default, "Steve".to_owned(), &rules, "server-a"), "Alt");
        assert_eq!(default.as_deref(), Some("Steve"), "captured before the rule applied");
        assert_eq!(decide(&mut default, "Steve".to_owned(), &rules, "other"), "Steve");
        assert_eq!(decide(&mut default, "Steve".to_owned(), &rules, "server-a"), "Alt");
        let empty = parse_rules("");
        assert_eq!(
            decide(&mut default, "Changed".to_owned(), &empty, "x"),
            "Steve",
            "a later game choice does not move the captured default"
        );
    }

    #[test]
    fn login_lines_need_a_stored_login() {
        let rules = parse_rules("server-a=hunter2|server-b=");
        assert_eq!(login_command(&rules, "server-a"), Some("/login hunter2".to_owned()));
        assert_eq!(login_command(&rules, "server-b"), None, "an empty login sends nothing");
        assert_eq!(login_command(&rules, "server-c"), None);
    }

    #[test]
    fn configs_landing_here_are_absorbed() {
        set_names("");
        absorb("someone_else", NAMES_FILE, "server-x=NotMine");
        assert!(names().is_empty(), "another mod's config is nothing to us");
        absorb(ID, NAMES_FILE, "server-x=Mine");
        assert_eq!(lookup(&names(), "server-x"), Some("Mine"));
        absorb(ID, LOGINS_FILE, "server-x=hunter2");
        assert_eq!(login_command(&logins(), "server-x"), Some("/login hunter2".to_owned()));
    }
}
