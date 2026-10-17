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
use ec_net::{Advertiser, Node, browse, local_addrs, serve_connection};

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
    /// Stay online: announce this device and answer paired devices.
    Serve {
        /// UDP port to listen on (0 picks a free port).
        #[arg(long, default_value_t = 0)]
        port: u16,
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
            let paired = pair_with(Arc::new(identity), &invite, ask_user).await?;
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
        Command::Serve { port } => serve(Arc::new(identity), &trust, port).await?,
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

fn find<'a>(trust: &'a TrustStore, prefix: &str) -> Result<&'a TrustedDevice> {
    trust
        .find_by_prefix(prefix)
        .with_context(|| format!("no single paired device matches `{prefix}`; see `ec devices`"))
}

async fn pair(identity: Arc<DeviceIdentity>, trust: &mut TrustStore) -> Result<()> {
    let listener = PairingListener::bind(identity, SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?;
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

async fn serve(identity: Arc<DeviceIdentity>, trust: &TrustStore, port: u16) -> Result<()> {
    let trusted = trust.devices().map(|d| d.id);
    let node = Arc::new(Node::bind(
        identity.clone(),
        trusted,
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)),
    )?);
    let port = node.local_addr()?.port();
    let _advertiser = Advertiser::start(&identity, port)?;
    println!(
        "{} ({}) is online on UDP port {port}. Press Ctrl-C to stop.",
        identity.name(),
        identity.id().short()
    );
    println!("Pairings made while this runs take effect after a restart.");

    let accept_loop = {
        let node = node.clone();
        let name = identity.name().to_owned();
        async move {
            while let Some(conn) = node.accept().await {
                match conn {
                    Ok(conn) => {
                        println!(
                            "Connection from {} at {}",
                            conn.peer().short(),
                            conn.remote_addr()
                        );
                        tokio::spawn(serve_connection(conn, name.clone(), |id, name| {
                            println!("  {} says hello as \"{name}\"", id.short());
                        }));
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

async fn ping(
    identity: Arc<DeviceIdentity>,
    trust: &TrustStore,
    device: &TrustedDevice,
    count: u32,
) -> Result<()> {
    println!("Looking for {} on the network...", device.name);
    let found = browse(Duration::from_secs(3)).await?;
    let Some(target) = found.into_iter().find(|d| d.id == device.id) else {
        bail!(
            "{} was not found. Is `ec serve` running on it?",
            device.name
        );
    };

    let node = Node::bind(
        identity.clone(),
        trust.devices().map(|d| d.id),
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
    )?;
    let conn = node.connect(device.id, &target.addrs).await?;
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
