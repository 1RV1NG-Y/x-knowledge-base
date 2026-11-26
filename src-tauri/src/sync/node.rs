//! One device's sync endpoint: identity, pairing, discovery, and either the PC's server side
//! or the phone's mirror loop.
//!
//! Identity, pairing codes, mDNS and the QUIC endpoint follow reader-sync's core
//! (`core/src/node/mod.rs`). There is no replicated document: the PC's database is the only
//! source of truth, and the phone pulls a snapshot of it plus the media files it lists.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime},
};

use anyhow::{anyhow, bail, Context, Result};
use iroh::{
    endpoint::{presets, Connection, IdleTimeout, QuicTransportConfig, RecvStream, SendStream},
    protocol::{AcceptError, ProtocolHandler, Router},
    Endpoint, EndpointAddr, EndpointId, RelayMode, SecretKey, TransportAddr,
};
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncSeekExt, AsyncWriteExt},
    sync::{broadcast, watch, Notify},
};

use super::{
    proto::{self, FileRef, Manifest, MediaEntry, PairMsg, Request, Response, Ticket},
    snapshot::{self, Built, Fingerprint},
};
use crate::db::SCHEMA_VERSION;

pub const DEFAULT_PORT: u16 = 47130;
const INVITE_LIFETIME: Duration = Duration::from_secs(10 * 60);
const DIAL_TIMEOUT: Duration = Duration::from_secs(4);
const PAIR_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// While connected, the phone asks the PC for changes this often.
const RECHECK: Duration = Duration::from_secs(5 * 60);
const PARALLEL_DOWNLOADS: usize = 6;
const CHUNK: usize = 64 * 1024;

pub enum Role {
    /// The PC: serves snapshots of `db_path` and files under `media_root`.
    Server { db_path: PathBuf, media_root: PathBuf },
    /// The phone: mirrors the paired PC into `db_path` and `media_root`.
    Client { db_path: PathBuf, media_root: PathBuf, hooks: ClientHooks },
}

pub struct ClientHooks {
    /// A new snapshot is now the phone's database.
    pub applied: Box<dyn Fn() + Send + Sync>,
    /// A download started (true) or ended (false): keep the screen on meanwhile.
    pub busy: Box<dyn Fn(bool) + Send + Sync>,
}

pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub device_name: String,
    pub platform: String,
    /// IPv4 UDP port to listen on; a busy port falls back to any free one.
    pub port: u16,
    pub mdns: bool,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    StatusChanged,
    LibraryUpdated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub device_id: String,
    pub device_name: String,
    pub role: &'static str,
    pub port: Option<u16>,
    pub devices: Vec<DeviceStatus>,
    pub progress: Option<Progress>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub connected: bool,
    pub paired_at: String,
    pub last_sync: Option<String>,
}

/// The phone's sync state, for its sync screen.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// idle | connecting | checking | downloading | applying | upToDate | error
    pub phase: String,
    pub message: String,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub last_sync: Option<String>,
    pub last_error: Option<String>,
    pub posts: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    pub ticket: String,
    pub qr_svg: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PeerInfo {
    name: String,
    platform: String,
    paired_at: String,
    #[serde(default)]
    addrs: Vec<SocketAddr>,
    #[serde(default)]
    last_sync: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ClientState {
    applied: Option<String>,
    last_sync: Option<String>,
    posts: Option<u64>,
}

struct PendingInvite {
    secret: [u8; 32],
    expires: Instant,
}

struct Server {
    db_path: PathBuf,
    media_root: PathBuf,
    out_dir: PathBuf,
    /// The current snapshot and the one before it (a download may still be reading it).
    built: tokio::sync::Mutex<Option<(Fingerprint, Built)>>,
    previous: Mutex<Option<Built>>,
}

struct Client {
    db_path: PathBuf,
    media_root: PathBuf,
    incoming: PathBuf,
    hooks: ClientHooks,
    progress: Mutex<Progress>,
    state: Mutex<ClientState>,
    wake: Notify,
}

struct Inner {
    _lock: std::fs::File,
    id: EndpointId,
    dir: PathBuf,
    device_name: String,
    platform: String,
    endpoint: Endpoint,
    router: OnceLock<Router>,
    mdns: Option<MdnsAddressLookup>,
    peers: Mutex<BTreeMap<String, PeerInfo>>,
    invites: Mutex<Vec<PendingInvite>>,
    sessions: Mutex<HashMap<EndpointId, (Connection, u64)>>,
    next_token: AtomicU64,
    events: broadcast::Sender<Event>,
    closing: watch::Sender<bool>,
    server: Option<Server>,
    client: Option<Client>,
}

#[derive(Clone)]
pub struct Node(Arc<Inner>);

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").field("id", &self.0.id).finish()
    }
}

