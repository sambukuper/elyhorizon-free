//! The Minecraft identity of a Steam account, as e4steam 0.3.0 derives it
//! (`SteamMinecraftIdentity`): a stable UUID and a safe name that depend on
//! the authenticated SteamID alone, never on the name a client sends.
//!
//! The UUID is the mod's own: a version-3 UUID (Java's
//! `UUID.nameUUIDFromBytes`) of `e4steam:steam-identity:v1:<steam id>`, so a
//! world a guest's progress was saved under through the Java mod maps to the
//! same UUID here. The safe name is `s_` + the SteamID in base 36, which
//! fits Minecraft's name rules (16 letters, digits and underscores) and is
//! what the guest logs in as ([guard](crate::guard) checks it on the host).

use horizon_protocol::crypto::md5;
use steamworks::SteamId;

const UUID_NAMESPACE: &str = "e4steam:steam-identity:v1:";
const NAME_PREFIX: &str = "s_";

/// A stable, versioned Minecraft UUID for one SteamID (e4steam's
/// `SteamMinecraftIdentity.uuid`).
pub fn uuid(steam_id: u64) -> u128 {
    let material = format!("{UUID_NAMESPACE}{steam_id}");
    let mut h = md5(material.as_bytes());
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    u128::from_be_bytes(h)
}

/// The UUID as players and servers write it.
pub fn uuid_string(steam_id: u64) -> String {
    horizon_protocol::crypto::uuid_string(uuid(steam_id))
}

/// A Minecraft-compatible name derived from the SteamID (e4steam's
/// `safeName`): the Steam persona name is shown nowhere it would be trusted.
pub fn safe_name(steam_id: u64) -> String {
    format!("{NAME_PREFIX}{}", base36(steam_id))
}

/// The SteamID a safe name carries, for the host's join check.
pub fn steam_id_of(name: &str) -> Option<u64> {
    let id = name.strip_prefix(NAME_PREFIX)?;
    // A u64 in base 36 is at most 13 digits ("s_" + 13 stays within
    // Minecraft's 16); anything longer is not one of this scheme's names.
    if id.is_empty() || id.len() > 13 {
        return None;
    }
    u64::from_str_radix(id, 36).ok()
}

/// The safe name a guest over Steam must log in with.
pub fn expected_name(steam_id: SteamId) -> String {
    safe_name(steam_id.raw())
}

fn base36(mut n: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse(); // Java's Long.toString writes the high digit first
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vectors computed against the Java algorithm
    /// (`UUID.nameUUIDFromBytes("e4steam:steam-identity:v1:" + id)` and
    /// `Long.toString(id, 36)`), e4steam's `SteamMinecraftIdentity`.
    #[test]
    fn matches_the_java_mods_derivation() {
        for (steam_id, uuid, name) in [
            (
                76561197960265728,
                "6dd93370-5365-38a4-9daa-17484fc197fc",
                "s_kxuogshzmyo",
            ),
            (
                12345678901234567,
                "9cfb726f-27f4-385e-8602-fff058d4a4b6",
                "s_3dk6el9t85j",
            ),
        ] {
            assert_eq!(uuid_string(steam_id), uuid);
            assert_eq!(safe_name(steam_id), name);
            assert_eq!(steam_id_of(name), Some(steam_id));
            // The derived UUID is a v3 one, like Java's nameUUIDFromBytes.
            assert_eq!(&uuid[14..15], "3");
        }
    }

    #[test]
    fn a_safe_name_is_a_legal_minecraft_name() {
        for id in [1, 480, 76561197960265728, u64::MAX] {
            let name = safe_name(id);
            assert!(name.len() <= 16, "{name} is too long");
            assert!(
                name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{name} holds an illegal character"
            );
            assert_eq!(steam_id_of(&name), Some(id));
        }
    }

    #[test]
    fn other_names_carry_no_steam_id() {
        for name in ["", "s_", "s", "s_", "Steve", "s_zzzzzzzzzzzzzzzzzzz", "S_1"] {
            assert_eq!(steam_id_of(name), None, "{name} parsed");
        }
    }
}
