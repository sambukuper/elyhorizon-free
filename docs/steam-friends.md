# Play with Steam friends

A Horizon port of [e4steam](https://github.com/Kamilhik/e4steam), the mod that
opens a Minecraft singleplayer world to Steam friends without port forwarding
or a public IP. Java mod jars do not run on Horizon, so what the mod does is
implemented natively: the host's world opens to a **Steam lobby**, friends
join from a Steam invitation or the world's address, and the game's stream
travels over **Steam's P2P networking** — direct when it can, through Valve's
relays when not — between the two ends of the integrated server, exactly the
shape Play with Friends ([play-with-friends.md](play-with-friends.md)) carries
its invite-code sessions. The implementation lives in `crates/horizon-steam`;
the client's part is `game/steam_friends.rs` and `game/friends.rs`.

Like the mod, Horizon permanently uses **Valve's shared test App ID 480**
(Spacewar), so no Steam app of its own is needed — and foreign App ID 480
traffic is filtered: a lobby without Horizon's metadata is refused on join,
and a P2P connection from an account with no seat in the lobby is refused on
connect.

## What carries over from the mod

| The mod's behaviour | The port |
| --- | --- |
| App ID 480, filtering unrelated traffic | Same App ID; lobby metadata (`horizon_protocol`, `horizon_game`, `horizon_host`) gates joins, lobby membership gates connections |
| "Open to LAN → Steam friends" / "Invitation only" | Play with Friends' Steam section: **Open to Steam Friends** (a friends lobby) and **Invitation Only** (a private lobby; invitations go through the Steam overlay's invite dialog) |
| "Invite friends" opens the Steam overlay | The same, through the SDK's invite-dialog entry point |
| Stable identity per Steam account (0.3.0+) | The mod's own scheme: name `s_<steamid in base 36>`, UUID v3 of `e4steam:steam-identity:v1:<steamid>` — so a guest's progress follows their Steam account, and worlds first played through the Java mod map to the same UUIDs |
| The host never trusts the connecting client's name | The host's tunnel reads the guest's first packets and refuses a login name that isn't the connecting Steam account's |
| One TCP stream per guest over Steam P2P | One Steam networking-sockets connection per guest, pumped to the integrated server's loopback port (virtual port 480, as the mod's dedicated transport) |
| Dedicated servers (`d-...steam`, anonymous GameServer sign-in) | **Not ported** — a follow-up; Horizon's servers have their own hosting story |
| UDP voice-chat bridge (Java voice mods) | **Not ported** — Horizon has no Java voice mods |

Cross-play with the Java mod is impossible in both directions (Horizon runs
none of its Java code), so the shared scheme above is about worlds and
identity, not interconnection.

## Playing

**Host.** Open a singleplayer world, pause, **Play with Friends** (the Friends
button). The Steam section (there when Steam is signed in) offers **Open to
Steam Friends** and **Invitation Only**. Starting it opens the world to LAN if
it isn't (with Open to LAN's defaults), creates the lobby, and shows the
world's Steam address — `steam:<lobby id>`, copyable, and pasteable into
Direct Connection (a bare lobby id pasted there works too). **Invite
Friends...** opens the Steam overlay's invite dialog; invited friends get
Steam's own invitation, and accepting it while Horizon runs joins from the
title screen. **Stop** closes the lobby; sessions already playing go on until
the world closes, as with the invite code.

**Guest.** Accept the Steam invitation (Horizon joins from the title screen),
or paste the host's `steam:<id>` address — in Direct Connection, or after the
title screen's Friends button in Join a Friend. The guest joins the lobby,
checks it is a Horizon world of the same game version (the mismatch message
names both versions, as the relay's does), connects over Steam's P2P network,
and logs in under the name of their own Steam account. Guests over Steam are
LAN guests to the world: 8 players at once, the same as Open to LAN.

The original mod shared as soon as the world opened to LAN, and the port
does the same: a plain **Open to LAN** starts a friends-lobby share on its
own, and the chat says so. The Options screen's Mods page lists the port as
built in.

No router changes, no relay, no VPN: Steam carries the traffic, as it does for
every Steam game's multiplayer. Both players need Steam running and signed in;
the usual Steam troubleshooting (`SteamAPI_Init failed`) applies, as
[e4steam's guide](https://github.com/Kamilhik/e4steam/blob/main/docs/STEAM_TROUBLESHOOTING.md)
describes it.

## Building and shipping

The code is always compiled (`cargo test` covers it); linking Steam happens
only with the client's `steam` feature:

```sh
cargo build --release -p horizon-client --features steam
```

The feature links the Steamworks SDK redistributable the `steamworks` crate
bundles, so the game exe needs the library beside it at run time. For a dev
build, copy it from the crate's own package (the same files e4steam ships
inside its jars):

- Windows: `steam_api64.dll` from
  `%CARGO_HOME%\registry\src\*\steamworks-sys-0.13.0\lib\steam\redistributable_bin\win64\`
  beside `target\release\horizon-client.exe`.

A build **without** the feature (dev builds, tests, the server build) needs
nothing: the Steam section simply isn't offered, `steam:` addresses report
"This build doesn't have Steam sharing.", and the game plays exactly as
vanilla 26.3.

**Releases are not wired up yet, on purpose.** Horizon's release binaries must
link only system libraries (`tools/check_imports.py`, enforced in the release
workflow), and a statically imported `steam_api` would fail that check — the
mod itself loads the same library dynamically for the same reason. Turning the
feature on in releases takes one of:

1. ship the redistributable beside the game and teach `check_imports.py` that
   this one is shipped with the folder (the smallest change, but the check's
   "no non-system DLL" rule exists for a reason);
2. load the library at run time with `libloading` and hand-write the flat C
   bindings and callback dispatch (e4steam's own approach over JNA — no link
   at all, the most work);
3. link Steam into a small companion library of our own ABI that the game
   loads at run time (upstream-tested FFI stays, one extra shipped file).

Until one of these lands, release builds ship without Steam sharing and the
feature is a developer- and fork-facing one.

## Testing

`cargo test -p horizon-steam` covers everything that doesn't need Steam: the
identity derivation (vectors computed against the Java algorithm, name and
UUID), the safe-name round trip and its limits, the address parsing, and the
host's opening check (a real handshake and Hello in pieces, wrong names, a
status ping, garbage). The lobby, the P2P connection and the overlay need a
signed-in Steam client and two accounts; manual coverage so far is the
single-machine loop (`--baseline`-style) and none of the two-account matrix —
the same state e4steam's own manual matrix started from, and recorded in its
[COMPATIBILITY.md](https://github.com/Kamilhik/e4steam/blob/main/COMPATIBILITY.md).
