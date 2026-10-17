//! A running device, as seen from Dart.

use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};

use ec_core::DeviceId;
use ec_runtime::{Config, Event, Runtime};
use ec_transfer::{ReceiveEvent, SendEvent};
use flutter_rust_bridge::frb;
use tokio::sync::oneshot;

use crate::frb_generated::StreamSink;

/// The runtime all Rust work runs on, independent of Dart's event loop.
static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("ec-core")
        .build()
        .unwrap_or_else(|e| panic!("cannot start the async runtime: {e}"))
});

/// The core started in this process, with its home folder.
static RUNNING: Mutex<Option<(String, Arc<Runtime>)>> = Mutex::new(None);

fn running(home: &str) -> Option<Arc<Runtime>> {
    let guard = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .filter(|(h, _)| h == home)
        .map(|(_, r)| r.clone())
}

/// Sets up logging and panic messages for Dart. Called automatically by the bridge.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Where the core keeps its data.
pub struct CoreConfig {
    /// Folder for the identity and paired devices (app-private).
    pub home: String,
    /// Name for this device when it is set up for the first time.
    pub device_name: String,
    /// Folder for received files.
    pub receive_dir: String,
}

/// A paired device.
pub struct DeviceView {
    /// Full id (64 hex characters).
    pub id: String,
    /// Short id for display, such as `3F9A-C210`.
    pub short_id: String,
    /// Its name.
    pub name: String,
    /// Whether it is on the network now.
    pub online: bool,
}

/// Background events for the UI.
#[derive(Clone)]
pub enum CoreEvent {
    /// Read `devices()` again.
    DevicesChanged,
    /// A device was paired.
    Paired {
        /// Its name.
        name: String,
    },
    /// Files are arriving.
    ReceiveStarted {
        /// Short id of the sender.
        from: String,
        /// Number of files.
        files: u32,
        /// Total size.
        total_bytes: u64,
        /// Bytes already here from an interrupted attempt.
        already_have: u64,
    },
    /// Receive progress.
    ReceiveProgress {
        /// Bytes received.
        received: u64,
        /// Total bytes.
        total: u64,
    },
    /// A file was saved.
    Saved {
        /// Its path.
        path: String,
    },
    /// A transfer finished.
    ReceiveFinished,
    /// A transfer stopped early; it resumes when sent again.
    ReceiveInterrupted {
        /// Why.
        reason: String,
    },
}

/// Steps of pairing.
#[derive(Clone)]
pub enum PairingEvent {
    /// Show this invite as a QR code (when hosting).
    Invite {
        /// The `ecpair:` link.
        uri: String,
    },
    /// Ask the user to compare the code, then call `answer_pairing`.
    Confirm {
        /// The other device's name.
        peer_name: String,
        /// Its short id.
        short_id: String,
        /// The six-digit code, as `"042 917"`.
        code: String,
    },
    /// Pairing finished.
    Paired {
        /// The other device's name.
        name: String,
    },
    /// Pairing failed.
    Failed {
        /// Why.
        reason: String,
    },
}

/// Progress of a send.
#[derive(Clone)]
pub enum SendUpdate {
    /// Bytes acknowledged by the receiver.
    Progress {
        /// Sent so far.
        sent: u64,
        /// To send in this attempt.
        to_send: u64,
        /// Network paths in use.
        paths: u32,
    },
    /// A connection was lost and the send is retrying.
    Retrying {
        /// Why.
        reason: String,
    },
    /// Finished.
    Done {
        /// Number of files.
        files: u32,
        /// Bytes sent.
        bytes: u64,
        /// Duration in milliseconds.
        millis: u64,
        /// Paths used.
        paths: u32,
    },
    /// Failed.
    Failed {
        /// Why.
        reason: String,
    },
}

/// A running device. Create it once with [`EcCore::start`].
pub struct EcCore {
    runtime: Arc<Runtime>,
    pending_answer: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
}

impl EcCore {
    /// Starts the device: identity, network presence and receiving.
    ///
    /// The core outlives the Dart side: on Android the activity (and its Dart isolate)
    /// can be recreated while the process keeps receiving in the background. Starting
    /// again with the same home returns the running core instead of a second one.
    pub async fn start(config: CoreConfig) -> anyhow::Result<EcCore> {
        if let Some(runtime) = running(&config.home) {
            return Ok(Self::wrap(runtime));
        }
        let home = config.home.clone();
        let config = Config {
            home: PathBuf::from(config.home),
            default_name: config.device_name,
            receive_dir: PathBuf::from(config.receive_dir),
            port: ec_runtime::DEFAULT_PORT,
            mdns: true,
        };
        let runtime = RT.spawn(Runtime::start(config)).await??;
        *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = Some((home, runtime.clone()));
        Ok(Self::wrap(runtime))
    }

