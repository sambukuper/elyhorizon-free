//! The relay session, on its own thread inside a small tokio runtime: fetch
//! a relay from the broker, connect over QUIC, open the control stream,
//! publish the assigned domain, then bridge every stream the relay opens
//! (one per guest) to the world's loopback port, byte for byte. e4mc's
//! `QuiclimeSession` is the reference; the shape is the same as
//! horizon-steam's share and horizon-friends' host: started in the
//! background, polled by the game.
use crate::control::{self, Clientbound, Decoder, Serverbound};
use quinn::{Endpoint, RecvStream, SendStream};
use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::watch};

/// The e4mc broker hands out the least loaded relay.
pub const BROKER_URL: &str = "https://broker.e4mc.link/getBestRelay";
/// The ALPN both ends of the relay session speak.
const ALPN: &[u8] = b"quiclime";
/// How many guest streams the session advertises (the Java client's 512;
/// the integrated server's own limit of 8 is what actually gates).
const MAX_STREAMS: u32 = 512;
/// QUIC keep-alive: the relay's own is 5 s, so ours must be shorter or the
/// session dies during quiet periods.
const KEEP_ALIVE: Duration = Duration::from_secs(3);

pub struct SessionConfig {
    /// The world's loopback port (Open to LAN's).
    pub lan: SocketAddr,
    /// A specific relay, skipping the broker (a self-hosted quiclime, tests).
    pub relay: Option<(String, u16)>,
}

#[derive(Default)]
pub struct SessionState {
    pub domain: Option<String>,
    pub guests: usize,
    pub problem: Option<String>,
}

pub(crate) struct Shared {
    pub config: SessionConfig,
    pub events: Sender<crate::PublishEvent>,
    pub state: Arc<Mutex<SessionState>>,
    pub guests: Arc<AtomicUsize>,
    pub stop: Arc<watch::Sender<bool>>,
}

impl Shared {
    fn emit(&self, event: crate::PublishEvent) {
        let _ = self.events.send(event);
    }
    pub fn stopped(&self) -> bool {
        *self.stop.borrow()
    }
    fn set_problem(&self, problem: String) {
        self.state.lock().unwrap().problem = Some(problem.clone());
        self.emit(crate::PublishEvent::Problem(problem));
    }
}

/// The thread body: one runtime, one session, until stopped or failed.
pub(crate) fn run(shared: Arc<Shared>) {
    // The broker is plain HTTPS and blocking; it runs before the runtime.
    let relay = shared.config.relay.clone().map(Ok).unwrap_or_else(|| {
        broker_relay().map_err(|e| {
            if !shared.stopped() {
                shared.set_problem(format!("the relay broker didn't answer: {e}"));
            }
            e
        })
    });
    let relay = match relay {
        Ok(relay) => relay,
        Err(_) => return,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("e4mc: tokio runtime");
    let stop = shared.stop.subscribe();
    runtime.block_on(async {
        // The endpoint must outlive the session: dropping it kills the
        // connections silently, without the close frame that tells the relay
        // to end its guests. It is released only after wait_idle.
        let endpoint = match make_endpoint() {
            Ok(endpoint) => endpoint,
            Err(error) => {
                if !shared.stopped() {
                    shared.set_problem(error.to_string());
                }
                return;
            }
        };
        let result = session(&shared, relay, stop.clone(), &endpoint).await;
        if let Err(error) = result
            && !shared.stopped()
        {
            shared.set_problem(error.to_string());
        }
        if shared.stopped() {
            // The close frame has to reach the relay before the socket dies.
            let _ = tokio::time::timeout(Duration::from_secs(3), endpoint.wait_idle()).await;
        }
    });
    log::info!("e4mc: session ended, shutting the runtime down");
    runtime.shutdown_timeout(Duration::from_secs(2));
}

fn broker_relay() -> Result<(String, u16), String> {
    let response = ureq::get(BROKER_URL)
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| e.to_string())?;
    let broker: BrokerResponse = response
        .into_json()
        .map_err(|e| format!("the broker's answer made no sense: {e}"))?;
    Ok((broker.host, broker.port))
}

#[derive(serde::Deserialize)]
struct BrokerResponse {
    #[allow(dead_code)]
    id: String,
    host: String,
    port: u16,
}

fn make_endpoint() -> anyhow::Result<Endpoint> {
    let mut root = rustls::RootCertStore::empty();
    root.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(root)
        .with_no_client_auth();
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(KEEP_ALIVE));
    transport.max_concurrent_bidi_streams(MAX_STREAMS.into());
    transport
        .stream_receive_window((1_250_000u32).into())
        .receive_window((8_000_000u32).into());
    let mut client = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    ));
    client.transport_config(Arc::new(transport));
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client);
    Ok(endpoint)
}

