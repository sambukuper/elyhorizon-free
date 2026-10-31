<p align="center">
  <img src="docs/media/banner.webp" alt="ElyHorizon: a fork of Horizon, Minecraft: Java Edition 26.3 rewritten in Rust" width="100%">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Minecraft-Java%2026.3-5D9C3F" alt="Minecraft Java 26.3">
  <img src="https://img.shields.io/badge/Rust-stable-B7410E?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/vanilla%20parity-100%25-F0795A" alt="Vanilla parity 100%">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-2A2470" alt="Windows and Linux">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPLv3-3B2F7E" alt="GNU AGPL v3 licence"></a>
</p>

# ElyHorizon

A fork of [Horizon](https://github.com/horizon-rust/horizon): Minecraft: Java Edition 26.3 rewritten in Rust, a native game that reads the same worlds, follows the same rules and generates the same terrain as the Java game. You play it through a small launcher, on files prepared from your own copy of Minecraft. It is free, open source and unofficial.

The fork exists for one reason: **playing with friends**. Opening a singleplayer world to someone over the internet normally means port forwarding or a VPN. This fork ports the e4mc mod that removes that step, wires it into the game and the launcher, and ships builds of both. Everything else behaves as upstream's does.

> Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft. Horizon contains no Minecraft code, assets or data: you need your own legally obtained copy of Minecraft: Java Edition 26.3.

## What this fork adds

### The world goes public when you open it to LAN (the e4mc port)

A native Rust port of [e4mc](https://modrinth.com/project/qANg5Jrr), the "open a LAN server to anyone, anywhere, anytime" mod. Pause, Open to LAN, and the world publishes itself on e4mc's public relay: the chat shows an address like `mc.e4mc.link/xxxx`, and whoever holds it joins from any client. Java players with the original e4mc mod go through the same relay, so the host and the guests don't have to run the same game. No port forwarding, no VPN, no accounts.

**Play with Friends** in the pause menu carries the details: the address with a copy button, the guest count, and **Stop Publishing**. When the relay connection drops, the publisher reconnects on its own and the chat shows the fresh address; the old one dies with the session, so send the current one. That is a limit of the relay's protocol, which assigns no permanent domains.

### Join a Friend understands addresses

The launcher's Join a Friend field used to take only Horizon invite codes and quietly ignore everything else. It now accepts an e4mc address, `host:port`, IPv4 or `[IPv6]:port` as well, passes it to the game, and a bad input gets a visible reason instead of silence.

### Ely.by skins

Skins from Ely.by render, including other players' on a server.

### The launcher keeps your client build in every version (game-mod)

A client build kept in the launcher's `game-mod` folder is swapped into every version the launcher starts, new installs included; each version's own binary stays beside it as `.upstream`. This is how the ports survive a game update: the next version plays the kept build on its first launch. Empty the folder and rename the `.upstream` files back for vanilla.

## Installing this fork

Get the launcher from [this fork's releases](https://github.com/sambukuper/elyhorizon-launcher/releases) (`fork-*` tags). Each release holds the launcher for Windows (zip) and Linux (tar.gz), and the game client for the same two systems; the game client in the releases carries the e4mc port. You need your own copy of Minecraft: Java Edition 26.3, played once, so the launcher can prepare the game's files from it (the one-time preparation uses a Java 25 runtime), and a GPU with Vulkan. Step by step: [Install and first launch](https://playhorizon.dev/docs/install).

The port also installs on its own, into an existing checkout:

```bash
# Linux: e4mc from the latest release binary
curl -fsSL https://raw.githubusercontent.com/sambukuper/e4mc-minecraft-rust/main/install.sh | bash -s -- --download
```

## Playing with someone, the short version

1. Host: start the game from the launcher, open a world, pause, Open to LAN.
2. The chat shows the world's address. Send it to your friend as is.
3. Guest: paste it into the launcher's Join a Friend, or the game's Direct Connection.
4. When the host's address changes (a relay drop does that), the chat shows the new one; join again with it.

## About Horizon

Everything not described here works as upstream documents it: the [website](https://playhorizon.dev), the [documentation](https://playhorizon.dev/docs/), [upstream releases](https://github.com/horizon-rust/horizon/releases), the [roadmap](https://github.com/horizon-rust/horizon/blob/main/ROADMAP.md) and [contributing](https://github.com/horizon-rust/horizon/blob/main/CONTRIBUTING.md). The upstream README carries the project's status, the differential-testing story and the platform details.

## Java parity

### Java parity: 100%

Each subsystem scores 0 (absent), 0.25 (started, not playable), 0.5 (playable,
major vanilla features missing), 0.75 (playable, mostly complete) or 1 (complete
and verified against vanilla by differential tests). The percentage is the
average; `tests/test_parity.py` keeps it in sync with this table. Last audited
2026-10-13 against the harness results: all 281 harnesses pass on Linux, each
rerun whenever one of its inputs changed, and the Mac mini now runs every one of
them too (Rust uses Java's own float maths on each platform).

| Subsystem | Score | Main gaps |
| --- | ---: | --- |
| Window, input and game loop | 1 | None: the game loop, options, camera, auto-jump, toggle sprint/sneak, local input and the frame limiter match vanilla |
| Assets and resources | 1 | None: item models match vanilla over 196,290 stacks and the client draws every item through them; the resource pack screen matches vanilla's drawing over 3,000 sessions of clicks, scrolling and searching, its incompatibility confirmation included |
| Network protocol and multiplayer | 1 | None: joins vanilla servers; the dedicated server (`--dedicated`) matches vanilla's files, status ping, login, configuration, play, chunks, chat, recipes, advancements and command tree, a client built from vanilla's packet code joins it, Horizon joins a vanilla server, and LAN discovery matches over 462 cases (offline mode only) |
| Chunk storage and palettes | 1 | None: client and saves share paletted containers, identical to vanilla over 266 scenarios |
| Section meshing | 1 | None: 112 scenes and over a million quads identical to vanilla |
| Rendering | 1 | None: entity models, block entity renderers (407 cases, End portals, gateways and the loom's banner preview included) and their animations (296 cases, End gateway beams included), the HUD overlays (2,200 states), the recipe book (600 states), the player list, subtitles and the world border match vanilla |
| Lighting | 1 | None: saved and relit light matches vanilla, slabs and stairs included |
| World generation | 1 | None: 2,352 whole chunks over 225 structure sites, in all three dimensions, identical to vanilla |
| Region files and saves | 1 | None: saves round-trip with vanilla, and worlds from 1.8 to 26.2 upgrade as vanilla's DataFixers do, superflat worlds included; old options.txt and hotbar.nbt files are upgraded too |
| Entities and physics | 1 | None: attributes, dimensions, death loot, spawn rules, natural spawning (15,500 cases), raids and patrols (1,900 cases), spawned mobs' gear, enchantments and state (`finalizeSpawn`, 24,000 cases) and mob AI (92 scenarios: villager breeding and gossip, the warden, ghastlings and copper golems included) match vanilla |
| Interaction and inventory | 1 | None: menu clicks match over 8,000 click sequences, the loom and the crafter included; recipe book placement (6,000 cases), its buttons (1,200), the saved book (1,500) and the loom and crafter screens (1,600) match vanilla |
| Menus, HUD and chat | 1 | None: every command runs as vanilla (758 commands over 39,685 parsed inputs), scoreboards and teams match (81,000 operations), the player list matches over 872,000 draw calls with heads drawn from players' skins (authlib's textures, SkinManager's lookup, legacy skin conversion), and the chat matches focused and scrolled; the resource pack screen (selection over 4,000 cases, drawing and its incompatibility confirmation over 3,000 sessions) and the chat input with its command suggestions, usage lines, Tab completion and history (3,000 sessions on vanilla's client command tree) match vanilla |
| Audio | 1 | None: the sound engine, sound events, music, situational music, ambience and subtitles match vanilla |
| Block behaviours and ticking | 1 | None: block updates, random ticks, redstone, dispensers (6,000 cases), crafters (4,000 crafts), block entity removal (containers spill, 4,000 cases), copper golem chest sorting and the sculk family (435 scenarios: shrieker warnings and warden summoning included) match vanilla |

### Known issues

Skins served over https show the default skin unless already cached; pack icons show the unknown-pack texture in game; the creative hotbar keys (C/X) aren't wired to the hotbar loader; item tooltips lack attribute lines.


## License and credits

- Horizon's original work is MIT (kept as [LICENSE-MIT](LICENSE-MIT)). This fork is offered under the GNU AGPL v3 ([LICENSE](LICENSE)): the same freedoms, plus the requirement to share the source when a modified build serves users over a network.
- [e4mc](https://modrinth.com/project/qANg5Jrr) by vgskye and contributors (MIT): the relay protocol and the flow the port reimplements.
- The port's own repository: [e4mc-minecraft-rust](https://github.com/sambukuper/e4mc-minecraft-rust) (MIT).
- Not an official Minecraft product; not affiliated with Mojang, Microsoft or Valve.
