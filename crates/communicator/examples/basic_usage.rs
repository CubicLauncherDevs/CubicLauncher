use communicator::{Activity, DiscordRpcClient};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    let mut client = DiscordRpcClient::new("1305247641252397059");


    client.connect().await?;

    let activity = Activity::builder()
        .details("Developing a Rust app")
        .state("Coding")
        .start_timestamp(now)
        .large_image("rust", "Rust Programming")
        .small_image("vscode", "VS Code")
        .build();

    client.set_activity(activity).await?;

    loop {
        time::sleep(time::Duration::from_secs(10)).await;
    }
}
