# Publish the world to anyone (e4mc)

A Horizon port of [e4mc](https://github.com/vgskye/e4mc-minecraft-architectury),
the "Open a LAN server to anyone, anywhere, anytime" mod: the open world is
published through e4mc's public relay and gets a public address that creates
itself. Anyone who has it joins, even from vanilla Minecraft — no Steam, no
port forwarding, no invitation. The implementation lives in
`crates/horizon-e4mc`; the client's part is `game/e4mc_friends.rs` and
`game/friends.rs`.

This is the sibling of the Steam sharing
([steam-friends.md](steam-friends.md), the e4steam port): Steam sharing is
friends-only with per-Steam-account identity, e4mc publishing is public with
no identity at all. Both sit on the same Play with Friends screen and both
carry the integrated server's stream to the world's LAN port unchanged.

## How it works

The host connects out to a [quiclime](https://github.com/vgskye/e4mc-quiclime)
relay over QUIC (ALPN `quiclime`, the broker's least loaded one), asks it for
a domain assignment, and the relay answers with the world's address, like
`oink-unwell.eu.e4mc.link`. Each guest who joins the domain arrives at the
relay as a TCP connection, is identified by the Minecraft handshake's virtual
host, and becomes one bidirectional QUIC stream on the host's session — the
relay forwards the handshake as the first bytes. The host bridges every
stream to the integrated server's loopback port byte for byte; guests are
plain LAN guests from there. Steam encrypts its transport; here the relay
sees the game's plaintext (e4mc's own design: the address is public, the
mod trusts nothing about the joiners, and vanilla's ban list works as
usual — the mod patches bans to stick to the relay's view of the address,
which this port does not need: the integrated server has its own handling).

Stopping the publish closes the session: the domain stops answering, and
guests still playing are dropped with it. The domain lives as long as the
publish does.

## Playing

Open a singleplayer world, pause, **Play with Friends**. The **Anyone
(e4mc)** section: **Publish to Anyone** opens the world to LAN if it isn't
(Open to LAN's defaults), reaches the relay and shows the green address —
copyable, and joinable as a plain server address from any client. The chat
announces it too. **Stop Publishing** takes the world back down.

The address is public by design: anyone who finds it joins. For friends-only
play use the invite code or the Steam section instead.

## Testing

`cargo test -p horizon-e4mc` covers the control codec (frames in pieces,
unknown kinds, oversized frames, multi-byte lengths) and the facade. One
`--ignored` test runs the whole chain live against e4mc's public broker and
relay: a domain is assigned, a guest joins it with a real Minecraft
handshake, bytes flow both ways, and stopping disconnects the guest. Run it
by hand, it needs the network and e4mc's public service:

```sh
cargo test -p horizon-e4mc -- --ignored --nocapture
```

`E4MC_RELAY=host:port` pins a self-hosted quiclime relay instead of the
broker (the mod's `relayHost`/`relayPort`).
