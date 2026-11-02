use super::*;

fn payload(text: &str) -> Arc<[u8]> {
    Arc::from(text.as_bytes())
}

#[test]
fn market_cache_bounds_payload_bytes_and_replaces_entries() {
    let cache = JsonCache::new(100);
    let twenty = payload("\"xxxxxxxxxxxxxxxxxxxx\"");
    for i in 0..1000 {
        cache.set(i.to_string(), Arc::clone(&twenty));
    }
    assert!(cache.entries.lock().bytes <= 100);
    assert!(cache.entries.lock().map.len() <= 4);
    cache.set("999".into(), payload("\"new\""));
    assert_eq!(cache.get("999").as_deref(), Some(b"\"new\"".as_slice()));
    cache.set("huge".into(), payload(&format!("\"{}\"", "x".repeat(101))));
    assert!(cache.get("huge").is_none());
}

#[test]
fn market_cache_expires_without_revisiting_the_same_url() {
    let cache = JsonCache::new(100);
    cache.set("old".into(), payload("null"));
    cache.entries.lock().map.get_mut("old").unwrap().inserted = Instant::now() - TTL;
    cache.expire();
    assert_eq!(cache.entries.lock().bytes, 0);
    assert!(cache.entries.lock().map.is_empty());
}

#[test]
fn market_cache_evicts_least_recently_used_payload() {
    let cache = JsonCache::new(12);
    cache.set("a".into(), payload("null"));
    cache.set("b".into(), payload("null"));
    assert!(cache.get("a").is_some());
    cache.set("c".into(), payload("null"));
    assert!(cache.get("b").is_none());
    assert!(cache.get("a").is_some());
}

#[tokio::test]
async fn market_cache_sweeper_does_not_keep_a_dropped_cache_alive() {
    let cache = JsonCache::new(100);
    let weak = Arc::downgrade(&cache);
    cache.set("a".into(), payload("null"));
    assert!(cache.sweeping.load(Ordering::Relaxed));
    drop(cache);
    tokio::task::yield_now().await;
    assert!(weak.upgrade().is_none());
}
