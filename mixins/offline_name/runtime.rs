// Offline Name's rules, as pure standard library: how a `names`/`logins`
// setting value parses into per-server rules, which name a join presents,
// and when the automatic login line is due. The mod itself is thin splices
// over Horizon's own join and chat paths; this library carries the rules
// and their tests. (Plain comments, not `//!`: the mixin CLI rejects any
// module source whose top-of-file lines parse as file attributes.)

/// The mod's id: its settings folder and its settings keys' owner.
pub const ID: &str = "offline_name";

/// How many ticks the player must have been in the world before the login
/// line goes out: long enough for the server's auth plugin to have greeted
/// the join, short enough to feel instant (3 seconds at 20 tps).
pub const LOGIN_DELAY_TICKS: i32 = 60;

/// Parse one setting value into rules: `host=name|host2=name2`. Entries
/// split on the first `=` (values may contain `=`); blank and malformed
/// entries are skipped, and surrounding whitespace is trimmed everywhere.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse_into_rules() {
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
        assert_eq!(
            decide(&mut default, "Steve".to_owned(), &rules, "server-a"),
            "Alt"
        );
        assert_eq!(
            default.as_deref(),
            Some("Steve"),
            "captured before the rule applied"
        );
        assert_eq!(
            decide(&mut default, "Steve".to_owned(), &rules, "other"),
            "Steve"
        );
        assert_eq!(
            decide(&mut default, "Steve".to_owned(), &rules, "server-a"),
            "Alt"
        );
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
        assert_eq!(
            login_command(&rules, "server-a"),
            Some("/login hunter2".to_owned())
        );
        assert_eq!(
            login_command(&rules, "server-b"),
            None,
            "an empty login sends nothing"
        );
        assert_eq!(login_command(&rules, "server-c"), None);
    }
}
