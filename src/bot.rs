//! The connected bot: the game plus a normal Mumble connection. The bot is a plain client with its
//! own certificate, so an unmodified murmur needs no plugin, no config change and no protocol
//! extension.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use mumcord_protocol::{
    CertVerifier, ConnectConfig, DisconnectReason, Event, Handle, ServerCert, TextTarget, User,
    connect, permissions, proto,
};
use tokio::sync::mpsc;
use tokio::time::{Instant, interval};

use crate::cards::Renderer;
use crate::cert;
use crate::config::{Config, Token};
use crate::html;
use crate::pins::Pins;
use crate::question::{self, Bank};
use crate::quiz::{Action, Now, Quiz};
use crate::rng::Rng;
use crate::scores::{Scores, SqliteScores};
use crate::strings::Strings;

/// Commands the runner takes from the process (SIGHUP, Ctrl-C) or from a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Read the question bank again.
    Reload,
    /// Send what is queued, then close.
    Stop,
}

/// Login timeout the upstream bot used for its IRC handshake.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10);

/// Reconnect wait after `attempt` failed connections: 1 s, doubling up to a minute. The upstream
/// bot reconnected at once, which hammers a server that is down.
pub fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(
        1_u64
            .checked_shl(attempt.saturating_sub(1))
            .unwrap_or(60)
            .min(60),
    )
}

/// The wait before the next connection. A kick means another instance uses the same nick or
/// certificate, or an admin removed the bot: waiting the full minute instead of doubling keeps two
/// bots from kicking each other, and gives the server's autoban time to stop counting the logins.
pub fn reconnect_delay(attempt: u32, reason: &DisconnectReason) -> Duration {
    if matches!(reason, DisconnectReason::Kicked { .. }) {
        return Duration::from_secs(60);
    }
    backoff(attempt)
}

/// Whether the bot may write in the channel, from the server's effective permissions for it.
pub fn may_write(effective: &BTreeMap<u32, u32>, channel: u32) -> bool {
    effective.get(&channel).copied().unwrap_or(0) & permissions::WRITE != 0
}

pub struct Bot {
    cfg: Config,
    quiz: Quiz,
    render: Renderer,
    start: Instant,
    /// Session to name, so a rename can move the score instead of starting a new player.
    names: BTreeMap<u32, String>,
    /// The channel the bot plays in: the one it joins at login, or the configured one.
    game: Option<u32>,
}

impl Bot {
    pub fn new(cfg: Config) -> Result<Bot> {
        cfg.validate()?;
        let strings = Strings::with_overrides(&cfg.strings)?;
        let bank = Bank::load(&cfg.quiz.questions)?;
        if bank.len() < question::MIN_QUESTIONS {
            tracing::warn!(
                "question bank has {} questions; {} or more make the recycling sensible",
                bank.len(),
                question::MIN_QUESTIONS
            );
        }
        let scores: Box<dyn Scores> = Box::new(SqliteScores::open(&cfg.quiz.hiscores)?);
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let quiz = Quiz::new(&cfg, &strings, bank, scores, Rng::seeded(seed));
        let render = Renderer::new(cfg.quiz.cards);
        Ok(Bot {
            cfg,
            quiz,
            render,
            start: Instant::now(),
            names: BTreeMap::new(),
            game: None,
        })
    }

    /// Keeps the bot connected: connect, play, and after a lost connection wait with a growing
    /// delay until `Command::Stop` arrives.
    ///
    /// Every loss is logged with the number of the connection and the reason, so `Kicked` (another
    /// instance with the same nick, or an admin), `Closed`, `Timeout` and a network error can be
    /// told apart.
    pub async fn run(&mut self, commands: &mut mpsc::Receiver<Command>) -> Result<()> {
        let mut connection = 0;
        loop {
            let outcome = self.connect_once(commands).await;
            connection += 1;
            let reason = match outcome {
                Ok(None) => return Ok(()),
                Ok(Some(reason)) => reason,
                Err(error) => DisconnectReason::Error(error.to_string()),
            };
            let wait = reconnect_delay(connection, &reason);
            tracing::info!("connection {connection} ended: {reason:?}; reconnecting in {wait:?}");
            if matches!(commands.try_recv(), Ok(Command::Stop)) {
                return Ok(());
            }
            tokio::time::sleep(wait).await;
        }
    }

    fn now(&self) -> Now {
        self.start.elapsed().as_secs()
    }

