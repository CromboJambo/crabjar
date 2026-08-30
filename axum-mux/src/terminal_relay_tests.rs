//! Live round-trip tests for the terminal relay (ADR-005, item 5).
//!
//! These tests spin up a real relay on an ephemeral port and drive it with
//! a real WebSocket client (`tokio-tungstenite`), so they exercise the full
//! wire path: upgrade, join, fan-out, and cleanup.
use crate::terminal_relay::{start, TerminalRelayState};
use crabjar_terminal::TerminalEvent;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

/// Read the next non-close, non-pong message from a WebSocket, with a
/// timeout. Returns the raw tungstenite message.
async fn read_msg(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> WsMessage {
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
            .await
            .expect("timed out waiting for message")
            .expect("stream ended");
        match msg {
            Ok(WsMessage::Close(_)) => panic!("unexpected close"),
            Ok(WsMessage::Pong(_)) => continue,
            Ok(other) => return other,
            Err(e) => panic!("websocket read error: {e}"),
        }
    }
}

/// Assert that a message is a text frame whose JSON has the given `"type"`.
fn assert_type(msg: &WsMessage, expected: &str) {
    if let WsMessage::Text(text) = msg {
        let v: serde_json::Value = serde_json::from_str(text).expect("not JSON");
        assert_eq!(
            v.get("type").and_then(|t| t.as_str()),
            Some(expected),
            "expected type {expected}, got: {text}"
        );
    } else {
        panic!("expected text frame, got: {msg:?}");
    }
}

/// Connect a WebSocket client to the relay and join a session.
async fn connect_and_join(
    url: &str,
    session: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (mut ws, _) = connect_async(url).await.expect("connect failed");
    // Read the initial pong.
    let _ = read_msg(&mut ws).await;
    // Send join.
    let join = serde_json::json!({ "type": "join", "session": session }).to_string();
    ws.send(WsMessage::Text(join)).await.expect("send join");
    // Settle: the server processes the join asynchronously; a short
    // delay guarantees the receiver is subscribed before the test
    // publishes (avoids a race where an early publish misses the
    // subscriber).
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    ws
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_fans_out_binary_input_as_raw_event() {
    let (addr, _state) = start("127.0.0.1".to_string(), 0)
        .await
        .expect("start relay");
    let url = format!("ws://{addr}/ws");

    let mut client = connect_and_join(&url, "s1").await;

    // Send binary input (keystrokes).
    client
        .send(WsMessage::Binary(b"echo hello".to_vec()))
        .await
        .expect("send binary");

    // The relay wraps it in a Raw event and fans it back out.
    let msg = read_msg(&mut client).await;
    assert_type(&msg, "raw");
    if let WsMessage::Text(text) = &msg {
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(v["data"], "echo hello");
        assert_eq!(v["id"], 0, "relay uses placeholder id");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_fans_out_published_events_to_all_joined_clients() {
    let (addr, state) = start("127.0.0.1".to_string(), 0)
        .await
        .expect("start relay");
    let url = format!("ws://{addr}/ws");

    let mut c1 = connect_and_join(&url, "s2").await;
    let mut c2 = connect_and_join(&url, "s2").await;

    // The producer's handle (returned by `start`) sees the joined
    // session and can publish into it.
    assert!(state.has_session("s2").await, "both clients joined s2");

    let event = TerminalEvent::Output {
        id: 7,
        data: "published".to_string(),
        exit_code: Some(0),
        at: chrono::Utc::now(),
    };
    assert!(
        state.publish_event("s2", &event).await,
        "publish to joined session should succeed"
    );

    // Both clients receive the fanned-out event.
    for (i, client) in [&mut c1, &mut c2].into_iter().enumerate() {
        let msg = read_msg(client).await;
        assert_type(&msg, "output");
        if let WsMessage::Text(text) = &msg {
            let v: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(v["id"], 7, "client {i}");
            assert_eq!(v["data"], "published", "client {i}");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_fans_out_inbound_event_to_other_clients() {
    let (addr, _state) = start("127.0.0.1".to_string(), 0)
        .await
        .expect("start relay");
    let url = format!("ws://{addr}/ws");

    let mut c1 = connect_and_join(&url, "s3").await;
    let mut c2 = connect_and_join(&url, "s3").await;

    // c1 sends a terminal event (as if it were the producer pushing
    // through a client connection).
    let event = TerminalEvent::Command {
        id: 42,
        text: "echo world".to_string(),
        started_at: chrono::Utc::now(),
        at: chrono::Utc::now(),
    };
    let event_json = serde_json::to_string(&event).unwrap();
    c1.send(WsMessage::Text(event_json))
        .await
        .expect("send event");

    // Both clients should receive the fanned-out event (the sender via
    // the direct send, the other via the broadcast).
    for (i, client) in [&mut c1, &mut c2].into_iter().enumerate() {
        let msg = read_msg(client).await;
        assert_type(&msg, "command");
        if let WsMessage::Text(text) = &msg {
            let v: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(v["id"], 42, "client {i}");
            assert_eq!(v["text"], "echo world", "client {i}");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_rejects_invalid_control_message() {
    let (addr, _state) = start("127.0.0.1".to_string(), 0)
        .await
        .expect("start relay");
    let url = format!("ws://{addr}/ws");

    let mut client = connect_and_join(&url, "s4").await;

    // Send a text frame that is neither a valid control message nor a
    // valid TerminalEvent.
    client
        .send(WsMessage::Text("not a valid message".to_string()))
        .await
        .expect("send invalid");

    // The relay logs a warning and drops it. No error frame is sent
    // (the relay is best-effort). Verify the connection is still
    // alive by sending a ping and getting a pong.
    client
        .send(WsMessage::Text(
            serde_json::json!({"type": "ping"}).to_string(),
        ))
        .await
        .expect("send ping");
    let msg = read_msg(&mut client).await;
    assert_type(&msg, "pong");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_publish_event_no_session_returns_false() {
    let state = TerminalRelayState::new();
    let event = TerminalEvent::Raw {
        id: 0,
        data: "test".to_string(),
        at: chrono::Utc::now(),
    };
    assert!(
        !state.publish_event("nonexistent", &event).await,
        "publish to nonexistent session should return false"
    );
}
