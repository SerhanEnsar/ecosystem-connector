//! `ec`: a command line tool for trying out Ecosystem Connector.
//!
//! It exercises the Rust core directly: identity, pairing, discovery and connections.
//! The desktop and mobile apps will use the same core.

use std::io::{BufRead, Write};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use ec_core::{DeviceIdentity, PairingInvite, TrustStore, TrustedDevice};
use ec_net::pairing::{PairingListener, PairingPrompt, pair_with};
use ec_net::{Advertiser, Node, Router, browse, local_addrs, serve_connection};
use ec_transfer::{ReceiveEvent, Receiver, SendEvent, SendOptions, send_paths};

#[derive(Parser)]
#[command(name = "ec", version, about = "Ecosystem Connector command line tool")]
struct Cli {
    /// Where this device's identity and paired devices are stored.
    /// Use different folders to run two devices on one machine.
    #[arg(long, env = "EC_HOME", global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show this device's id and name.
    Id,
    /// Change this device's name.
    Rename {
        /// The new name.
        name: String,
    },
    /// Show a pairing QR code and wait for another device to join.
    Pair,
    /// Pair with a device using the invite link it shows.
    Join {
        /// The invite link, starting with `ecpair:`.
        invite: String,
    },
    /// List paired devices.
    Devices,
    /// Remove a paired device.
    Forget {
        /// The device id, or a unique prefix of it.
        device: String,
    },
    /// List Ecosystem Connector devices on the local network.
    Discover {
        /// How long to look, in seconds.
        #[arg(long, default_value_t = 3)]
        seconds: u64,
    },
    /// Stay online: announce this device, answer paired devices and receive files.
    Serve {
        /// UDP port to listen on (0 picks a free port).
        #[arg(long, default_value_t = ec_net::DEFAULT_PORT)]
        port: u16,
        /// Folder for received files [default: Downloads/Ecosystem Connector].
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Send files or folders to a paired device that is running `ec serve`.
    Send {
        /// The device id, or a unique prefix of it.
        device: String,
        /// Files or folders to send.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Chunks in flight per network path.
        #[arg(long, default_value_t = 8)]
        streams: usize,
    },
    /// Fling a web page to a paired device, which opens it in its browser.
    Fling {
        /// The device id, or a unique prefix of it.
        device: String,
        /// The page address (http or https).
        url: String,
        /// The page title.
        #[arg(long, default_value = "")]
        title: String,
    },
    /// Connect to a paired device and measure the round-trip time.
    Ping {
        /// The device id, or a unique prefix of it.
        device: String,
        /// Number of pings.
        #[arg(long, default_value_t = 4)]
        count: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("EC_LOG").unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let home = match cli.home {
        Some(home) => home,
        None => default_home()?,
    };
    let default_name = gethostname::gethostname().to_string_lossy().into_owned();
    let mut identity = DeviceIdentity::load_or_create(&home, &default_name)
        .with_context(|| format!("loading identity from {}", home.display()))?;
    let mut trust = TrustStore::load(&home)?;

    match cli.command {
        Command::Id => {
            println!("Name: {}", identity.name());
            println!("Id:   {} ({})", identity.id().short(), identity.id());
            println!("Home: {}", home.display());
        }
        Command::Rename { name } => {
            identity.rename(&home, &name)?;
            println!("Renamed to {}", identity.name());
        }
        Command::Pair => pair(Arc::new(identity), &mut trust).await?,
        Command::Join { invite } => {
            let invite = PairingInvite::parse(&invite)?;
            println!("Connecting...");
            // Tell the other device where `ec serve` listens by default.
            let paired =
                pair_with(Arc::new(identity), &invite, ec_net::DEFAULT_PORT, ask_user).await?;
            save_paired(&mut trust, paired)?;
        }
        Command::Devices => {
            if trust.devices().next().is_none() {
                println!(
                    "No paired devices. Run `ec pair` on one device and `ec join` on the other."
                );
            }
            for device in trust.devices() {
                println!("{}  {}", device.id.short(), device.name);
            }
        }
        Command::Forget { device } => {
            let device = find(&trust, &device)?.clone();
            trust.remove(&device.id)?;
            println!("Forgot {}", device.name);
        }
        Command::Discover { seconds } => {
            let me = identity.id();
            let found = browse(Duration::from_secs(seconds)).await?;
            let others: Vec<_> = found.into_iter().filter(|d| d.id != me).collect();
            if others.is_empty() {
                println!("No devices found. Is `ec serve` running on the other device?");
            }
            for device in others {
                let status = if trust.contains(&device.id) {
                    "paired"
                } else {
                    "not paired"
                };
                let addrs: Vec<String> = device.addrs.iter().map(|a| a.to_string()).collect();
                println!(
                    "{}  {:<24} {:<11} {}",
                    device.id.short(),
                    device.name,
                    status,
                    addrs.join(", ")
                );
            }
        }
        Command::Serve { port, dir } => {
            let dir = match dir {
                Some(dir) => dir,
                None => default_receive_dir()?,
            };
            serve(Arc::new(identity), &trust, port, dir).await?
        }
        Command::Send {
            device,
            paths,
            streams,
        } => {
            let device = find(&trust, &device)?.clone();
            send(Arc::new(identity), &trust, &device, paths, streams).await?;
        }
        Command::Fling { device, url, title } => {
            let device = find(&trust, &device)?.clone();
            let addrs = addrs_for(&device, Duration::from_secs(3)).await?;
            let node = Node::bind(
                Arc::new(identity),
                trust.devices().map(|d| d.id),
                SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
            )?;
            let conn = node.connect(device.id, &addrs).await?;
            ec_fling::fling(&conn, ec_fling::FlingItem::Url { url, title }).await?;
            println!("Flung to {}.", device.name);
            node.shutdown().await;
        }
        Command::Ping { device, count } => {
            let device = find(&trust, &device)?.clone();
            ping(Arc::new(identity), &trust, &device, count).await?;
        }
    }
    Ok(())
}

fn default_home() -> Result<PathBuf> {
    let base = if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    let base = base.context("cannot find a data folder; pass --home")?;
    Ok(base.join("EcosystemConnector"))
}

fn default_receive_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .context("cannot find the home folder; pass --dir")?;
    Ok(PathBuf::from(home)
        .join("Downloads")
        .join("Ecosystem Connector"))
}

fn find<'a>(trust: &'a TrustStore, prefix: &str) -> Result<&'a TrustedDevice> {
    trust
        .find_by_prefix(prefix)
        .with_context(|| format!("no single paired device matches `{prefix}`; see `ec devices`"))
}

