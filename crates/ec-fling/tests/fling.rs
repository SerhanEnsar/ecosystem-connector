//! Flings over real QUIC connections.

#![allow(clippy::unwrap_used)]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use ec_core::DeviceIdentity;
use ec_fling::{FLING_KIND, FlingAck, FlingItem, FlingService, fling};
use ec_net::{Node, PeerConnection, Router, read_msg, serve_connection, write_msg};

type Received = Arc<Mutex<Vec<FlingItem>>>;

/// A sender connection to a receiver that records accepted flings.
async fn setup() -> (PeerConnection, Received) {
    let a = Arc::new(DeviceIdentity::generate("Tablet").unwrap());
    let b = Arc::new(DeviceIdentity::generate("Mac").unwrap());
    let local = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let sender = Node::bind(a.clone(), [b.id()], local).unwrap();
    let receiver = Arc::new(Node::bind(b.clone(), [a.id()], local).unwrap());

    let received: Received = Arc::default();
    let sink = received.clone();
    let mut router = Router::with_session("Mac", |_, _| {});
    router
        .add(
            FLING_KIND,
            Arc::new(FlingService::new(Arc::new(move |_, item| {
                sink.lock().unwrap().push(item)
            }))),
        )
        .unwrap();
    let router = Arc::new(router);
    let addr = receiver.local_addr().unwrap();
    tokio::spawn(async move {
        while let Some(Ok(conn)) = receiver.accept().await {
            tokio::spawn(serve_connection(conn, router.clone()));
        }
    });
    let conn = sender.connect(b.id(), &[addr]).await.unwrap();
    // Keep the sender node alive for the test.
    std::mem::forget(sender);
    (conn, received)
}

#[tokio::test]
async fn a_web_page_arrives() {
    let (conn, received) = setup().await;
    let page = FlingItem::Url {
        url: "https://en.wikipedia.org/wiki/QUIC".into(),
        title: "QUIC - Wikipedia".into(),
    };
    fling(&conn, page.clone()).await.unwrap();
    assert_eq!(*received.lock().unwrap(), vec![page]);
}

#[tokio::test]
async fn the_sender_refuses_unsafe_links_before_sending() {
    let (conn, received) = setup().await;
    let item = FlingItem::Url {
        url: "file:///etc/passwd".into(),
        title: String::new(),
    };
    assert!(fling(&conn, item).await.is_err());
    assert!(received.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_receiver_refuses_unsafe_links_from_a_modified_sender() {
    // A peer that skips validation and writes the item itself.
    let (conn, received) = setup().await;
    let (mut send, mut recv) = conn.open_bi(FLING_KIND).await.unwrap();
    let item = FlingItem::Url {
        url: "javascript:alert(1)".into(),
        title: String::new(),
    };
    write_msg(&mut send, &item).await.unwrap();
    let ack: FlingAck = read_msg(&mut recv).await.unwrap();
    assert!(matches!(ack, FlingAck::Refused(_)));
    assert!(received.lock().unwrap().is_empty());
}
