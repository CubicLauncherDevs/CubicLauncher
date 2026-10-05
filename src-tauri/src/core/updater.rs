use std::path::Path;

fn read_channel(settings: &Path) -> String {
    std::fs::read(settings)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|settings| settings["update_channel"].as_str().map(str::to_owned))
        .filter(|channel| channel == "prerelease")
        .unwrap_or_else(|| "stable".into())
}

/// Preserve all configured fallbacks. The first endpoint understands channels;
/// GitHub's latest.json remains a stable fallback for either channel.
pub(crate) fn configure(
    config: &mut serde_json::Value,
    settings: &Path,
    override_url: Option<&str>,
) -> Result<(), String> {
    let channel = read_channel(settings);
    let configured: Vec<String> = if let Some(url) = override_url {
        // A local test override must never unexpectedly fall back to production.
        vec![url.to_owned()]
    } else {
        config["endpoints"]
            .as_array()
            .ok_or("Missing updater endpoints")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "Invalid updater endpoint".to_string())
            })
            .collect::<Result<_, _>>()?
    };
    if configured.is_empty() {
        return Err("Missing updater endpoints".into());
    }
    let mut endpoints = Vec::new();
    for (index, endpoint) in configured.iter().enumerate() {
        let mut url = url::Url::parse(endpoint).map_err(|error| error.to_string())?;
        if index == 0 {
            let pairs: Vec<_> = url
                .query_pairs()
                .filter(|(key, _)| key != "channel")
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect();
            url.set_query(None);
            if !pairs.is_empty() || channel == "prerelease" {
                let mut query = url.query_pairs_mut();
                query.extend_pairs(pairs);
                if channel == "prerelease" {
                    query.append_pair("channel", "prerelease");
                }
            }
        }
        endpoints.push(url.to_string());
    }
    let object = config
        .as_object_mut()
        .ok_or("Invalid updater configuration")?;
    object.insert("endpoints".into(), serde_json::json!(endpoints));
    Ok(())
}

#[cfg(test)]
#[path = "../tests/core/updater.rs"]
mod tests;
