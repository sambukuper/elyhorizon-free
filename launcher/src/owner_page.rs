//! The Microsoft account screen (`horizon_launcher::account::launch`): signing in for online
//! play, the player's own name and skin, and the launcher's own download of Minecraft. Offline
//! play needs no account, so nothing opens it but the player: Sign in on Home, or after a
//! download that needed an account. The sign-in runs on it, code and all.
use super::*;

impl Launcher {
    /// Whether the screen covers the window: the player asked for it, and no question of consent
    /// or opening is over it. The data question comes first.
    pub(crate) fn asking_owner(&self) -> bool {
        self.owner_screen_open && !self.asking_consent() && self.opening.is_none()
    }

    /// Opens the screen and starts the sign-in on it.
    pub(crate) fn sign_in_to_play(&mut self, provider: account::AccountProvider) {
        self.owner_screen_open = true;
        self.start_sign_in(provider);
    }

    /// Closes it; it comes back with the next Sign in.
    fn close_owner_screen(&mut self) {
        if let Some(s) = self.sign_in.take() {
            s.cancel();
        }
        self.sign_in_state = None;
        self.owner_screen_open = false;
    }

    pub(crate) fn owner_screen(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let microsoft = account::client_id().is_some();
        let ely = account::ely_client_id().is_some();
        let configured = microsoft || ely;
        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(28.))
            .child(caps(tr("Account")).text_color(t.muted))
            .child(
                text(theme::HEADING)
                    .text_color(t.ink)
                    .child(format!("{}.", tr(account::NEEDS_OWNER))),
            )
            .child(text(theme::UI).text_color(t.muted).child(tr(
                "ElyHorizon plays offline with no account at all. Signing in adds online servers, \
                 your own name and skin, and ElyHorizon's own download of Minecraft from Mojang.",
            )))
            .child(text(theme::UI).text_color(t.muted).child(tr(
                "Your worlds, offline names and settings stay where they are.",
            )));
        let status = match &self.sign_in_state {
            Some(SignInState::Starting) => {
                let who = match self.sign_in.as_ref().map(|s| s.provider()) {
                    Some(account::AccountProvider::Ely) => tr("Asking Ely.by…"),
                    _ => tr("Asking Microsoft…"),
                };
                Some(div().child(who))
            }
            Some(SignInState::Code(code)) => Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(trf(
                        "Open {link} in your browser and enter this code:",
                        &[("link", code.verification_uri())],
                    ))
                    .child(
                        text(theme::HEADING)
                            .font_family(theme::MONO)
                            .text_color(t.ink)
                            .child(code.user_code().to_string()),
                    ),
            ),
            Some(SignInState::Done(Err(e))) => Some(div().text_color(t.danger).child(e.clone())),
            Some(SignInState::Done(Ok(_))) => None,
            None if !configured => Some(div().child(tr(account::NOT_CONFIGURED_ANY))),
            None => None,
        };
        body = body.children(status.map(|s| {
            plate(&t)
                .p(px(16.))
                .text_size(px(theme::UI.px))
                .text_color(t.ink_soft)
                .child(s)
        }));
        let mut answers = div()
            .flex()
            .justify_end()
            .gap(px(12.))
            .px(px(28.))
            .pb(px(28.))
            .child(
                button("owner-later", "Not now", Kind::Secondary, &t).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.close_owner_screen();
                        cx.notify();
                    },
                )),
            );
        answers = match &self.sign_in_state {
            Some(SignInState::Code(code)) => {
                let (uri, user_code) = (
                    code.verification_uri().to_string(),
                    code.user_code().to_string(),
                );
                answers
                    .child(
                        button("owner-copy", "Copy code", Kind::Secondary, &t).on_click(
                            move |_, _, cx| {
                                copy_to_clipboard(
                                    cx,
                                    gpui::ClipboardItem::new_string(user_code.clone()),
                                )
                            },
                        ),
                    )
                    .child(
                        button("owner-open", "Open link", Kind::Primary, &t)
                            .on_click(move |_, _, cx| cx.open_url(&uri)),
                    )
            }
            Some(SignInState::Starting) => answers,
            _ if !configured && account::ely_client_id().is_none() => answers,
            _ => {
                let mut answers = answers;
                if account::client_id().is_some() {
                    answers = answers.child(
                        button("owner-sign-in", "Sign in", Kind::Primary, &t).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.sign_in_to_play(account::AccountProvider::Microsoft);
                                cx.notify();
                            }),
                        ),
                    );
                }
                if account::ely_client_id().is_some() {
                    answers = answers.child(
                        button(
                            "owner-sign-in-ely",
                            "Sign in with Ely.by",
                            Kind::Secondary,
                            &t,
                        )
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                this.sign_in_to_play(account::AccountProvider::Ely);
                                cx.notify();
                            },
                        )),
                    );
                }
                answers
            }
        };
        let card = plate(&t)
            .flex()
            .flex_col()
            // A set width: text measured at it wraps where it is drawn.
            .w(px(560.))
            .max_w_full()
            .bg(t.page)
            .border_1()
            .border_color(t.rule)
            .child(body)
            .child(answers);
        div()
            .id("owner-gate")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(32.))
            .py(px(24.))
            .bg(t.page.opacity(0.92))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .occlude()
            .child(card)
    }
}