impl Node {
    pub async fn open(cfg: NodeConfig) -> Result<Self> {
        let dir = cfg.data_dir.clone();
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("lock"))?;
        if !try_lock(&lock)? {
            bail!("Phone sync is already running in another copy of X Knowledge Base.");
        }
        let key = load_or_create_key(&dir.join("device.key"))?;
        let id = key.public();
        let peers = std::fs::read(dir.join("peers.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();

        let (server, client) = match cfg.role {
            Role::Server { db_path, media_root } => (
                Some(Server {
                    db_path,
                    media_root,
                    out_dir: dir.join("snapshots"),
                    built: tokio::sync::Mutex::new(None),
                    previous: Mutex::new(None),
                }),
                None,
            ),
            Role::Client { db_path, media_root, hooks } => {
                let state: ClientState = std::fs::read(dir.join("client.json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                    .unwrap_or_default();
                let progress = Progress {
                    phase: "idle".into(),
                    last_sync: state.last_sync.clone(),
                    posts: state.posts,
                    ..Default::default()
                };
                std::fs::create_dir_all(&media_root)?;
                (
                    None,
                    Some(Client {
                        db_path,
                        media_root,
                        incoming: dir.join("incoming"),
                        hooks,
                        progress: Mutex::new(progress),
                        state: Mutex::new(state),
                        wake: Notify::new(),
                    }),
                )
            }
        };

        let endpoint = bind_endpoint(&key, cfg.port).await?;
        let mdns = if cfg.mdns {
            let mdns = MdnsAddressLookup::builder()
                .build(id)
                .map_err(|e| anyhow!("starting mDNS: {e}"))?;
            endpoint.address_lookup().map_err(|e| anyhow!("{e}"))?.add(mdns.clone());
            Some(mdns)
        } else {
            None
        };
        let is_server = server.is_some();
        let node = Node(Arc::new(Inner {
            _lock: lock,
            id,
            dir,
            device_name: clean_name(&cfg.device_name),
            platform: cfg.platform,
            endpoint: endpoint.clone(),
            router: OnceLock::new(),
            mdns,
            peers: Mutex::new(peers),
            invites: Mutex::default(),
            sessions: Mutex::default(),
            next_token: AtomicU64::new(1),
            events: broadcast::channel(64).0,
            closing: watch::channel(false).0,
            server,
            client,
        }));
        if is_server {
            let router = Router::builder(endpoint)
                .accept(proto::SYNC_ALPN, SyncHandler(node.clone()))
                .accept(proto::PAIR_ALPN, PairHandler(node.clone()))
                .spawn();
            let _ = node.0.router.set(router);
        } else {
            tokio::spawn(node.clone().client_loop());
        }
        if node.0.mdns.is_some() {
            tokio::spawn(node.clone().mdns_loop());
        }
        tracing::info!(%id, "sync node ready");
        Ok(node)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.0.events.subscribe()
    }

    pub fn addr(&self) -> EndpointAddr {
        self.0.endpoint.addr()
    }

    pub async fn shutdown(&self) {
        let _ = self.0.closing.send(true);
        if let Some(router) = self.0.router.get() {
            let _ = router.shutdown().await;
        }
        self.0.endpoint.close().await;
    }

    /// The app came back to the foreground or the network changed: rebind and retry now.
    pub async fn network_changed(&self) {
        self.0.endpoint.network_change().await;
        self.sync_now();
    }

    /// Phone: check the PC for changes right away.
    pub fn sync_now(&self) {
        if let Some(client) = &self.0.client {
            client.wake.notify_one();
        }
    }

    pub fn status(&self) -> Status {
        let sessions = self.0.sessions.lock().unwrap();
        let devices = self
            .0
            .peers
            .lock()
            .unwrap()
            .iter()
            .map(|(id, p)| DeviceStatus {
                id: id.clone(),
                name: p.name.clone(),
                platform: p.platform.clone(),
                connected: id.parse().is_ok_and(|id: EndpointId| sessions.contains_key(&id)),
                paired_at: p.paired_at.clone(),
                last_sync: p.last_sync.clone(),
            })
            .collect();
        Status {
            device_id: self.0.id.to_string(),
            device_name: self.0.device_name.clone(),
            role: if self.0.server.is_some() { "pc" } else { "phone" },
            port: self.0.endpoint.bound_sockets().into_iter().find(|a| a.is_ipv4()).map(|a| a.port()),
            devices,
            progress: self.0.client.as_ref().map(|c| c.progress.lock().unwrap().clone()),
        }
    }

    fn changed(&self) {
        let _ = self.0.events.send(Event::StatusChanged);
    }

    fn trusted(&self, peer: &EndpointId) -> bool {
        self.0.peers.lock().unwrap().contains_key(&peer.to_string())
    }

    fn save_peers(&self, peers: &BTreeMap<String, PeerInfo>) {
        if let Err(e) = serde_json::to_vec_pretty(peers)
            .map_err(anyhow::Error::from)
            .and_then(|b| write_atomic(&self.0.dir.join("peers.json"), &b))
        {
            tracing::warn!("could not save peers.json: {e:#}");
        }
    }

    pub fn remove_device(&self, id: &str) -> Result<()> {
        let mut peers = self.0.peers.lock().unwrap();
        if peers.remove(id).is_none() {
            bail!("Unknown device");
        }
        self.save_peers(&peers);
        drop(peers);
        if let Ok(key) = id.parse::<EndpointId>() {
            if let Some((conn, _)) = self.0.sessions.lock().unwrap().remove(&key) {
                conn.close(1u32.into(), b"removed");
            }
        }
        self.changed();
        Ok(())
    }

    fn add_peer_addrs(&self, id: EndpointId, addrs: impl IntoIterator<Item = SocketAddr>) {
        let mut peers = self.0.peers.lock().unwrap();
        let Some(info) = peers.get_mut(&id.to_string()) else { return };
        let before = info.addrs.clone();
        for a in addrs.into_iter().filter(usable_lan_addr) {
            info.addrs.retain(|known| *known != a);
            info.addrs.insert(0, a);
        }
        info.addrs.truncate(8);
        if info.addrs != before {
            self.save_peers(&peers);
            drop(peers);
            self.sync_now();
        }
    }

    // ---- Pairing (the PC shows a code, the phone uses it) ------------------------------

    pub fn create_invite(&self) -> Result<Invite> {
        let secret: [u8; 32] = rand::random();
        let mut invites = self.0.invites.lock().unwrap();
        invites.retain(|i| i.expires > Instant::now());
        invites.push(PendingInvite { secret, expires: Instant::now() + INVITE_LIFETIME });
        let full = self.0.endpoint.addr();
        let addr = EndpointAddr::from_parts(
            full.id,
            full.ip_addrs().filter(|a| usable_lan_addr(a)).map(|a| TransportAddr::Ip(*a)),
        );
        let ticket = Ticket { addr, secret }.encode();
        Ok(Invite {
            qr_svg: proto::qr_svg(&ticket)?,
            ticket,
            expires_at: humantime::format_rfc3339_seconds(SystemTime::now() + INVITE_LIFETIME).to_string(),
        })
    }

    /// Phone: pairs with the PC whose code was scanned or pasted. A phone mirrors one PC, so
    /// this replaces any earlier pairing.
    pub async fn use_code(&self, text: &str) -> Result<String> {
        if self.0.client.is_none() {
            bail!("Show this code on your phone instead.");
        }
        let ticket = Ticket::decode(text)?;
        let other = ticket.addr.id;
        if other == self.0.id {
            bail!("That code came from this device.");
        }
        let conn = tokio::time::timeout(PAIR_TIMEOUT, self.0.endpoint.connect(ticket.addr.clone(), proto::PAIR_ALPN))
            .await
            .map_err(|_| anyhow!("Couldn't reach the PC. Make sure both are on the same Wi-Fi network."))?
            .map_err(|e| anyhow!("Couldn't reach the PC: {e}"))?;
        let (mut send, mut recv) = conn.open_bi().await?;
        let msg = PairMsg::Request {
            secret: ticket.secret,
            name: self.0.device_name.clone(),
            platform: self.0.platform.clone(),
        };
        proto::write_msg(&mut send, &msg).await?;
        send.finish()?;
        let reply = tokio::time::timeout(PAIR_TIMEOUT, proto::read_msg::<_, PairMsg>(&mut recv))
            .await
            .map_err(|_| anyhow!("The PC didn't answer."))??;
        conn.close(0u32.into(), b"paired");
        let (name, platform) = match reply {
            Some(PairMsg::Accepted { name, platform }) => (name, platform),
            Some(PairMsg::Rejected { reason }) => bail!(reason),
            _ => bail!("The PC gave an unexpected answer."),
        };
        {
            let mut peers = self.0.peers.lock().unwrap();
            peers.clear();
            peers.insert(
                other.to_string(),
                PeerInfo {
                    name: clean_name(&name),
                    platform: clean_name(&platform),
                    paired_at: now_iso(),
                    addrs: ticket.addr.ip_addrs().copied().filter(usable_lan_addr).collect(),
                    last_sync: None,
                },
            );
            self.save_peers(&peers);
        }
        let stale: Vec<_> = self.0.sessions.lock().unwrap().drain().collect();
        for (_, (conn, _)) in stale {
            conn.close(0u32.into(), b"replaced");
        }
        self.changed();
        self.sync_now();
        Ok(name)
    }

    fn take_invite(&self, secret: &[u8; 32]) -> bool {
        let mut invites = self.0.invites.lock().unwrap();
        invites.retain(|i| i.expires > Instant::now());
        match invites.iter().position(|i| constant_time_eq(&i.secret, secret)) {
            Some(pos) => {
                invites.remove(pos);
                true
            }
            None => false,
        }
    }

    async fn handle_pair(&self, conn: Connection) -> Result<()> {
        let peer = conn.remote_id();
        let (mut send, mut recv) = conn.accept_bi().await?;
        let reply = match proto::read_msg(&mut recv).await? {
            Some(PairMsg::Request { secret, name, platform }) if self.take_invite(&secret) => {
                let mut peers = self.0.peers.lock().unwrap();
                peers.insert(
                    peer.to_string(),
                    PeerInfo {
                        name: clean_name(&name),
                        platform: clean_name(&platform),
                        paired_at: now_iso(),
                        ..Default::default()
                    },
                );
                self.save_peers(&peers);
                tracing::info!(%peer, "paired");
                PairMsg::Accepted { name: self.0.device_name.clone(), platform: self.0.platform.clone() }
            }
            Some(PairMsg::Request { .. }) => PairMsg::Rejected {
                reason: "This code has expired or was already used. Show a new one on the PC.".into(),
            },
            _ => bail!("malformed pairing request"),
        };
        proto::write_msg(&mut send, &reply).await?;
        send.finish()?;
        let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
        self.changed();
        Ok(())
    }

    // ---- Sessions ----------------------------------------------------------------------

    fn register(&self, peer: EndpointId, conn: &Connection) -> SessionGuard {
        let token = self.0.next_token.fetch_add(1, Ordering::Relaxed);
        if let Some((old, _)) = self.0.sessions.lock().unwrap().insert(peer, (conn.clone(), token)) {
            old.close(0u32.into(), b"replaced");
        }
        self.changed();
        SessionGuard { node: self.clone(), peer, token }
    }

    // ---- PC: serving -------------------------------------------------------------------

    async fn serve(&self, conn: Connection) -> Result<()> {
        let peer = conn.remote_id();
        if !self.trusted(&peer) {
            conn.close(403u32.into(), b"not paired");
            bail!("refused connection from unpaired device {peer}");
        }
        let _guard = self.register(peer, &conn);
        while let Ok((send, recv)) = conn.accept_bi().await {
            let node = self.clone();
            tokio::spawn(async move {
                if let Err(e) = node.serve_stream(peer, send, recv).await {
                    tracing::debug!(%peer, "sync request failed: {e:#}");
                }
            });
        }
        Ok(())
    }

    async fn serve_stream(&self, peer: EndpointId, mut send: SendStream, mut recv: RecvStream) -> Result<()> {
        let server = self.0.server.as_ref().context("not a PC")?;
        let Some(request) = proto::read_msg::<_, Request>(&mut recv).await? else { return Ok(()) };
        match request {
            Request::Manifest { protocol, schema } => {
                let reply = if protocol != proto::PROTOCOL_VERSION {
                    Response::Refused {
                        reason: format!("The phone uses sync version {protocol}; the PC uses {}. Update both apps.", proto::PROTOCOL_VERSION),
                    }
                } else if schema < SCHEMA_VERSION {
                    Response::Refused { reason: "The phone app is older than the PC's. Install the new phone app.".into() }
                } else {
                    match server.current().await {
                        Ok(built) => Response::Manifest(built.manifest),
                        Err(e) => Response::Refused { reason: format!("The PC couldn't prepare your library: {e:#}") },
                    }
                };
                proto::write_msg(&mut send, &reply).await?;
                send.finish()?;
            }
            Request::File { file, offset } => {
                let _ = send.set_priority(-1);
                let Some((path, size)) = server.locate(&file).await else {
                    proto::write_msg(&mut send, &Response::Missing).await?;
                    send.finish()?;
                    return Ok(());
                };
                if offset > size {
                    proto::write_msg(&mut send, &Response::Refused { reason: "offset past the end of the file".into() }).await?;
                    send.finish()?;
                    return Ok(());
                }
                proto::write_msg(&mut send, &Response::Have { size }).await?;
                let mut file = tokio::fs::File::open(&path).await?;
                file.seek(std::io::SeekFrom::Start(offset)).await?;
                let mut remaining = size - offset;
                let mut buf = vec![0u8; CHUNK];
                while remaining > 0 {
                    let want = buf.len().min(remaining as usize);
                    let n = tokio::io::AsyncReadExt::read(&mut file, &mut buf[..want]).await?;
                    if n == 0 {
                        bail!("{} shrank while being sent", path.display());
                    }
                    send.write_all(&buf[..n]).await?;
                    remaining -= n as u64;
                }
                send.finish()?;
                let _ = send.stopped().await;
            }
            Request::Applied { hash } => {
                tracing::info!(%peer, snapshot = %snapshot::hex(&hash[..8]), "phone applied snapshot");
                let mut peers = self.0.peers.lock().unwrap();
                if let Some(info) = peers.get_mut(&peer.to_string()) {
                    info.last_sync = Some(now_iso());
                    self.save_peers(&peers);
                }
                drop(peers);
                self.changed();
                send.finish()?;
            }
        }
        Ok(())
    }

    // ---- Phone: mirroring --------------------------------------------------------------

    fn client(&self) -> &Client {
        self.0.client.as_ref().expect("phone role")
    }

    fn set_progress(&self, f: impl FnOnce(&mut Progress)) {
        f(&mut self.client().progress.lock().unwrap());
        self.changed();
    }

    async fn client_loop(self) {
        let mut closing = self.0.closing.subscribe();
        let mut delay = Duration::from_secs(1);
        loop {
            let pc = self.0.peers.lock().unwrap().iter().next().map(|(id, info)| (id.clone(), info.clone()));
            if let Some((id, info)) = pc {
                match id.parse::<EndpointId>() {
                    Ok(peer) => match self.client_session(peer, &info).await {
                        Ok(()) => delay = Duration::from_secs(1),
                        Err(e) => {
                            let message = format!("{e:#}");
                            tracing::debug!("sync with {}: {message}", info.name);
                            self.set_progress(|p| {
                                if message.starts_with("couldn't connect") {
                                    p.phase = "idle".into();
                                    p.message = format!("Waiting for {}. Open X Knowledge Base on the PC.", info.name);
                                } else {
                                    p.phase = "error".into();
                                    p.message = message.clone();
                                    p.last_error = Some(message.clone());
                                }
                            });
                            delay = (delay * 2).min(MAX_BACKOFF);
                        }
                    },
                    Err(_) => delay = MAX_BACKOFF,
                }
            } else {
                self.set_progress(|p| {
                    p.phase = "idle".into();
                    p.message = "Not paired with a PC yet.".into();
                });
                delay = MAX_BACKOFF;
            }
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = self.client().wake.notified() => {}
                _ = closing.wait_for(|c| *c) => return,
            }
        }
    }

    async fn client_session(&self, peer: EndpointId, info: &PeerInfo) -> Result<()> {
        let addr = EndpointAddr::from_parts(peer, info.addrs.iter().copied().map(TransportAddr::Ip));
        let conn = match tokio::time::timeout(DIAL_TIMEOUT, self.0.endpoint.connect(addr, proto::SYNC_ALPN)).await {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => bail!("couldn't connect: {e}"),
            Err(_) => bail!("couldn't connect: timed out"),
        };
        let _guard = self.register(peer, &conn);
        let mut closing = self.0.closing.subscribe();
        loop {
            self.sync_once(&conn, &info.name).await?;
            tokio::select! {
                _ = tokio::time::sleep(RECHECK) => {}
                _ = self.client().wake.notified() => {}
                _ = conn.closed() => return Ok(()),
                _ = closing.wait_for(|c| *c) => return Ok(()),
            }
        }
    }

    async fn request_manifest(conn: &Connection) -> Result<Manifest> {
        let (mut send, mut recv) = conn.open_bi().await?;
        proto::write_msg(&mut send, &Request::Manifest { protocol: proto::PROTOCOL_VERSION, schema: SCHEMA_VERSION }).await?;
        send.finish()?;
        match proto::read_msg::<_, Response>(&mut recv).await? {
            Some(Response::Manifest(manifest)) => Ok(manifest),
            Some(Response::Refused { reason }) => bail!(reason),
            _ => bail!("The PC gave an unexpected answer."),
        }
    }

    async fn sync_once(&self, conn: &Connection, pc: &str) -> Result<()> {
        self.set_progress(|p| {
            p.phase = "checking".into();
            p.message = format!("Checking {pc} for changes…");
        });
        let manifest = Self::request_manifest(conn).await?;
        if manifest.schema > SCHEMA_VERSION {
            bail!("The PC's app is newer than this one. Install the new phone app.");
        }
        let client = self.client();
        let root = client.media_root.clone();
        let entries = manifest.media.clone();
        let missing: Vec<MediaEntry> = tokio::task::spawn_blocking(move || {
            entries
                .into_iter()
                .filter(|e| std::fs::metadata(root.join(&e.path)).map_or(true, |m| m.len() != e.size))
                .collect()
        })
        .await?;
        let wanted = snapshot::hex(&manifest.snapshot);
        let need_db = client.state.lock().unwrap().applied.as_deref() != Some(wanted.as_str()) || !client.db_path.exists();
        if missing.is_empty() && !need_db {
            self.finish_sync(&manifest);
            return Ok(());
        }

        (client.hooks.busy)(true);
        let _awake = BusyGuard(self.clone());
        let bytes_total = missing.iter().map(|e| e.size).sum::<u64>() + if need_db { manifest.snapshot_size } else { 0 };
        let files_total = missing.len() as u64 + u64::from(need_db);
        self.set_progress(|p| {
            p.phase = "downloading".into();
            p.message = format!("Downloading from {pc}…");
            p.files_done = 0;
            p.files_total = files_total;
            p.bytes_done = 0;
            p.bytes_total = bytes_total;
        });

        // Media first, so the new database never points at files that haven't arrived.
        let mut tasks = tokio::task::JoinSet::new();
        let mut queue = missing.into_iter();
        let mut skipped = 0u64;
        loop {
            while tasks.len() < PARALLEL_DOWNLOADS {
                let Some(entry) = queue.next() else { break };
                let node = self.clone();
                let conn = conn.clone();
                tasks.spawn(async move {
                    let dest = node.client().media_root.join(&entry.path);
                    node.fetch(&conn, FileRef::Media { path: entry.path.clone() }, &dest, entry.size, false).await
                });
            }
            let Some(result) = tasks.join_next().await else { break };
            // A file the PC no longer has is skipped (the post shows its online copy).
            if !result?? {
                skipped += 1;
            }
            self.set_progress(|p| p.files_done += 1);
        }

        if need_db {
            std::fs::create_dir_all(&client.incoming)?;
            let part = client.incoming.join(format!("{wanted}.db"));
            if !self.fetch(conn, FileRef::Snapshot { hash: manifest.snapshot }, &part, manifest.snapshot_size, true).await? {
                // The PC rebuilt its snapshot meanwhile; the next check picks up the new one.
                let _ = std::fs::remove_file(&part);
                client.wake.notify_one();
                return Ok(());
            }
            let (hash, _) = snapshot::hash_file(&part)?;
            if hash != manifest.snapshot {
                let _ = std::fs::remove_file(&part);
                bail!("The library arrived damaged; it will be downloaded again.");
            }
            self.set_progress(|p| {
                p.phase = "applying".into();
                p.message = "Updating the library…".into();
                p.files_done += 1;
            });
            let (db, media) = (client.db_path.clone(), client.media_root.clone());
            tokio::task::spawn_blocking(move || snapshot::apply(&part, &db, &media)).await??;
            {
                let mut state = client.state.lock().unwrap();
                state.applied = Some(wanted);
                self.save_client_state(&state);
            }
        }
        let keep: HashSet<String> = manifest.media.iter().map(|e| e.path.clone()).collect();
        let (root, incoming) = (client.media_root.clone(), client.incoming.clone());
        tokio::task::spawn_blocking(move || {
            snapshot::prune_media(&root, &keep);
            let _ = std::fs::remove_dir_all(incoming);
        })
        .await?;
        if skipped > 0 {
            tracing::warn!("{skipped} media files were missing on the PC");
        }
        (client.hooks.applied)();
        let _ = self.0.events.send(Event::LibraryUpdated);
        self.finish_sync(&manifest);
        // Tell the PC (for its "last synced" line); not worth failing the sync over.
        if let Ok((mut send, _recv)) = conn.open_bi().await {
            let _ = proto::write_msg(&mut send, &Request::Applied { hash: manifest.snapshot }).await;
            let _ = send.finish();
        }
        Ok(())
    }

    fn finish_sync(&self, manifest: &Manifest) {
        let client = self.client();
        let now = now_iso();
        {
            let mut state = client.state.lock().unwrap();
            state.last_sync = Some(now.clone());
            state.posts = Some(manifest.posts);
            self.save_client_state(&state);
        }
        self.set_progress(|p| {
            p.phase = "upToDate".into();
            p.message = "Up to date".into();
            p.last_sync = Some(now);
            p.last_error = None;
            p.posts = Some(manifest.posts);
        });
    }

    fn save_client_state(&self, state: &ClientState) {
        if let Err(e) = serde_json::to_vec_pretty(state)
            .map_err(anyhow::Error::from)
            .and_then(|b| write_atomic(&self.0.dir.join("client.json"), &b))
        {
            tracing::warn!("could not save client.json: {e:#}");
        }
    }

    /// Downloads one file to `dest` via `dest.part`, resuming a partial one. Returns false
    /// if the PC no longer has it (or it changed size there).
    async fn fetch(&self, conn: &Connection, file: FileRef, dest: &Path, expected: u64, durable: bool) -> Result<bool> {
        let part = sibling(dest, ".part");
        if let Some(parent) = part.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut offset = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
        if offset > expected {
            tokio::fs::remove_file(&part).await?;
            offset = 0;
        }
        let (mut send, mut recv) = conn.open_bi().await?;
        proto::write_msg(&mut send, &Request::File { file, offset }).await?;
        send.finish()?;
        match proto::read_msg::<_, Response>(&mut recv).await? {
            Some(Response::Have { size }) if size == expected => {}
            Some(Response::Have { .. }) | Some(Response::Missing) => return Ok(false),
            Some(Response::Refused { reason }) => bail!(reason),
            _ => bail!("The PC gave an unexpected answer."),
        }
        let mut out = tokio::fs::OpenOptions::new().create(true).append(true).open(&part).await?;
        if offset > 0 {
            self.set_progress(|p| p.bytes_done += offset);
        }
        let mut buf = vec![0u8; CHUNK];
        let mut last_report = Instant::now();
        let mut unreported = 0u64;
        while offset < expected {
            let want = buf.len().min((expected - offset) as usize);
            let n = recv.read(&mut buf[..want]).await?.context("the PC stopped sending")?;
            out.write_all(&buf[..n]).await?;
            offset += n as u64;
            unreported += n as u64;
            if last_report.elapsed() > Duration::from_millis(200) {
                let add = std::mem::take(&mut unreported);
                self.set_progress(|p| p.bytes_done += add);
                last_report = Instant::now();
            }
        }
        if recv.read(&mut buf[..1]).await?.is_some_and(|n| n > 0) {
            bail!("the PC sent more than the file's size");
        }
        out.flush().await?;
        if durable {
            out.sync_all().await?;
        }
        drop(out);
        self.set_progress(|p| p.bytes_done += unreported);
        tokio::fs::rename(&part, dest).await?;
        Ok(true)
    }

    async fn mdns_loop(self) {
        let Some(mdns) = self.0.mdns.clone() else { return };
        let mut events = mdns.subscribe().await;
        let mut closing = self.0.closing.subscribe();
        loop {
            let event = tokio::select! {
                e = events.next() => e,
                _ = closing.wait_for(|c| *c) => return,
            };
            match event {
                Some(DiscoveryEvent::Discovered { endpoint_info, .. }) => {
                    let addr = endpoint_info.into_endpoint_addr();
                    if self.trusted(&addr.id) {
                        self.add_peer_addrs(addr.id, addr.ip_addrs().copied());
                    }
                }
                Some(_) => {}
                None => return,
            }
        }
    }
}

impl Server {
    async fn current(&self) -> Result<Built> {
        let mut built = self.built.lock().await;
        let fingerprint = snapshot::fingerprint(&self.db_path);
        if let Some((known, snapshot)) = built.as_ref() {
            if *known == fingerprint {
                return Ok(snapshot.clone());
            }
        }
        let (db, media, out) = (self.db_path.clone(), self.media_root.clone(), self.out_dir.clone());
        let started = Instant::now();
        let fresh = tokio::task::spawn_blocking(move || snapshot::build(&db, &media, &out)).await??;
        tracing::info!(
            posts = fresh.manifest.posts,
            media = fresh.manifest.media.len(),
            bytes = fresh.manifest.snapshot_size,
            "built snapshot in {:?}",
            started.elapsed()
        );
        let replaced = built.replace((fingerprint, fresh.clone())).map(|(_, b)| b);
        // Keep the current and previous files; delete older ones.
        let mut previous = self.previous.lock().unwrap();
        if let Some(old) = replaced.filter(|old| old.file != fresh.file) {
            if let Some(older) = previous.replace(old) {
                if older.file != fresh.file {
                    let _ = std::fs::remove_file(&older.file);
                }
            }
        }
        let keep: Vec<&Path> = [Some(fresh.file.as_path()), previous.as_ref().map(|b| b.file.as_path())].into_iter().flatten().collect();
        if let Ok(entries) = std::fs::read_dir(&self.out_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "db") && !keep.contains(&path.as_path()) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(fresh)
    }

    async fn locate(&self, file: &FileRef) -> Option<(PathBuf, u64)> {
        let current = self.built.lock().await.as_ref().map(|(_, b)| b.clone());
        let previous = self.previous.lock().unwrap().clone();
        let path = match file {
            FileRef::Snapshot { hash } => [current, previous]
                .into_iter()
                .flatten()
                .find(|b| b.manifest.snapshot == *hash)
                .map(|b| b.file)?,
            FileRef::Media { path } => {
                let listed = [current, previous].into_iter().flatten().any(|b| b.media.contains(path));
                if !listed {
                    return None;
                }
                self.media_root.join(path)
            }
        };
        let size = tokio::fs::metadata(&path).await.ok().filter(|m| m.is_file())?.len();
        Some((path, size))
    }
}

struct SessionGuard {
    node: Node,
    peer: EndpointId,
    token: u64,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let mut sessions = self.node.0.sessions.lock().unwrap();
        if sessions.get(&self.peer).is_some_and(|(_, t)| *t == self.token) {
            sessions.remove(&self.peer);
        }
        drop(sessions);
        self.node.changed();
    }
}

struct BusyGuard(Node);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        (self.0.client().hooks.busy)(false);
    }
}