    /// One connection. `None` when the process should stop, the reason when the connection ended.
    async fn connect_once(
        &mut self,
        commands: &mut mpsc::Receiver<Command>,
    ) -> Result<Option<DisconnectReason>> {
        let data_dir = self
            .cfg
            .server
            .data_dir
            .clone()
            .unwrap_or_else(cert::default_dir);
        let cert_name = self
            .cfg
            .server
            .cert_name
            .clone()
            .unwrap_or_else(|| sanitize(&self.cfg.server.name));
        let identity = if self.cfg.server.superuser {
            None
        } else {
            Some(cert::load_or_create(&data_dir, &cert_name)?)
        };
        let server = format!("{}:{}", self.cfg.server.host, self.cfg.server.port);
        if !self.cfg.server.pin {
            tracing::warn!(
                "server.pin = false: any server certificate is accepted (local test servers only)"
            );
        }
        let pins = Pins::new(data_dir);
        let verify: CertVerifier = if self.cfg.server.pin {
            Arc::new(move |cert: &ServerCert| match pins.check(&server, cert) {
                Ok(trusted) => trusted,
                Err(error) => {
                    tracing::error!("{error}");
                    false
                }
            })
        } else {
            Arc::new(|_| true)
        };
        let config = ConnectConfig {
            host: self.cfg.server.host.clone(),
            port: self.cfg.server.port,
            username: if self.cfg.server.superuser {
                "SuperUser".to_owned()
            } else {
                self.cfg.server.name.clone()
            },
            password: self.password(),
            tokens: self
                .cfg
                .tokens
                .iter()
                .map(|token| token.password.clone())
                .collect(),
            identity,
            verify_cert: verify,
            login_timeout: LOGIN_TIMEOUT,
        };
        let mut connection = match connect(config).await {
            Ok(connection) => connection,
            Err(error) => return Err(error.into()),
        };
        let handle = connection.handle.clone();
        tracing::info!(
            "connected to {}:{} as {}",
            self.cfg.server.host,
            self.cfg.server.port,
            self.cfg.server.name
        );
        if self.cfg.server.register {
            // murmur answers with a denial if the right is not there; the login itself is fine.
            match handle.register_self() {
                Ok(()) => tracing::info!("registering the bot's certificate"),
                Err(error) => tracing::warn!("cannot register the certificate: {error}"),
            }
        }

        // The bot plays in the channel the server put it in at login; nothing is joined by name.
        let login = handle.with_state(|state| {
            state
                .session
                .and_then(|session| state.users.get(&session))
                .map(|user| user.channel_id)
        });
        let Some(login_channel) = login else {
            return Ok(None);
        };
        // The configured channel is optional: without it the bot plays where the server put it.
        self.game = Some(login_channel);
        if let Some(want) = &self.cfg.server.channel {
            match self.resolve(&handle, want) {
                Some(id) => {
                    if self.needs_password(&handle, id) {
                        tracing::warn!(
                            "channel {want} needs a password: add a [[token]] block for it"
                        );
                    }
                    handle.join_channel(id)?;
                    self.game = Some(id);
                }
                None => tracing::warn!(
                    "channel {want} is not on the server yet; the bot plays where it is and joins {want} when it appears"
                ),
            }
        }
        self.set_game_name(&handle);
        tracing::info!("playing in channel {}", self.quiz.channel());
        self.sync_players(&handle)?;
        let actions = self.quiz.reset();
        self.apply(&handle, actions)?;

        let mut ticker = interval(Duration::from_secs(self.cfg.quiz.tick_secs));
        loop {
            tokio::select! {
                event = connection.events.recv() => {
                    let Some(event) = event else { return Ok(None) };
                    if let Event::Disconnected(reason) = event {
                        return Ok(Some(reason));
                    }
                    self.handle_event(&handle, &event)?;
                }
                _ = ticker.tick() => {
                    let actions = self.quiz.tick(self.now());
                    self.apply(&handle, actions)?;
                }
                command = commands.recv() => {
                    match command {
                        Some(Command::Reload) => self.reload()?,
                        Some(Command::Stop) | None => {
                            self.close(&handle).await;
                            return Ok(None);
                        }
                    }
                }
            }
        }
    }

    /// Sends what is still queued at the server's pace, then closes, so the last lines of the game
    /// are not lost.
    async fn close(&mut self, handle: &Handle) {
        handle.disconnect();
        handle.closed().await;
    }

