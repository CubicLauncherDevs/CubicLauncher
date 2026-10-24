use super::*;

#[test]
fn nonce_increments() {
    let a = next_nonce().parse::<u64>().unwrap();
    let b = next_nonce().parse::<u64>().unwrap();
    assert_eq!(b, a + 1);
}

#[test]
fn buttons_include_url_in_activity() {
    let activity = Activity::builder()
        .button("GitHub", "https://github.com")
        .button("Docs", "https://docs.rs")
        .build();

    let (val, meta) = build_activity_payload(&activity);

    // Buttons include label AND url in the activity object
    let labels = val["buttons"].as_array().unwrap();
    assert_eq!(labels[0]["label"], "GitHub");
    assert_eq!(labels[0]["url"], "https://github.com");
    assert_eq!(labels[1]["label"], "Docs");
    assert_eq!(labels[1]["url"], "https://docs.rs");

    // No separate metadata field
    assert!(meta.is_none());
}

#[tokio::test]
async fn client_stores_id() {
    let client = DiscordRpcClient::new("123456789");
    assert_eq!(client.client_id, "123456789");
}

#[test]
fn backoff_grows_and_caps() {
    let mut backoff = Backoff::new();

    let first = backoff.next_delay();
    assert!(first >= RECONNECT_MIN, "first delay {first:?}");
    assert!(first < RECONNECT_MIN * 2, "first delay {first:?}");

    for _ in 0..16 {
        backoff.next_delay();
    }
    let capped = backoff.next_delay();
    assert!(capped >= RECONNECT_MAX, "capped delay {capped:?}");
    assert!(capped < RECONNECT_MAX * 2, "capped delay {capped:?}");

    backoff.reset();
    assert!(backoff.next_delay() < RECONNECT_MIN * 2);
}

#[tokio::test]
async fn frame_roundtrip_preserves_opcode_and_payload() {
    let payload = json!({"cmd": "SET_ACTIVITY", "nonce": "1"});
    let mut buf = Vec::new();
    write_frame(&mut buf, OP_FRAME, &payload).await.unwrap();

    let mut cursor = &buf[..];
    let (opcode, value) = read_frame(&mut cursor).await.unwrap();

    assert_eq!(opcode, OP_FRAME);
    assert_eq!(value, payload);
}

#[tokio::test]
async fn empty_frame_is_accepted_as_null() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&OP_PING.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());

    let mut cursor = &buf[..];
    let (opcode, value) = read_frame(&mut cursor).await.unwrap();

    assert_eq!(opcode, OP_PING);
    assert!(value.is_null());
}

#[tokio::test]
async fn oversized_frame_is_rejected() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&OP_FRAME.to_le_bytes());
    buf.extend_from_slice(&(MAX_FRAME_BYTES + 1).to_le_bytes());

    let mut cursor = &buf[..];
    let err = read_frame(&mut cursor).await.unwrap_err();

    assert!(matches!(err, DiscordRpcError::Protocol(_)));
}
