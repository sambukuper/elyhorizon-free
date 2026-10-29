//! Steam sharing: the e4steam port ([horizon-steam]) behind the Play with
//! Friends screen. The host's world opens to a Steam lobby (friends join
//! freely, or only by invitation) and guests' streams travel over Steam's
//! P2P networking to the integrated server's LAN port. Without the client's
//! `steam` feature every type is here with the same shape, but nothing is
//! offered: `available` is false, `start` refuses, invites never arrive.
//!
//! [horizon-steam]: ../../../horizon_steam/index.html

/// Who may join, as the Play with Friends screen offers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Steam friends join freely (e4steam's "Steam friends").
    Friends,
    /// Joining needs an invitation the host sends in the Steam overlay.
    Invitation,
}

/// What the Play with Friends screen shows about Steam sharing.
#[derive(Clone, Debug, Default)]
#[cfg_attr(not(feature = "steam"), allow(dead_code))]
pub struct View {
    /// The world is open to Steam friends right now.
    pub sharing: bool,
    /// The world's Steam address (`steam:<lobby id>`).
    pub address: Option<String>,
    /// Why Steam sharing isn't serving, when the player asked for it.
    pub problem: Option<String>,
}

/// What a tick reports to the game.
#[derive(Debug)]
#[cfg_attr(not(feature = "steam"), allow(dead_code))]
pub enum Event {
    /// A friend's session is being carried over Steam.
    GuestJoined,
    /// The friend's session ended.
    GuestLeft,
}

#[cfg(feature = "steam")]
mod imp {
    use super::{Access, Event, View};
    use horizon_protocol::{GAME_VERSION, PROTOCOL_VERSION};
    use std::time::Instant;

    pub struct Sharing {
        share: Option<horizon_steam::share::Share>,
        copied: Option<Instant>,
    }

    impl Sharing {
        /// Whether Steam sharing can be offered (Steam is running).
        pub fn available() -> bool {
            horizon_steam::available()
        }

        /// Start sharing the world already open to LAN on `lan`.
        pub fn start(lan: std::net::SocketAddr, access: Access) -> Result<Sharing, String> {
            let share = horizon_steam::share::Share::start(horizon_steam::share::ShareConfig {
                lan,
                protocol: PROTOCOL_VERSION,
                game: GAME_VERSION.to_owned(),
                access: match access {
                    Access::Friends => horizon_steam::share::Access::Friends,
                    Access::Invitation => horizon_steam::share::Access::Invitation,
                },
            })?;
            Ok(Sharing {
                share: Some(share),
                copied: None,
            })
        }

        /// End the share: the lobby closes; sessions already playing go on.
        pub fn stop(&mut self) {
            if let Some(share) = self.share.take() {
                share.stop();
            }
        }

        /// New sessions and Steam's news, once a tick.
        pub fn tick(&mut self) -> Vec<Event> {
            let Some(share) = &self.share else {
                return Vec::new();
            };
            share
                .poll()
                .into_iter()
                .filter_map(|event| match event {
                    horizon_steam::share::ShareEvent::GuestJoined { .. } => {
                        Some(Event::GuestJoined)
                    }
                    horizon_steam::share::ShareEvent::GuestLeft { .. } => Some(Event::GuestLeft),
                    horizon_steam::share::ShareEvent::Lobby(_) => None,
                })
                .collect()
        }

        /// What the screen shows.
        pub fn view(&self) -> View {
            match &self.share {
                Some(share) => {
                    let state = share.state();
                    View {
                        sharing: true,
                        address: state.address,
                        problem: state.problem,
                    }
                }
                None => View::default(),
            }
        }

        /// The Steam address, for the copy button.
        pub fn address(&self) -> Option<String> {
            self.view().address
        }

        /// The address was just copied (the button says so for a moment).
        pub fn mark_copied(&mut self) {
            self.copied = Some(Instant::now());
        }
        pub fn just_copied(&self) -> bool {
            self.copied
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(2))
        }

        /// Open the Steam overlay's invite dialog.
        pub fn invite(&mut self) -> Result<(), String> {
            match &self.share {
                Some(share) => share.invite_friends(),
                None => Err("Steam sharing isn't on".into()),
            }
        }

        /// A Steam invite the player accepted in the overlay: the address to
        /// join, or None. Polled by the game, so a friend's invite arrives
        /// whether or not this game is sharing.
        pub fn take_invite() -> Option<String> {
            static RX: std::sync::OnceLock<
                Option<std::sync::Mutex<std::sync::mpsc::Receiver<horizon_steam::SteamEvent>>>,
            > = std::sync::OnceLock::new();
            let rx = RX
                .get_or_init(|| horizon_steam::subscribe().ok().map(std::sync::Mutex::new))
                .as_ref()?;
            loop {
                match rx.lock().unwrap().try_recv() {
                    Ok(horizon_steam::SteamEvent::JoinRequested { lobby }) => {
                        return Some(horizon_steam::lobby_address(lobby));
                    }
                    // Lobby membership changes matter to nobody here.
                    Ok(_) => {}
                    Err(_) => return None,
                }
            }
        }
    }
}

#[cfg(not(feature = "steam"))]
mod imp {
    use super::{Access, Event, View};
    use std::net::SocketAddr;

    pub struct Sharing;

    impl Sharing {
        pub fn available() -> bool {
            false
        }
        pub fn start(_lan: SocketAddr, _access: Access) -> Result<Sharing, String> {
            Err("This build doesn't have Steam sharing.".into())
        }
        pub fn stop(&mut self) {}
        pub fn tick(&mut self) -> Vec<Event> {
            Vec::new()
        }
        pub fn view(&self) -> View {
            View::default()
        }
        pub fn address(&self) -> Option<String> {
            None
        }
        pub fn mark_copied(&mut self) {}
        pub fn just_copied(&self) -> bool {
            false
        }
        pub fn invite(&mut self) -> Result<(), String> {
            Err("This build doesn't have Steam sharing.".into())
        }
        pub fn take_invite() -> Option<String> {
            None
        }
    }
}

pub use imp::Sharing;