    /// Names the channel the bot plays in, from the server state.
    fn set_game_name(&mut self, handle: &Handle) {
        let Some(id) = self.game else {
            return;
        };
        let name = handle
            .with_state(|state| state.channels.get(&id).map(|channel| channel.name.clone()))
            .unwrap_or_default();
        self.quiz.set_channel(&name);
    }

    /// The password configured for a channel (by name or by path), if there is one.
    pub fn token_for(&self, channel: &str) -> Option<&Token> {
        self.cfg
            .tokens
            .iter()
            .find(|token| token.channel.eq_ignore_ascii_case(channel))
    }

    /// murmur answers a join to a restricted channel with a denial: the password has to be one of
    /// the tokens presented at login, so a missing block is worth saying.
    fn needs_password(&self, handle: &Handle, channel: u32) -> bool {
        handle.with_state(|state| {
            let Some(entry) = state.channels.get(&channel) else {
                return false;
            };
            entry.is_enter_restricted
                && entry.can_enter == Some(false)
                && self.token_for(&entry.name).is_none()
        })
    }

    /// Config value first, then the environment: `SERVER_PASSWORD`, or `SUPERUSER_PASSWORD` for the
    /// admin login of a local test server.
    fn password(&self) -> Option<String> {
        let name = if self.cfg.server.superuser {
            "SUPERUSER_PASSWORD"
        } else {
            "SERVER_PASSWORD"
        };
        self.cfg
            .server
            .password
            .clone()
            .or_else(|| std::env::var(name).ok().filter(|value| !value.is_empty()))
    }

    fn handle_event(&mut self, handle: &Handle, event: &Event) -> Result<()> {
        match event {
            Event::TextMessage(message) => {
                let own = handle.with_state(|state| state.session);
                if message.actor == own {
                    // Own messages come back to the client: never an answer to itself.
                    return Ok(());
                }
                let name = handle
                    .with_state(|state| {
                        message
                            .actor
                            .and_then(|session| state.users.get(&session))
                            .map(|user| user.name.clone())
                    })
                    .unwrap_or_default();
                if name.is_empty() {
                    return Ok(());
                }
                let text = html::to_text(&message.message);
                if text.is_empty() {
                    return Ok(());
                }
                let in_channel = message
                    .target
                    .channels
                    .iter()
                    .chain(message.target.trees.iter())
                    .any(|id| Some(*id) == self.game);
                let private = message
                    .target
                    .sessions
                    .iter()
                    .any(|session| Some(*session) == own);
                if !in_channel && !private {
                    return Ok(());
                }
                if self.cfg.quiz.verbose {
                    tracing::info!("{name}: {text}");
                }
                let actions = self.quiz.message(&name, &text, self.now())?;
                self.apply(handle, actions)?;
            }
            Event::UserUpdated(change) => {
                let name = change.user.name.clone();
                let previous = self.names.get(&change.user.session).cloned();
                if let Some(previous) = previous
                    && previous != name
                {
                    let actions = self.quiz.rename(&previous, &name)?;
                    self.apply(handle, actions)?;
                }
                self.names.insert(change.user.session, name);
                self.sync_players(handle)?;
            }
            Event::UserRemoved { user, reason, .. } => {
                if reason.is_some() {
                    tracing::info!(
                        "{} left: {}",
                        user.name,
                        reason.as_deref().unwrap_or("kicked")
                    );
                }
                self.names.remove(&user.session);
                self.sync_players(handle)?;
            }
            Event::ChannelUpdated(updated) => {
                // A configured channel that was not there at login.
                let Some(want) = &self.cfg.server.channel else {
                    return Ok(());
                };
                if Some(updated.id) == self.game || updated.name != *want {
                    return Ok(());
                }
                tracing::info!("channel {want} appeared, joining it");
                handle.join_channel(updated.id)?;
                self.game = Some(updated.id);
                self.quiz.set_channel(&updated.name);
                self.sync_players(handle)?;
            }
            Event::PermissionDenied(denied) => {
                tracing::warn!("the server refused a request: {denied}");
            }
            _ => {}
        }
        Ok(())
    }