#[derive(Debug, Clone)]
struct SyncHandler(Node);

impl ProtocolHandler for SyncHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        if let Err(e) = self.0.serve(connection).await {
            tracing::debug!("incoming sync session: {e:#}");
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PairHandler(Node);

impl ProtocolHandler for PairHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        if let Err(e) = self.0.handle_pair(connection).await {
            tracing::warn!("pairing failed: {e:#}");
        }
        Ok(())
    }
}

async fn bind_endpoint(key: &SecretKey, port: u16) -> Result<Endpoint> {
    let transport = || {
        QuicTransportConfig::builder()
            .keep_alive_interval(Duration::from_secs(2))
            .max_idle_timeout(Some(IdleTimeout::try_from(Duration::from_secs(10)).expect("valid timeout")))
            .build()
    };
    let builder = || {
        Endpoint::builder(presets::Minimal)
            .secret_key(key.clone())
            .relay_mode(RelayMode::Disabled)
            .transport_config(transport())
    };
    if port != 0 {
        match builder().bind_addr(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))?.bind().await {
            Ok(e) => return Ok(e),
            Err(e) => tracing::warn!("port {port} unavailable ({e}); using any free port"),
        }
    }
    builder().bind().await.map_err(|e| anyhow!("could not open a network socket: {e}"))
}

