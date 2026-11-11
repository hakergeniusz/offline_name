# Offline Name

Join Horizon servers with a name of your choosing, per server — and, for
offline-mode servers running an auth plugin, log in automatically.

## Behavior

- **Per-server names** — when the client connects to a server (`Action::Connect`
  from the title screen: direct connect, the server list, LAN), the name the
  game presents is replaced with the rule for that address. A join without a
  matching rule keeps the game's own choice: the mod captures it once, so
  rule-less joins never inherit a previous server's name.
- **Automatic login** — after the join's player is in the world for three
  seconds (60 ticks), the stored `/login <secret>` for that address goes out
  through the client's own chat path, exactly the line the player would have
  typed. One line per join; a fresh join re-arms the wait.
- Only the window's game decides. The integrated server's game (singleplayer,
  LAN host) runs the same tick hook and stays silent.

Settings are read fresh at every join and every tick, so edits apply without
restarting the game.

## Configuration

Horizon stores mod settings as files:
`<game dir>/horizon-mods/.horizon-mod-data/offline_name/<key>` — one value per
file, edited by hand, applied live. Two keys:

- `names` — per-server display names: `host=name|host2=name2`
- `logins` — per-server login secrets: `host=secret|host2=secret2`

Hosts match the address as typed (direct connect, the server list, or the LAN
entry) exactly — `localhost:25565` and `localhost` are different hosts. Entries
split on `|`, a rule on the first `=`; blank and malformed entries are ignored.

**Passwords are stored as plain text.** That is what the original Auth Me's
auto-login needs to send `/login` unattended; keep the file private to this
machine. An empty secret sends nothing.

## Scope

- Offline names apply to joins made without an authenticated session — the
  engine lets a signed-in session's own name win over the passed one, by
  design. This mod never touches authentication itself.
- The fall-back name for rule-less joins is captured at the mod's first join
  in a session.

## Provenance

Feature ideas (joining with a chosen offline name, automatic login) come from
[Auth Me](https://github.com/axieum/authme) (MIT) — its README only; no code
was consulted or copied. This mod is a Horizon native mod implemented against
Horizon's own mod API and distributed under MIT, like the original.

## Building

The `.hnmod` is source-only: Horizon's launcher compiles it against the pinned
Horizon revision on install.

```sh
python3 tools/package.py   # writes target/offline_name-0.1.0.hnmod
```

## Development

- `python3 -I tools/test.py` — the standalone suite (format, clippy, the
  runtime's and the gate's tests).
- `python3 -I tools/horizon.py check|test|build` — staged against the pinned
  Horizon sources; the logs land in `target/`.

`compatibility.json` pins the Horizon revision and the hashes of the two
patched sources (`game.rs`, `game/native_hooks.rs`); a Horizon change there
needs the pins re-checked.
