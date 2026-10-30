<p align="center">
  <img src="docs/media/banner.webp" alt="ElyHorizon: a fork of Horizon, Minecraft: Java Edition 26.3 rewritten in Rust" width="100%">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Minecraft-Java%2026.3-5D9C3F" alt="Minecraft Java 26.3">
  <img src="https://img.shields.io/badge/Rust-stable-B7410E?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-2A2470" alt="Windows and Linux">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPLv3-3B2F7E" alt="GNU GPL v3 licence"></a>
</p>

# ElyHorizon

A fork of [Horizon](https://github.com/horizon-rust/horizon): Minecraft: Java Edition 26.3 rewritten in Rust, a native game that reads the same worlds, follows the same rules and generates the same terrain as the Java game. You play it through a small launcher, on files prepared from your own copy of Minecraft. It is free, open source and unofficial.

The fork exists for one reason: **playing with friends**. Opening a singleplayer world to someone over the internet normally means port forwarding or a VPN. This fork ports two Minecraft mods that remove that step, wires them into the game and the launcher, and ships builds of both. Everything else behaves as upstream's does.

> Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft. Horizon contains no Minecraft code, assets or data: you need your own legally obtained copy of Minecraft: Java Edition 26.3.

## What this fork adds

### The world goes public when you open it to LAN (the e4mc port)

A native Rust port of [e4mc](https://modrinth.com/project/qANg5Jrr), the "open a LAN server to anyone, anywhere, anytime" mod. Pause, Open to LAN, and the world publishes itself on e4mc's public relay: the chat shows an address like `mc.e4mc.link/xxxx`, and whoever holds it joins from any client. Java players with the original e4mc mod go through the same relay, so the host and the guests don't have to run the same game. No port forwarding, no VPN, no accounts.

**Play with Friends** in the pause menu carries the details: the address with a copy button, the guest count, and **Stop Publishing**. When the relay connection drops, the publisher reconnects on its own and the chat shows the fresh address; the old one dies with the session, so send the current one. That is a limit of the relay's protocol, which assigns no permanent domains.

### Steam friends (the e4steam port)

A native port of [e4steam](https://github.com/Kamilhik/e4steam). With Steam running, the Steam section of Play with Friends opens the world to a Steam lobby: friends get a Join button in their Steam friends list, invited friends get Steam's own invitation and join from the overlay, and the world's `steam:<lobby id>` address works in Direct Connection. Steam's P2P networking carries the game traffic, so this needs Steam on both sides and never touches the router.

This part links the Steamworks SDK, so it is **not in the release binaries** (those keep to system libraries). Build from source with `--features steam`, or let [e4steam-rust's installer](https://github.com/sambukuper/e4steam-rust) do the whole job.

### Join a Friend understands addresses

The launcher's Join a Friend field used to take only Horizon invite codes and quietly ignore everything else. It now accepts an e4mc address, `host:port`, IPv4 or `[IPv6]:port` as well, passes it to the game, and a bad input gets a visible reason instead of silence.

### Ely.by skins

Skins from Ely.by render, including other players' on a server.

### The launcher keeps your client build in every version (game-mod)

A client build kept in the launcher's `game-mod` folder is swapped into every version the launcher starts, new installs included; each version's own binary stays beside it as `.upstream`. This is how the ports survive a game update: the next version plays the kept build on its first launch. Empty the folder and rename the `.upstream` files back for vanilla.

## Installing this fork

Get the launcher from [this fork's releases](https://github.com/sambukuper/elyhorizon-launcher/releases) (`fork-*` tags). Each release holds the launcher for Windows (zip) and Linux (tar.gz), and the game client for the same two systems; the game client in the releases carries the e4mc port. You need your own copy of Minecraft: Java Edition 26.3, played once, so the launcher can prepare the game's files from it (the one-time preparation uses a Java 25 runtime), and a GPU with Vulkan. Step by step: [Install and first launch](https://playhorizon.dev/docs/install).

The ports also install on their own, into an existing checkout:

```powershell
# Windows: e4mc and Steam sharing, built from source
irm https://raw.githubusercontent.com/sambukuper/e4steam-rust/main/install.ps1 | iex
```

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

## License and credits

- Horizon is GPLv3, and so is this fork ([LICENSE](LICENSE)).
- [e4mc](https://modrinth.com/project/qANg5Jrr) by vgskye and contributors (MIT): the relay protocol and the flow the port reimplements.
- [e4steam](https://github.com/Kamilhik/e4steam) by Kamilchik and K2 Studio (Apache-2.0): the Steam lobby scheme, the identity and the virtual port.
- The ports' own repositories: [e4mc-minecraft-rust](https://github.com/sambukuper/e4mc-minecraft-rust), [e4steam-rust](https://github.com/sambukuper/e4steam-rust).
- Not an official Minecraft product; not affiliated with Mojang, Microsoft or Valve.
