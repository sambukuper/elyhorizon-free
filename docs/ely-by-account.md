# Ely.by account sign-in and online-mode play

Ely.by is a Russian Minecraft authentication service that lets players sign in with an Ely.by account and join online-mode servers. Horizon supports signing in with Ely.by through the launcher and joins online-mode servers like vanilla 26.3 does. Playing offline needs no account at all: the game starts under an offline name, or under the name it makes up. An Ely.by account adds online play, the player's own name and skin, and the launcher's own download of the game from Mojang ([what needs an account](#what-needs-an-account)).

## What the Ely.by app registration needs (needs your accounts)

Nothing below can be done by an agent: the app registration belongs to you. A build without the client ID can't sign in with Ely.by, so its launcher plays offline only ("This build of the launcher has no Ely.by sign-in"); release builds have it.

1. **Ely.by OAuth app** (https://account.ely.by/oauth2/applications):
   - Name: `ElyHorizon` (shown to players on the Ely.by consent page).
   - Redirect URI: none needed for the device-code flow.
   - Scopes: `account_info offline_access minecraft_server_session`.
   - Copy the **Client ID** (a string).

2. **Where the ID goes**:
   - Every build: `launcher/.cargo/config.toml` holds the ID, so a plain `cargo build` bakes it in
     and the "Sign in with Ely.by" button works out of the box — the same way the upstream C++
     Launcher keeps its IDs in its `CMakeLists.txt`. The build reads it with `option_env!` at
     compile time.
   - Release builds in CI: the GitHub repository variable (not a secret: client IDs are public)
     `HORIZON_ELY_CLIENT_ID` overrides the committed value, so a release can point at another app
     registration without touching the file.
   - Any build, at run time: the environment variable `HORIZON_ELY_CLIENT_ID` overrides the built-in
     value (empty or unset = not configured).

The app registration is the project's own: another application's client ID must never be used.

## Flow (launcher)

All requests use HTTPS and JSON:

1. **Device code**: `POST https://account.ely.by/api/oauth2/v1/devicecode`
   (`client_id`, `scope=account_info offline_access minecraft_server_session`). The launcher shows the
   `user_code` and `verification_uri` (https://account.ely.by/device) with a copy button and opens the
   browser.
2. **Poll** `POST https://account.ely.by/api/oauth2/v1/token`
   (`grant_type=urn:ietf:params:oauth:grant-type:device_code`) every `interval` seconds;
   `authorization_pending` waits, `slow_down` adds 5 s, errors end the sign-in with a message.
   Result: the access token (which is a Minecraft access token).
3. **Profile**: `GET https://account.ely.by/api/mojang/services/minecraft/profile` (Bearer token):
   UUID, name, skins, capes. 404 = owns the game but has no profile yet ("Create your Minecraft
   profile on minecraft.net first").
4. **Entitlements**: `GET https://api.minecraftservices.com/entitlements/mcstore` (Bearer token):
   the account must own `game_minecraft` or `product_minecraft` (Game Pass included), else "This
   account doesn't own Minecraft Java Edition".

The access token is a standard Minecraft token and works with the official Minecraft services for
joining online-mode servers. Token renewal: the Microsoft token is renewed through its refresh token
when it has less than an hour left, before every launch that plays online; Ely.by's token is a
Minecraft token and has no renewal of ours. The entitlements check runs at every launch of an account.

## What needs an account

An account is needed for online play and for the launcher's own download of the game, and for
nothing else. Offline play never asks for one.

- **Offline play needs nothing.** An offline name, or the name the game makes up, starts the game
  with no sign-in, no keychain item and no request: the launcher hands the game `--name` and no
  session. Offline names are added in Settings → Account (or `elyhorizon-launcher account offline
  NAME`), and the top bar switches between them, the accounts and the game's own name.
- **Online play needs an owner.** Joining an online-mode server uses the session of an account
  that owns Minecraft: Java Edition. Play asks `account::launch` for it: the accounts are asked
  again, the selected one first, until one owns the game (the token renewal below if due, then
  entitlements and profile). A session that can't be renewed (no network, expired sign-in) or an
  account that doesn't own the game doesn't stop the game: it starts offline, and the window and
  the game's output say why.
- **The launcher's own download of the game needs an owner** ([below](#downloading-the-game-for-an-owner)):
  it is the one thing nothing on the machine can stand in for.
- **The window asks when the player wants it.** Sign in is on Home (a row while no account here
  owns the game) and in Settings; the account screen and its device code open only from there.
  Nothing covers the window on its own (`launcher/src/owner_page.rs`).
- **The answer lives with the tokens.** Each account's keychain item keeps the last answer
  (`owns`), so no plain file can say yes. With no network (or a sign-in that can't be renewed) the
  last answer stands, so being offline never loses an owner who passed before.
- **Who isn't affected:** the game binary itself (`horizon-client`, `horizon-ctl launch`, the
  harnesses, CI), the dedicated server and `horizon-server`, the website. The launcher's tests use
  accounts in a memory store (`MemoryStore`, and `HORIZON_NO_KEYCHAIN` for the CLI and MCP tests,
  which then have no owner — which no longer stops a game).

## Downloading the game for an owner

Decided 2026-10-27. This is the only exception to "Horizon never downloads the game", and it
applies only on the player's machine, for an account that owns the game (checked just before). The
launcher's "Download and prepare" (`elyhorizon-launcher prepare --download`) fetches the pinned
version straight from Mojang (`launcher/src/mojang.rs`): the version file, the client jar,
libraries, the asset index and the assets, each checked against the size and sha1 Mojang lists.
They go to `minecraft/` in the launcher's data folder, and the preparation runs as usual. The
download can be cancelled and goes on where it stopped. CI, agents, harnesses and the repo never
download or keep Mojang's files.

## Token storage

- Secrets (the Minecraft access token and its expiry, the last ownership answer) go to the OS
  keychain, one item per account: Keychain on macOS, Credential Manager on Windows, the Secret
  Service (GNOME Keyring, KWallet) on Linux. Service name `dev.elyhorizon.launcher`, account = the
  Minecraft UUID. Credential Manager takes at most 2560 bytes per item, which the secrets of a sign-in
  exceed: on macOS and Windows a longer secret is split over the items `<uuid>`, `<uuid>.1`, `<uuid>.2`...
- `accounts.json` in the launcher's data folder keeps only public data: UUID, name, skin URL,
  provider (Microsoft or Ely.by), which account is selected. Never a token.
- No keychain available (a Linux box without a Secret Service, a locked keychain): sign-in still
  works for the session but nothing is saved, and the launcher says "No keychain available: you will
  have to sign in again next time" (it never falls back to a plain file).
- Tokens never appear in logs, errors, `elyhorizon-launcher` CLI output, MCP results or crash reports.
  Requests are logged by endpoint name only.

## Handing the session to the game

Vanilla's launcher passes `--accessToken <token>` on the command line, where every user of the
machine can read it in the process list. Horizon keeps it out of argv:

- The launcher writes `session-<random>.json` (mode 0600 on Unix, created new, in the game home's
  `sessions/` folder) with `{"name", "uuid", "access_token", "user_type": "msa", "xuid"}` and starts
  the game with `--session-file PATH`.
- The game reads the file at start and deletes it at once; the token lives only in the game's memory.
  A file older than two minutes is refused (and deleted), so a stale file can't be replayed.
- Without a session file the game is offline as today (`--name`, offline UUID).
- `HORIZON_SESSION_FILE` is the same as `--session-file` (development and harnesses).

## Online-mode login (game)

Vanilla's `ClientHandshakePacketListenerImpl.handleHello`, `Crypt` and authlib's
`MinecraftServicesSessionService.joinServer`:

1. `ClientboundHelloPacket` (server id, X.509 RSA public key, challenge, should_authenticate).
2. Generate a random 16-byte AES key (the shared secret).
3. Server hash: SHA-1 over `serverId` (ISO-8859-1), the shared secret, the encoded public key;
   `new BigInteger(digest).toString(16)` (signed two's complement hex, e.g. "Notch" →
   `4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48`, "jeb_" → `-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1`).
4. If `should_authenticate`: `POST` the session service's `join` endpoint
   `{"accessToken", "selectedProfile" (undashed UUID), "serverId": hash}`, the URL taken from the
   services discovery document (`https://discovery.minecraftservices.com/minecraft/client`, fetched
   once per game). Errors map like authlib's `MinecraftClientHttpException`:
   `ForbiddenOperationException` or 401 → "Failed to log in: Invalid session (Try restarting your
   game and the launcher)", `InsufficientPrivilegesException` → "Multiplayer is disabled…",
   `multiplayer.access.banned` / `FORCED_USERNAME_CHANGE` → "You are banned from playing online",
   5xx or no network → "The authentication servers are currently not reachable…". Without a session
   the game says "This server is in online mode. Sign in with your Microsoft account in the Horizon
   launcher to join it." (Vanilla goes on after a failed join on LAN-discovered servers, which then
   refuse the player anyway; Horizon stops at once.)
5. Send `ServerboundKeyPacket` (the secret and the challenge, each RSA PKCS#1 v1.5 encrypted with
   the server key), then AES/CFB8 (key = IV = the secret) both ways.

The connecting screen follows vanilla's states: "Connecting to the server..." until the encryption
request, "Logging in..." while joining the session service, "Encrypting...", then "Joining world...".
Code: `horizon-protocol` (`rsa.rs`, `crypto::server_hash`), `horizon-client` (`auth.rs`, `net.rs`).

Horizon's own dedicated and LAN servers stay offline-mode (they don't verify players with the
session service); that is separate work.

`HORIZON_API_DISCOVERY` overrides the discovery URL, the counterpart of authlib's
`-Dminecraft.api.discovery.host`; it is how the harness points both the vanilla server and Horizon at
a local mock.

## Skins and capes

Profile properties (`textures`, base64 JSON) arrive in `ClientboundPlayerInfoUpdatePacket` for every
player and in the login profile for ourselves. Like authlib's `unpackTextures`, only URLs under the
discovery document's `profiles/getTexture` `validUris` (textures.minecraft.net) are fetched; skins
(classic/slim from the `metadata.model`), capes and elytra textures are downloaded over HTTPS on a
worker thread, cached on disk by texture hash (`assets/skins/xx/hash`, like vanilla's
`SkinManager`), converted from legacy 64×32 the way `SkinTextureDownloader` does and given to the
player renderer. Until a skin arrives the default skin for the UUID shows (as today).

## Chat signing

26.3's dedicated servers default to `enforce-secure-profile=true`. A client without a chat session
can join and play, but every chat message and signed command argument is refused with "Chat
disabled due to missing profile public key. Please try reconnecting." (`SignedMessageChain.Decoder.unsigned`).
To chat on such servers the game will:

- fetch the profile key pair from the services `player/getCertificates` endpoint (Minecraft token);
- send `ServerboundChatSessionUpdatePacket` (random session UUID, key expiry, public key, Mojang's
  key signature) after joining;
- sign each chat message and signable command argument with SHA256withRSA over vanilla's
  `SignedMessageBody` layout and keep the last-seen message tracker and acknowledgements.

Until chat signing lands, Horizon joins online servers but chat works only where
`enforce-secure-profile=false` (or the server can't validate keys).

## Harnesses

`tools/test_online_mode_parity.py` (dev only, needs no account): a local mock of the Minecraft
services (discovery document, `join`, `hasJoined`/`verify`, no public keys) and a vanilla 26.3
dedicated server in online mode started with `-Dminecraft.api.discovery.host=<mock>` (authlib
10.0.77's only override: it replaces the whole discovery URL; the old `minecraft.api.session.host`
style properties are gone). Horizon's
headless client (`HORIZON_NET_REPORT`) joins with a session file holding a mock token: the server
verifies the join through the mock with the hash it computed itself (so the RSA wrapping, the hash
and AES/CFB8 are all checked against vanilla's code), the player spawns, chats and runs `/list`.
Negative cases: a token the mock rejects (the client must show vanilla's "Invalid session" message)
and a client that skips the join (the server must say "Failed to verify username!").

Rust unit tests: server hash vectors (Notch, jeb_, simon), RSA wrapping round trip with a test key,
X.509 key parsing, AES/CFB8 vectors (already in `horizon-protocol`), the session file (permissions,
age, deletion), the error mapping. Launcher: the whole token chain against a local mock of the
Ely.by and Minecraft endpoints, and the keychain abstraction with an in-memory backend.

## CLI

- `elyhorizon-launcher account ely-sign-in` — start an Ely.by device-code sign-in (prints the link
  and code, polls in the background).
- `elyhorizon-launcher account list` — shows all signed-in accounts (Microsoft and Ely.by), their
  UUIDs, which one plays, and whether sign-in is configured for each provider.
- `elyhorizon-launcher account sign-in` — Microsoft device-code sign-in (existing).

## MCP tools

- `account_list` — lists all signed-in accounts (Microsoft and Ely.by), which one plays, whether
  sign-in is configured for each provider, and the status of any sign-in in progress.
- `account_sign_in` — starts a Microsoft sign-in.
- `account_ely_sign_in` — starts an Ely.by sign-in.