async fn session(
    shared: &Arc<Shared>,
    (host, port): (String, u16),
    mut stop: watch::Receiver<bool>,
    endpoint: &Endpoint,
) -> anyhow::Result<()> {
    log::info!("e4mc: using relay {host}:{port}");

    let address = tokio::net::lookup_host((host.as_str(), port))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("the relay's name didn't resolve"))?;
    let connecting = endpoint.connect(address, host.as_str())?;
    // A relay that doesn't answer hangs the handshake forever without this.
    let connection = tokio::select! {
        _ = stop.changed() => return Ok(()),
        connection = tokio::time::timeout(Duration::from_secs(10), connecting) => {
            connection.map_err(|_| anyhow::anyhow!("the relay didn't answer in 10 seconds"))??
        }
    };
    log::info!("e4mc: relay session open");

    // The control channel: one bidirectional stream carries the domain
    // negotiation and the relay's broadcasts. It stays open.
    let (mut send_control, mut recv_control) = connection.open_bi().await?;
    write_message(&mut send_control, &Serverbound::ProbeCapabilities).await?;
    write_message(&mut send_control, &Serverbound::RequestDomainAssignment).await?;
    let mut decoder = Decoder::default();
    let mut buf = vec![0u8; 4096];
    let domain = 'domain: loop {
        let read = tokio::select! {
            _ = stop.changed() => return Ok(()),
            read = recv_control.read(&mut buf) => match read {
                Ok(Some(read)) => read,
                Ok(None) => anyhow::bail!("the relay closed the control channel before assigning a domain"),
                Err(e) => return Err(e.into()),
            },
        };
        for message in decoder.feed(&buf[..read]).map_err(anyhow::Error::msg)? {
            if let Clientbound::DomainAssignmentComplete(domain) = message {
                break 'domain domain;
            }
        }
    };
    log::info!("e4mc: domain assigned: {domain}");
    shared.state.lock().unwrap().domain = Some(domain.clone());
    shared.emit(crate::PublishEvent::Published { domain });

    // Serving: the relay's broadcasts on the control channel, and one
    // bidirectional stream per guest.
    loop {
        tokio::select! {
            _ = stop.changed() => {
                log::info!("e4mc: stop asked: closing the relay session");
                connection.close(0u32.into(), b"horizon stopped sharing");
                return Ok(());
            }
            read = recv_control.read(&mut buf) => {
                let read = match read {
                    Ok(Some(read)) => read,
                    Ok(None) => anyhow::bail!("the relay closed the session"),
                    Err(e) => return Err(e.into()),
                };
                for message in decoder.feed(&buf[..read]).map_err(anyhow::Error::msg)? {
                    match message {
                        Clientbound::RequestMessageBroadcast(message) => {
                            shared.emit(crate::PublishEvent::Broadcast(message))
                        }
                        Clientbound::DomainAssignmentComplete(domain) => {
                            shared.state.lock().unwrap().domain = Some(domain);
                        }
                        _ => {}
                    }
                }
            }
            stream = connection.accept_bi() => {
                let (send, recv) = stream?;
                tokio::spawn(bridge(shared.clone(), send, recv));
            }
        }
    }
}

async fn write_message(send: &mut SendStream, message: &Serverbound) -> anyhow::Result<()> {
    send.write_all(&control::frame(&message.to_json())).await?;
    Ok(())
}

/// Returns the guest count to zero when the bridge ends, however it ends:
/// the runtime cancels the task on shutdown, and a plain decrement after
/// the pump would be skipped with it.
struct GuestGuard {
    shared: Arc<Shared>,
}

impl Drop for GuestGuard {
    fn drop(&mut self) {
        self.shared.guests.fetch_sub(1, Ordering::Relaxed);
        self.shared.state.lock().unwrap().guests -= 1;
        self.shared.emit(crate::PublishEvent::GuestLeft);
    }
}

/// One guest: the relay opened a stream, bridge it to the world's port
/// until either side ends it. Half-closes carry through: a guest's EOF
/// reaches the server and the server's EOF reaches the guest.
async fn bridge(shared: Arc<Shared>, mut send: SendStream, mut recv: RecvStream) {
    shared.guests.fetch_add(1, Ordering::Relaxed);
    shared.state.lock().unwrap().guests += 1;
    shared.emit(crate::PublishEvent::GuestJoined);
    let _guard = GuestGuard {
        shared: shared.clone(),
    };
    let result = match TcpStream::connect(shared.config.lan).await {
        Ok(server) => {
            let (mut server_read, mut server_write) = server.into_split();
            let to_server = async {
                let copied = tokio::io::copy(&mut recv, &mut server_write).await;
                let _ = server_write.shutdown().await;
                copied
            };
            let to_guest = tokio::io::copy(&mut server_read, &mut send);
            let (a, b) = tokio::join!(to_server, to_guest);
            let _ = send.finish();
            match (a, b) {
                (Ok(_), Ok(_)) => Ok(()),
                (Err(e), _) | (_, Err(e)) => Err(anyhow::anyhow!(e)),
            }
        }
        Err(e) => {
            let _ = send.finish();
            Err(anyhow::anyhow!(e))
        }
    };
    if let Err(e) = result {
        log::info!("e4mc: a guest's bridge ended: {e}");
    }
}