    /// Players are the people in the bot's voice channel, or following it with Listen. Names are
    /// remembered with their session so a whisper can find a player again: the login sends no events
    /// for the people who were already on the server.
    fn sync_players(&mut self, handle: &Handle) -> Result<()> {
        let Some(channel) = self.game else {
            return Ok(());
        };
        let players: BTreeSet<String> = handle.with_state(|state| {
            state
                .users
                .values()
                .filter(|user| is_player(user, channel))
                .map(|user| {
                    self.names.insert(user.session, user.name.clone());
                    user.name.clone()
                })
                .collect()
        });
        let known: BTreeSet<String> = self.quiz.quizzers().keys().cloned().collect();
        for name in players.difference(&known) {
            self.quiz.add(name);
        }
        for name in known.difference(&players) {
            self.quiz.remove(name);
        }
        Ok(())
    }

    fn apply(&mut self, handle: &Handle, actions: Vec<Action>) -> Result<()> {
        for action in actions {
            match action {
                Action::Channel(line) => {
                    if self.cfg.quiz.verbose {
                        tracing::info!("say: {}", line.text);
                    }
                    let Some(id) = self.game else {
                        tracing::warn!("nowhere to say it: the bot has no channel");
                        continue;
                    };
                    handle.send_text(
                        &TextTarget::channel(id),
                        &self.render.render(line.kind, &line.text),
                    )?;
                }
                Action::Private { quizzer, line } => {
                    let session = self
                        .names
                        .iter()
                        .find(|(_, name)| *name == &quizzer)
                        .map(|(session, _)| *session);
                    match session {
                        Some(session) => handle.send_text(
                            &TextTarget::user(session),
                            &self.render.render(line.kind, &line.text),
                        )?,
                        None => tracing::warn!("cannot whisper to {quizzer}: not on the server"),
                    }
                }
                Action::Topic(text) => {
                    let Some(id) = self.game else { continue };
                    if !handle.with_state(|state| may_write(&state.permissions, id)) {
                        tracing::info!(
                            "no Write permission in the channel: the topic stays as it is"
                        );
                        continue;
                    }
                    // murmur forwards a description inline only below 128 bytes.
                    let description = truncate(&text, 127);
                    handle.send_channel_state(proto::ChannelState {
                        channel_id: Some(id),
                        description: Some(description),
                        ..Default::default()
                    })?;
                }
                Action::JoinChannel(name) => {
                    let id = self.resolve(handle, &name);
                    match id {
                        Some(id) => {
                            if self.needs_password(handle, id) {
                                tracing::warn!(
                                    "channel {name} needs a password: add a [[token]] block for it"
                                );
                            }
                            tracing::info!("joining channel {name}");
                            handle.join_channel(id)?;
                            self.game = Some(id);
                            self.quiz.set_channel(&name);
                            self.sync_players(handle)?;
                        }
                        None => tracing::warn!("no channel {name} on the server"),
                    }
                }
                Action::ReloadQuestions => self.reload()?,
            }
        }
        Ok(())
    }

    fn reload(&mut self) -> Result<()> {
        let bank = Bank::load(&self.cfg.quiz.questions)?;
        tracing::info!("reloaded {} questions", bank.len());
        self.quiz.set_bank(bank);
        Ok(())
    }

    /// A channel by name or by path (`Root/Quiz`), used by `!join`. The root channel is 0 on every
    /// standard server, whatever the server calls it; an ambiguous name needs a path.
    fn resolve(&self, handle: &Handle, name: &str) -> Option<u32> {
        handle.with_state(|state| {
            if name.contains('/') {
                let mut parts = name.split('/').filter(|part| !part.is_empty()).peekable();
                let mut id = 0;
                let root_name = state.channels.get(&0).map(|channel| channel.name.as_str());
                if parts
                    .peek()
                    .is_some_and(|part| *part == "Root" || Some(*part) == root_name)
                {
                    parts.next();
                }
                for part in parts {
                    id = state.child_by_name(id, part)?.id;
                }
                return Some(id);
            }
            if name.eq_ignore_ascii_case("root") {
                return Some(0);
            }
            let mut found = state
                .channels
                .values()
                .filter(|channel| channel.name == name);
            match (found.next(), found.next()) {
                (Some(channel), None) => Some(channel.id),
                _ => None,
            }
        })
    }
}

fn is_player(user: &User, channel: u32) -> bool {
    user.channel_id == channel || user.listening.contains(&channel)
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn truncate(text: &str, limit: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if out.len() + c.len_utf8() > limit {
            break;
        }
        out.push(c);
    }
    out
}