async fn pair(identity: Arc<DeviceIdentity>, trust: &mut TrustStore) -> Result<()> {
    let listener = PairingListener::bind(
        identity,
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        ec_net::DEFAULT_PORT,
    )?;
    let invite = listener.invite(&local_addrs())?.to_uri();

    let qr = qrcode::QrCode::new(invite.as_bytes())?;
    let image = qr
        .render::<qrcode::render::unicode::Dense1x2>()
        .quiet_zone(true)
        .build();
    println!("{image}");
    println!("Scan the code, or run this on the other device:\n");
    println!("  ec join '{invite}'\n");
    println!("Waiting up to 2 minutes. The invite works once.");

    let paired = listener.accept(ask_user).await?;
    save_paired(trust, paired)
}

fn save_paired(trust: &mut TrustStore, paired: ec_net::pairing::PairedDevice) -> Result<()> {
    let device = trust.add(paired.id, &paired.name)?;
    if let Some(addr) = paired.addr {
        trust.set_addrs(&device.id, &[addr])?;
    }
    println!("Paired with {} ({}).", device.name, device.id.short());
    Ok(())
}

/// Shows the verification code and asks the user to compare it with the other device.
async fn ask_user(prompt: PairingPrompt) -> bool {
    let question = format!(
        "\n{} ({}) wants to pair.\nDoes the other device show the code {}? [y/N] ",
        prompt.peer_name,
        prompt.peer.short(),
        prompt.code
    );
    tokio::task::spawn_blocking(move || {
        print!("{question}");
        let _ = std::io::stdout().flush();
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer).is_ok()
            && matches!(answer.trim(), "y" | "Y" | "yes" | "e" | "E" | "evet")
    })
    .await
    .unwrap_or(false)
}

