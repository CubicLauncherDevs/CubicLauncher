use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Sirve una sola respuesta. Con `hold`, la conexión queda abierta después del cuerpo, como la
/// de un servidor que nunca termina de mandar.
async fn serve_skin(
    head: &str,
    body: Vec<u8>,
    hold: bool,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/skin.png", listener.local_addr().unwrap());
    let head = head.to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let count = socket.read(&mut buffer).await.unwrap();
            assert!(count > 0 && request.len() < 16_384);
            request.extend_from_slice(&buffer[..count]);
        }
        // El cliente puede cortar a mitad del cuerpo: eso es lo que se prueba, no un fallo.
        let _ = socket.write_all(head.as_bytes()).await;
        let _ = socket.write_all(&body).await;
        if hold {
            std::future::pending::<()>().await;
        }
    });
    (url, server)
}

fn skin_of_size(size: usize) -> Vec<u8> {
    let mut body = PNG_SIGNATURE.to_vec();
    body.resize(size, 0);
    body
}

#[tokio::test]
async fn url_skin_stops_at_the_limit_without_waiting_for_the_rest() {
    // Sin Content-Length y sin cerrar la conexión: solo se puede cortar contando lo que llega.
    let (url, server) = serve_skin(
        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\n\r\n",
        skin_of_size(SKIN_URL_MAX_BYTES + 1),
        true,
    )
    .await;

    let result = tokio::time::timeout(Duration::from_secs(5), download_skin(&url))
        .await
        .expect("la descarga esperó al resto de la respuesta");
    let error = result.unwrap_err();
    assert!(error.contains("la skin pesa demasiado"), "{error}");
    server.abort();
}

#[tokio::test]
async fn url_skin_rejects_a_declared_size_over_the_limit() {
    let (url, server) = serve_skin(
        &format!(
            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n",
            SKIN_URL_MAX_BYTES + 1
        ),
        PNG_SIGNATURE.to_vec(),
        true,
    )
    .await;

    let result = tokio::time::timeout(Duration::from_secs(5), download_skin(&url))
        .await
        .expect("la descarga no miró Content-Length");
    let error = result.unwrap_err();
    assert!(error.contains("la skin pesa demasiado"), "{error}");
    server.abort();
}

#[tokio::test]
async fn url_skin_up_to_the_limit_arrives_whole() {
    let body = skin_of_size(SKIN_URL_MAX_BYTES);
    let (url, server) = serve_skin(
        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n",
        body.clone(),
        false,
    )
    .await;

    assert_eq!(download_skin(&url).await.unwrap(), body);
    server.await.unwrap();
}
