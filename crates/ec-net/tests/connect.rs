//! End-to-end tests over real QUIC connections on the loopback interface.

#![allow(clippy::unwrap_used)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use ec_core::{DeviceId, DeviceIdentity, PairingSecret};
use std::future::{Ready, ready};

use ec_net::pairing::{PairingListener, PairingPrompt, pair_with};
use ec_net::{Error, Node, serve_connection};
use tokio::sync::oneshot;

const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

fn identity(name: &str) -> Arc<DeviceIdentity> {
    Arc::new(DeviceIdentity::generate(name).unwrap())
}

fn any_port() -> SocketAddr {
    SocketAddr::new(LOCALHOST, 0)
}

/// Returns a confirm callback that answers `answer` and reports the code it was shown.
fn confirm_with(
    answer: bool,
) -> (
    impl FnOnce(PairingPrompt) -> Ready<bool>,
    oneshot::Receiver<String>,
) {
    let (tx, rx) = oneshot::channel();
    let confirm = move |prompt: PairingPrompt| {
        let _ = tx.send(prompt.code);
        ready(answer)
    };
    (confirm, rx)
}

#[tokio::test]
async fn pairing_succeeds_and_both_sides_see_the_same_code() {
    let (mac, phone) = (identity("Mac"), identity("Phone"));
    let listener = PairingListener::bind(mac.clone(), any_port()).unwrap();
    let invite = listener.invite(&[LOCALHOST]).unwrap();

    let (mac_confirm, mac_code) = confirm_with(true);
    let responder = tokio::spawn(async move { listener.accept(mac_confirm).await });
    let (phone_confirm, phone_code) = confirm_with(true);
    let paired_mac = pair_with(phone.clone(), &invite, phone_confirm)
        .await
        .unwrap();
    let paired_phone = responder.await.unwrap().unwrap();

    assert_eq!(paired_mac.id, mac.id());
    assert_eq!(paired_mac.name, "Mac");
    assert_eq!(paired_phone.id, phone.id());
    assert_eq!(paired_phone.name, "Phone");
    assert_eq!(mac_code.await.unwrap(), phone_code.await.unwrap());
}

#[tokio::test]
async fn pairing_with_wrong_secret_is_rejected_and_closes_the_window() {
    let (mac, phone) = (identity("Mac"), identity("Phone"));
    let listener = PairingListener::bind(mac, any_port()).unwrap();
    let mut invite = listener.invite(&[LOCALHOST]).unwrap();
    invite.secret = PairingSecret::generate();

    let (mac_confirm, _) = confirm_with(true);
    let responder = tokio::spawn(async move { listener.accept(mac_confirm).await });
    let (phone_confirm, _) = confirm_with(true);
    let initiator = pair_with(phone, &invite, phone_confirm).await;

    assert!(
        matches!(initiator, Err(Error::PairingFailed(_))),
        "{initiator:?}"
    );
    assert!(matches!(
        responder.await.unwrap(),
        Err(Error::PairingFailed(_))
    ));
}

#[tokio::test]
async fn pairing_fails_when_one_user_declines() {
    let (mac, phone) = (identity("Mac"), identity("Phone"));
    let listener = PairingListener::bind(mac, any_port()).unwrap();
    let invite = listener.invite(&[LOCALHOST]).unwrap();

    let (mac_confirm, _) = confirm_with(false);
    let responder = tokio::spawn(async move { listener.accept(mac_confirm).await });
    let (phone_confirm, _) = confirm_with(true);
    let initiator = pair_with(phone, &invite, phone_confirm).await;

    assert!(
        matches!(initiator, Err(Error::PairingFailed(_))),
        "{initiator:?}"
    );
    assert!(matches!(
        responder.await.unwrap(),
        Err(Error::PairingFailed(_))
    ));
}

#[tokio::test]
async fn pairing_refuses_a_listener_with_another_identity() {
    // The invite names a device id that the listener does not have, as if an attacker
    // answered on the invite's address. TLS pinning must refuse it, and the real
    // invite must still work afterwards.
    let (mac, phone) = (identity("Mac"), identity("Phone"));
    let listener = PairingListener::bind(mac, any_port()).unwrap();
    let invite = listener.invite(&[LOCALHOST]).unwrap();
    let mut forged = invite.clone();
    forged.responder = DeviceId::from_bytes([9; 32]);

    let (mac_confirm, _) = confirm_with(true);
    let responder = tokio::spawn(async move { listener.accept(mac_confirm).await });

    let (c, _) = confirm_with(true);
    let attempt = pair_with(phone.clone(), &forged, c).await;
    assert!(matches!(attempt, Err(Error::Unreachable)), "{attempt:?}");

    let (c, _) = confirm_with(true);
    pair_with(phone, &invite, c).await.unwrap();
    responder.await.unwrap().unwrap();
}

/// Two nodes that trust each other, with `b` answering sessions.
fn paired_nodes() -> (Node, Arc<Node>, Arc<DeviceIdentity>) {
    let (a_id, b_id) = (identity("A"), identity("B"));
    let a = Node::bind(a_id.clone(), [b_id.id()], any_port()).unwrap();
    let b = Arc::new(Node::bind(b_id.clone(), [a_id.id()], any_port()).unwrap());
    (a, b, b_id)
}

fn serve(node: Arc<Node>) {
    tokio::spawn(async move {
        while let Some(conn) = node.accept().await {
            if let Ok(conn) = conn {
                tokio::spawn(serve_connection(conn, "B".into(), |_, _| {}));
            }
        }
    });
}

#[tokio::test]
async fn paired_nodes_connect_and_ping() {
    let (a, b, b_id) = paired_nodes();
    let b_addr = b.local_addr().unwrap();
    serve(b);

    let conn = a.connect(b_id.id(), &[b_addr]).await.unwrap();
    assert_eq!(conn.peer(), b_id.id());
    let mut session = conn.open_session("A").await.unwrap();
    assert_eq!(session.peer_name(), "B");
    for _ in 0..3 {
        session.ping().await.unwrap();
    }
    session.finish().await;
}

#[tokio::test]
async fn unpaired_client_is_refused() {
    let (a_id, b_id) = (identity("A"), identity("B"));
    // A trusts B, but B does not trust A.
    let a = Node::bind(a_id, [b_id.id()], any_port()).unwrap();
    let b = Arc::new(Node::bind(b_id.clone(), [], any_port()).unwrap());
    let b_addr = b.local_addr().unwrap();
    serve(b);

    let result = async {
        a.connect(b_id.id(), &[b_addr])
            .await?
            .open_session("A")
            .await
    }
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn connecting_to_an_impostor_fails() {
    // A believes device X lives at B's address; B's certificate is not X's.
    let (a, b, _) = paired_nodes();
    let impostor_addr = b.local_addr().unwrap();
    let x = DeviceId::from_bytes([3; 32]);
    a.trust(x);
    serve(b);

    let result = a.connect(x, &[impostor_addr]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn connecting_to_an_unpaired_device_is_refused_locally() {
    let (a, _b, _) = paired_nodes();
    let stranger = DeviceId::from_bytes([4; 32]);
    let result = a.connect(stranger, &[any_port()]).await;
    assert!(matches!(result, Err(Error::NotPaired(_))));
}