async fn serve(
    identity: Arc<DeviceIdentity>,
    trust: &TrustStore,
    port: u16,
    dir: PathBuf,
) -> Result<()> {
    let trusted = trust.devices().map(|d| d.id);
    let node = Arc::new(Node::bind(
        identity.clone(),
        trusted,
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)),
    )?);
    let port = node.local_addr()?.port();
    let _advertiser = Advertiser::start(&identity, port)?;

    let mut router = Router::with_session(identity.name(), |id, name| {
        println!("  {} says hello as \"{name}\"", id.short());
    });
    // Only paired devices can connect at all, so their transfers are accepted.
    let accept: ec_transfer::AcceptPolicy = Arc::new(|offer: &ec_transfer::Offer| {
        println!(
            "Receiving {} file(s), {} from {} (first: {})",
            offer.files,
            human_bytes(offer.total_bytes),
            offer.peer.short(),
            offer.first_path
        );
        true
    });
    let progress = Progress::new();
    let events: ec_transfer::ReceiveEvents = Arc::new(move |event| match event {
        ReceiveEvent::Started { already_have, .. } if already_have > 0 => {
            println!("  resuming: {} already here", human_bytes(already_have));
        }
        ReceiveEvent::Progress { received, total } => progress.show(received, total, None),
        ReceiveEvent::Saved { path, open } => {
            let open = if open.is_some() {
                " (sender asked to open it)"
            } else {
                ""
            };
            println!("\r  saved {}{open}", path.display())
        }
        ReceiveEvent::Interrupted { reason } => {
            println!("\r  interrupted ({reason}); it will resume when sent again")
        }
        _ => {}
    });
    let receiver = Arc::new(Receiver::new(dir.clone(), accept, events));
    router.add(ec_transfer::CONTROL_KIND, receiver.clone())?;
    router.add(ec_transfer::CHUNK_KIND, receiver)?;
    router.add(
        ec_fling::FLING_KIND,
        Arc::new(ec_fling::FlingService::new(Arc::new(
            |from, item| match item {
                ec_fling::FlingItem::Url { url, title } => {
                    println!("{} flung a page: {title} {url}", from.short())
                }
            },
        ))),
    )?;
    let router = Arc::new(router);

    println!(
        "{} ({}) is online on UDP port {port}. Press Ctrl-C to stop.",
        identity.name(),
        identity.id().short()
    );
    println!("Received files go to {}", dir.display());
    println!("Pairings made while this runs take effect after a restart.");

    let accept_loop = {
        let node = node.clone();
        async move {
            while let Some(conn) = node.accept().await {
                match conn {
                    Ok(conn) => {
                        tracing::info!(peer = %conn.peer().short(), addr = %conn.remote_addr(), "connected");
                        tokio::spawn(serve_connection(conn, router.clone()));
                    }
                    Err(e) => println!("Refused a connection: {e}"),
                }
            }
        }
    };
    tokio::select! {
        () = accept_loop => {}
        _ = tokio::signal::ctrl_c() => println!("\nStopping."),
    }
    node.shutdown().await;
    Ok(())
}