/// Exclusive advisory lock without blocking; `Ok(false)` means another process holds it.
fn try_lock(file: &std::fs::File) -> std::io::Result<bool> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        // SAFETY: flock on a valid, open file descriptor owned by `file`.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::EWOULDBLOCK) {
            Ok(false)
        } else {
            Err(error)
        }
    }
    #[cfg(not(unix))]
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(std::fs::TryLockError::Error(e)) => Err(e),
    }
}

fn load_or_create_key(path: &Path) -> Result<SecretKey> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let bytes: [u8; 32] = bytes.try_into().map_err(|_| anyhow!("device.key is damaged"))?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let key = SecretKey::generate();
            write_atomic(path, &key.to_bytes())?;
            Ok(key)
        }
        Err(e) => Err(e).context("reading device.key"),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = sibling(path, ".tmp");
    {
        let mut file = std::fs::File::create(&temp)?;
        std::io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temp, path)?;
    Ok(())
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Addresses worth dialing or putting in a code: skip loopback and link-local.
fn usable_lan_addr(a: &SocketAddr) -> bool {
    match a {
        SocketAddr::V4(v4) => !v4.ip().is_loopback() && !v4.ip().is_link_local(),
        SocketAddr::V6(v6) => !v6.ip().is_loopback() && !v6.ip().is_unicast_link_local(),
    }
}

fn clean_name(name: &str) -> String {
    let name: String = name.trim().chars().filter(|c| !c.is_control()).take(64).collect();
    if name.is_empty() {
        "Unnamed device".into()
    } else {
        name
    }
}

fn now_iso() -> String {
    humantime::format_rfc3339_seconds(SystemTime::now()).to_string()
}

fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
