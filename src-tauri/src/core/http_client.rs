use std::sync::LazyLock;
use std::time::Duration;

pub const USER_AGENT: &str = "CubicLauncher/27.0.1";

pub static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(8)
        .build()
        .expect("Failed to create HTTP client")
});

pub static YGGDRASIL_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(8)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 10 {
                return attempt.error("too many redirects");
            }
            if attempt.url().scheme() == "http"
                && attempt
                    .previous()
                    .last()
                    .is_some_and(|url| url.scheme() == "https")
            {
                return attempt.error("refusing HTTPS to HTTP redirect");
            }
            attempt.follow()
        }))
        .build()
        .expect("Failed to create HTTP client")
});