async fn send(
    identity: Arc<DeviceIdentity>,
    trust: &TrustStore,
    device: &TrustedDevice,
    paths: Vec<PathBuf>,
    streams: usize,
) -> Result<()> {
    println!("Looking for {} on the network...", device.name);
    let addrs = addrs_for(device, Duration::from_secs(3)).await?;
    if addrs.is_empty() {
        bail!(
            "{} was not found. Is `ec serve` running on it?",
            device.name
        );
    }
    let node = Arc::new(Node::bind(
        identity,
        trust.devices().map(|d| d.id),
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
    )?);

    let peer = device.id;
    let rediscover: ec_transfer::AddrSource = Arc::new(move || {
        Box::pin(async move {
            find_addrs(peer, Duration::from_millis(1500))
                .await
                .unwrap_or_default()
        })
    });
    let options = SendOptions {
        streams_per_path: streams,
        rediscover: Some(rediscover),
        ..SendOptions::default()
    };
    let progress = Progress::new();
    let events: ec_transfer::SendEvents = Arc::new(move |event| match event {
        SendEvent::Accepted { to_send, total } if to_send < total => println!(
            "Resuming: {} of {} already on the other device.",
            human_bytes(total - to_send),
            human_bytes(total)
        ),
        SendEvent::PathAdded { addr } => println!("\r  path: {addr}"),
        SendEvent::PathLost { addr } => println!("\r  path lost: {addr}"),
        SendEvent::Progress {
            sent,
            to_send,
            paths,
        } => progress.show(sent, to_send, Some(paths)),
        SendEvent::Retrying { attempt, reason } => {
            println!("\r  connection lost ({reason}), retrying (attempt {attempt})")
        }
        _ => {}
    });

    println!("Hashing...");
    let report = send_paths(node.clone(), peer, addrs, paths, options, events).await?;
    let secs = report.elapsed.as_secs_f64().max(0.001);
    println!(
        "\nSent {} file(s), {} in {:.1} s ({}/s over {} path(s)).",
        report.files,
        human_bytes(report.bytes_sent),
        secs,
        human_bytes((report.bytes_sent as f64 / secs) as u64),
        report.paths_used
    );
    node.shutdown().await;
    Ok(())
}

/// Addresses to try for a paired device: what mDNS finds, then where it was reachable
/// before (for networks that block mDNS).
async fn addrs_for(device: &TrustedDevice, wait: Duration) -> Result<Vec<SocketAddr>> {
    let mut addrs = find_addrs(device.id, wait).await?;
    for addr in &device.addrs {
        if !addrs.contains(addr) {
            addrs.push(*addr);
        }
    }
    Ok(addrs)
}

async fn find_addrs(peer: ec_core::DeviceId, wait: Duration) -> Result<Vec<SocketAddr>> {
    Ok(browse(wait)
        .await?
        .into_iter()
        .find(|d| d.id == peer)
        .map(|d| d.addrs)
        .unwrap_or_default())
}

/// Prints a one-line progress indicator, at most a few times per second.
struct Progress {
    start: std::time::Instant,
    last: std::sync::Mutex<std::time::Instant>,
}

impl Progress {
    fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            start: now,
            last: std::sync::Mutex::new(now),
        }
    }

    fn show(&self, done: u64, total: u64, paths: Option<usize>) {
        let now = std::time::Instant::now();
        {
            let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
            if done < total && now.duration_since(*last) < Duration::from_millis(200) {
                return;
            }
            *last = now;
        }
        let secs = now.duration_since(self.start).as_secs_f64().max(0.001);
        let percent = if total == 0 {
            100.0
        } else {
            done as f64 * 100.0 / total as f64
        };
        let paths = paths.map(|p| format!(", {p} path(s)")).unwrap_or_default();
        print!(
            "\r  {percent:5.1}%  {} / {}  {}/s{paths}    ",
            human_bytes(done),
            human_bytes(total),
            human_bytes((done as f64 / secs) as u64)
        );
        let _ = std::io::stdout().flush();
    }
}

fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

async fn ping(
    identity: Arc<DeviceIdentity>,
    trust: &TrustStore,
    device: &TrustedDevice,
    count: u32,
) -> Result<()> {
    println!("Looking for {} on the network...", device.name);
    let addrs = addrs_for(device, Duration::from_secs(3)).await?;
    if addrs.is_empty() {
        bail!(
            "{} was not found. Is `ec serve` running on it?",
            device.name
        );
    }

    let node = Node::bind(
        identity.clone(),
        trust.devices().map(|d| d.id),
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
    )?;
    let conn = node.connect(device.id, &addrs).await?;
    let mut session = conn.open_session(identity.name()).await?;
    println!(
        "Connected to \"{}\" at {} (encrypted, identity verified).",
        session.peer_name(),
        conn.remote_addr()
    );
    for n in 1..=count {
        let rtt = session.ping().await?;
        println!("  ping {n}: {:.2} ms", rtt.as_secs_f64() * 1000.0);
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    session.finish().await;
    conn.close();
    node.shutdown().await;
    Ok(())
}
