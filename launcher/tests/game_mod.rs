//! The game-mod build on launch: a client build kept in `<data>/game-mod` replaces every installed
//! version's binary when it plays, and the version's own binary is kept once as
//! `horizon-client.upstream` (backend.rs apply_game_mod). Unix only: the stand-in game is a shell
//! script, so Windows has no partner here (the swap itself is platform-free; Linux is what CI runs).
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TAG: &str = "v2026.10.27.01";
const ORIGINAL: &str = "#!/bin/sh\necho the version's own game\n";
const MOD: &str = "#!/bin/sh\necho the game-mod build\n";

/// Fresh homes with one installed, prepared version whose game is a shell script, and a
/// game-mod folder holding a different build for the same game.
fn home(name: &str, mod_game_version: &str) -> PathBuf {
    let name = format!("horizon-game-mod-{name}-{}", std::process::id());
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    let version = root.join("launcher/versions").join(TAG);
    fs::create_dir_all(version.join(".local/horizon-client/26.3")).unwrap();
    fs::write(version.join(".installed"), "").unwrap();
    game(&version.join("horizon-client"), ORIGINAL);
    fs::write(
        version.join(".local/horizon-client/26.3/horizon-resources.json"),
        "{}",
    )
    .unwrap();
    fs::write(
        root.join("launcher/accounts.json"),
        r#"{"accounts":[],"selected":null,"offline":["Builder_1"],"offline_selected":"Builder_1"}"#,
    )
    .unwrap();
    let game_mod = root.join("launcher/game-mod");
    fs::create_dir_all(&game_mod).unwrap();
    game(&game_mod.join("horizon-client"), MOD);
    let version_file = game_mod.join("game-version.txt");
    fs::write(&version_file, format!("{mod_game_version}\n")).unwrap();
    root
}

fn game(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn play(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_elyhorizon-launcher"))
        .args(["play", "--version", TAG])
        .env("HORIZON_HOME", root.join("game"))
        .env("HORIZON_LAUNCHER_HOME", root.join("launcher"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1")
        .env("HORIZON_NO_AUDIO", "1")
        .env("HORIZON_TELEMETRY_URL", "off")
        .env("HORIZON_NO_KEYCHAIN", "1")
        .env("HORIZON_MSA_CLIENT_ID", "test-client")
        .output()
        .unwrap()
}

fn text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
fn the_game_mod_build_replaces_the_version_binary_and_the_original_keeps_once() {
    let root = home("swap", "26.3");
    let version = root.join("launcher/versions").join(TAG);
    let out = play(&root);
    // The stand-in game ran (the swap happens before it starts), so the launch itself succeeded.
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(version.join("horizon-client")).unwrap(),
        MOD
    );
    assert_eq!(
        fs::read_to_string(version.join("horizon-client.upstream")).unwrap(),
        ORIGINAL
    );
    // The launcher's own marker: the swapped build is recognised next time.
    assert!(version.join(".game-mod").exists());
    // Executable, as the version's own binary was: the launch just ran it.
    let mode = fs::metadata(version.join("horizon-client"))
        .unwrap()
        .permissions()
        .mode();
    assert_ne!(mode & 0o111, 0, "not executable");

    // A second launch changes nothing: the mod is in place, the original stays as it was.
    let again = play(&root);
    assert!(again.status.success(), "{}", text(&again));
    assert_eq!(
        fs::read_to_string(version.join("horizon-client.upstream")).unwrap(),
        ORIGINAL
    );
    assert_eq!(
        fs::read_to_string(version.join("horizon-client")).unwrap(),
        MOD
    );
}

#[test]
fn a_build_for_another_game_is_not_swapped_in() {
    let root = home("mismatch", "26.2");
    let version = root.join("launcher/versions").join(TAG);
    let out = play(&root);
    assert!(out.status.success(), "{}", text(&out));
    // The version plays its own game, and nothing was kept for later.
    assert_eq!(
        fs::read_to_string(version.join("horizon-client")).unwrap(),
        ORIGINAL
    );
    assert!(!version.join("horizon-client.upstream").exists());
    assert!(!version.join(".game-mod").exists());
}
