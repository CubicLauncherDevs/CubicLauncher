use super::*;
use tokio::net::TcpListener;

#[test]
fn parses_domains_ipv4_ipv6_and_ports() {
    for (raw, host, port) in [
        (" example.org ", "example.org", 25565),
        ("example.org:25566", "example.org", 25566),
        ("127.0.0.1", "127.0.0.1", 25565),
        ("[::1]:1234", "::1", 1234),
        ("[::1]", "::1", 25565),
        ("::1", "::1", 25565),
    ] {
        let target = ServerAddress::parse(raw).unwrap();
        assert_eq!(target.host, host);
        assert_eq!(target.port, port);
        assert_eq!(ServerAddress::parse(&target.authority()).unwrap(), target);
    }
    assert_eq!(
        ServerAddress::parse("münich.example").unwrap().host,
        "xn--mnich-kva.example"
    );
    for raw in [
        "",
        "http://example.org",
        "foo/bar",
        "host:0",
        "host:65536",
        "host:",
        "host:abc",
        "host:1:2",
        "a b",
        "a\nb",
        "[no-ipv6]",
        "[::1]oops",
        "@host",
        "host?foo",
    ] {
        assert!(ServerAddress::parse(raw).is_err(), "{raw}");
    }
}

#[tokio::test]
async fn srv_is_not_used_for_ip_or_custom_port() {
    for raw in ["127.0.0.1", "[::1]", "example.invalid:1234"] {
        let address = ServerAddress::parse(raw).unwrap();
        assert_eq!(resolve_target(&address).await, address);
    }
}

#[tokio::test]
async fn varints_reject_overflow_and_truncation() {
    for value in [0, 1, 127, 128, 16384, i32::MAX as u32] {
        let mut bytes = Vec::new();
        put_varint(&mut bytes, value);
        assert_eq!(read_varint(&mut bytes.as_slice()).await.unwrap(), value);
    }
    for bytes in [
        &[255, 255, 255, 255, 15][..],
        &[128, 128, 128, 128, 128],
        &[128],
    ] {
        assert!(read_varint(&mut &bytes[..]).await.is_err());
    }
}

#[test]
fn descriptions_handle_nested_components_and_strip_formatting() {
    assert_eq!(
        description(
            &serde_json::json!({"text":"§aHello ", "extra":[{"text":"world"}, "\n§lJava"]})
        ),
        "Hello world\nJava"
    );
    assert!(description(&serde_json::json!("x".repeat(20_000))).len() <= 4096);
    assert!(description(&serde_json::json!("🧊".repeat(20_000))).len() <= 4096);
}

#[tokio::test]
async fn status_protocol_handles_fragmentation_and_matching_pong() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let handshake = read_packet(&mut socket).await.unwrap();
        assert!(handshake.windows(12).any(|w| w == b"virtual.host"));
        assert_eq!(read_packet(&mut socket).await.unwrap(), [0]);
        let json = serde_json::to_vec(&serde_json::json!({
            "description":{"text":"§bLocal", "extra":[{"text":" test"}]},
            "players":{"online":3,"max":20}, "version":{"name":"1.21.1"},
            "favicon":"data:image/png;base64,bm90LXBuZw=="
        }))
        .unwrap();
        let mut response = vec![0];
        put_varint(&mut response, json.len() as u32);
        response.extend(json);
        let mut frame = Vec::new();
        put_varint(&mut frame, response.len() as u32);
        frame.extend(response);
        for byte in frame {
            socket.write_all(&[byte]).await.unwrap();
        }
        let ping = read_packet(&mut socket).await.unwrap();
        assert_eq!(ping.len(), 9);
        assert_eq!(ping[0], 1);
        send_packet(&mut socket, &ping).await.unwrap();
    });
    let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let status = exchange(
        &mut socket,
        &ServerAddress {
            host: "virtual.host".into(),
            port,
        },
    )
    .await
    .unwrap();
    assert!(status.online);
    assert_eq!(status.players, Some(3));
    assert_eq!(status.max_players, Some(20));
    assert_eq!(status.motd, "Local test");
    assert_eq!(status.version.as_deref(), Some("1.21.1"));
    assert!(status.ping.is_some());
    assert!(status.icon.is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn malformed_packets_are_rejected_without_large_allocations() {
    for response in [
        vec![0],
        vec![255, 255, 255, 127],
        vec![3, 0, 10, b'{'],
        vec![255, 255, 255, 255, 255],
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_packet(&mut socket).await.unwrap();
            read_packet(&mut socket).await.unwrap();
            socket.write_all(&response).await.unwrap();
        });
        let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        assert!(
            exchange(
                &mut socket,
                &ServerAddress {
                    host: "localhost".into(),
                    port
                }
            )
            .await
            .is_err()
        );
        task.await.unwrap();
    }
}

#[tokio::test]
async fn cancelled_query_closes_socket_and_releases_slot() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = ServerAddress {
        host: "127.0.0.1".into(),
        port: listener.local_addr().unwrap().port(),
    };
    let query = tokio::spawn(async move { query(&address).await });
    let (mut socket, _) = listener.accept().await.unwrap();
    read_packet(&mut socket).await.unwrap();
    read_packet(&mut socket).await.unwrap();
    query.abort();
    assert!(query.await.unwrap_err().is_cancelled());
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), socket.read_u8())
            .await
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}

#[tokio::test]
async fn unresponsive_server_can_be_timed_out() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut socket = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (_peer, _) = listener.accept().await.unwrap();
    let address = ServerAddress {
        host: "localhost".into(),
        port: 25565,
    };
    assert!(
        tokio::time::timeout(Duration::from_millis(30), exchange(&mut socket, &address))
            .await
            .is_err()
    );
}