    fn wrap(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            pending_answer: Arc::default(),
        }
    }

    /// This device's full id.
    #[frb(sync)]
    pub fn my_id(&self) -> String {
        self.runtime.id().to_string()
    }

    /// This device's short id.
    #[frb(sync)]
    pub fn my_short_id(&self) -> String {
        self.runtime.id().short()
    }

    /// This device's name.
    #[frb(sync)]
    pub fn my_name(&self) -> String {
        self.runtime.name().to_owned()
    }

    /// Paired devices.
    #[frb(sync)]
    pub fn devices(&self) -> Vec<DeviceView> {
        self.runtime
            .devices()
            .into_iter()
            .map(|d| DeviceView {
                id: d.id.to_string(),
                short_id: d.id.short(),
                name: d.name,
                online: d.online,
            })
            .collect()
    }

    /// Streams background events until the stream is closed on the Dart side.
    pub fn events(&self, sink: StreamSink<CoreEvent>) {
        let mut events = self.runtime.subscribe();
        RT.spawn(async move {
            loop {
                let event = match events.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                };
                if let Some(event) = convert(event)
                    && sink.add(event).is_err()
                {
                    return;
                }
            }
        });
    }

    /// Hosts pairing: emits an invite to show, then asks for confirmation.
    pub fn host_pairing(&self, sink: StreamSink<PairingEvent>) {
        let runtime = self.runtime.clone();
        let pending = self.pending_answer.clone();
        RT.spawn(async move {
            let invite_sink = sink.clone();
            let result = runtime
                .host_pairing(
                    move |uri| {
                        let _ = invite_sink.add(PairingEvent::Invite { uri });
                    },
                    confirm_through(sink.clone(), pending),
                )
                .await;
            finish_pairing(&sink, result.map(|d| d.name));
        });
    }

    /// Joins the device that shows `invite`.
    pub fn join_pairing(&self, invite: String, sink: StreamSink<PairingEvent>) {
        let runtime = self.runtime.clone();
        let pending = self.pending_answer.clone();
        RT.spawn(async move {
            let result = runtime
                .join_pairing(&invite, confirm_through(sink.clone(), pending))
                .await;
            finish_pairing(&sink, result.map(|d| d.name));
        });
    }

    /// The user's answer to the last [`PairingEvent::Confirm`].
    #[frb(sync)]
    pub fn answer_pairing(&self, accept: bool) {
        let sender = self
            .pending_answer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(sender) = sender {
            let _ = sender.send(accept);
        }
    }

    /// Sends files and folders to a paired device.
    pub fn send(&self, device_id: String, paths: Vec<String>, sink: StreamSink<SendUpdate>) {
        let runtime = self.runtime.clone();
        RT.spawn(async move {
            let id = match device_id.parse::<DeviceId>() {
                Ok(id) => id,
                Err(e) => {
                    let _ = sink.add(SendUpdate::Failed {
                        reason: e.to_string(),
                    });
                    return;
                }
            };
            let progress = sink.clone();
            let events: ec_transfer::SendEvents = Arc::new(move |event| {
                let update = match event {
                    SendEvent::Progress {
                        sent,
                        to_send,
                        paths,
                    } => SendUpdate::Progress {
                        sent,
                        to_send,
                        paths: paths as u32,
                    },
                    SendEvent::Retrying { reason, .. } => SendUpdate::Retrying { reason },
                    _ => return,
                };
                let _ = progress.add(update);
            });
            let paths = paths.into_iter().map(PathBuf::from).collect();
            let update = match runtime.send(id, paths, events).await {
                Ok(report) => SendUpdate::Done {
                    files: report.files as u32,
                    bytes: report.bytes_sent,
                    millis: report.elapsed.as_millis() as u64,
                    paths: report.paths_used as u32,
                },
                Err(e) => SendUpdate::Failed {
                    reason: e.to_string(),
                },
            };
            let _ = sink.add(update);
        });
    }

    /// Removes a paired device.
    #[frb(sync)]
    pub fn forget(&self, device_id: String) -> anyhow::Result<()> {
        let id: DeviceId = device_id.parse()?;
        self.runtime.forget(&id)?;
        Ok(())
    }
}

/// A pairing confirmation that asks Dart and waits for [`EcCore::answer_pairing`].
fn confirm_through(
    sink: StreamSink<PairingEvent>,
    pending: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
) -> impl FnOnce(ec_net::pairing::PairingPrompt) -> std::pin::Pin<Box<dyn Future<Output = bool> + Send>>
{
    move |prompt| {
        let (tx, rx) = oneshot::channel();
        *pending.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
        let _ = sink.add(PairingEvent::Confirm {
            peer_name: prompt.peer_name,
            short_id: prompt.peer.short(),
            code: prompt.code,
        });
        Box::pin(async move { rx.await.unwrap_or(false) })
    }
}

fn finish_pairing(sink: &StreamSink<PairingEvent>, result: ec_runtime::Result<String>) {
    let event = match result {
        Ok(name) => PairingEvent::Paired { name },
        Err(e) => PairingEvent::Failed {
            reason: e.to_string(),
        },
    };
    let _ = sink.add(event);
}

fn convert(event: Event) -> Option<CoreEvent> {
    Some(match event {
        Event::DevicesChanged => CoreEvent::DevicesChanged,
        Event::Paired { name, .. } => CoreEvent::Paired { name },
        Event::Receive(event) => match event {
            ReceiveEvent::Started {
                peer,
                files,
                total_bytes,
                already_have,
            } => CoreEvent::ReceiveStarted {
                from: peer.short(),
                files: files as u32,
                total_bytes,
                already_have,
            },
            ReceiveEvent::Progress { received, total } => {
                CoreEvent::ReceiveProgress { received, total }
            }
            ReceiveEvent::Saved { path } => CoreEvent::Saved {
                path: path.to_string_lossy().into_owned(),
            },
            ReceiveEvent::Finished => CoreEvent::ReceiveFinished,
            ReceiveEvent::Interrupted { reason } => CoreEvent::ReceiveInterrupted { reason },
        },
    })
}
