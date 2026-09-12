//! Run this fixture alone with --ignored --nocapture --test-threads=1.
use super::server_manager as servers;
use base64::{Engine, engine::general_purpose::STANDARD};
use fastnbt::Value;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    collections::HashMap,
    time::Instant,
};

#[derive(Clone, Copy, Default, Debug)]
struct Allocations {
    calls: usize,
    live: isize,
    peak: usize,
}
thread_local! { static TRACK: Cell<Option<Allocations>> = const { Cell::new(None) }; }
struct CountingAllocator;
// Test-only, per-thread accounting; no changes to the production allocator.
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn account(bytes: isize, allocation: bool) {
    let _ = TRACK.try_with(|track| {
        if let Some(mut stats) = track.get() {
            stats.calls += usize::from(allocation);
            stats.live += bytes;
            stats.peak = stats.peak.max(stats.live.max(0) as usize);
            track.set(Some(stats));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            account(layout.size() as isize, true);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        account(-(layout.size() as isize), false);
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, old, size) };
        if !result.is_null() {
            account(size as isize - old.size() as isize, true);
        }
        result
    }
}

fn cpu_ns() -> Option<u64> {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[test]
#[ignore = "manual allocation/CPU fixture: 50, 500 and 1000 servers with PNG icons"]
fn server_performance_fixture() {
    let dir = tempfile::tempdir().unwrap();
    // A deterministic, moderately sized icon rather than a highly compressible blank PNG.
    let mut pixels = image::RgbaImage::new(32, 32);
    let mut random = 42u32;
    for pixel in pixels.pixels_mut() {
        random = random.wrapping_mul(1664525).wrapping_add(1013904223);
        *pixel = image::Rgba([random as u8, (random >> 8) as u8, (random >> 16) as u8, 255]);
    }
    let mut png = std::io::Cursor::new(Vec::new());
    pixels.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let icon = STANDARD.encode(png.into_inner());
    for count in [50, 500, 1000] {
        let entries = (0..count)
            .map(|index| {
                Value::Compound(HashMap::from([
                    ("name".into(), Value::String(format!("Server {index}"))),
                    (
                        "ip".into(),
                        Value::String(format!("server-{index}.example")),
                    ),
                    ("icon".into(), Value::String(icon.clone())),
                ]))
            })
            .collect();
        let data = fastnbt::to_bytes(&HashMap::from([("servers", Value::List(entries))])).unwrap();
        std::fs::write(dir.path().join("servers.dat"), &data).unwrap();
        let cpu = cpu_ns();
        let start = Instant::now();
        servers::metrics::reset();
        TRACK.with(|t| t.set(Some(Allocations::default())));
        // Open metadata, fetch only the first page's icons, prepare small targets.
        let list = servers::list(dir.path()).unwrap();
        let queued: Vec<_> = list
            .servers
            .iter()
            .take(50)
            .map(|s| (s.index, s.address.clone()))
            .collect();
        let icons = servers::icons(
            dir.path(),
            &list.revision,
            queued.iter().map(|s| s.0).collect(),
        )
        .unwrap();
        let payload = serde_json::to_vec(&(&list, &icons)).unwrap();
        std::hint::black_box(&queued);
        let ipc = payload.len();
        let before_drop = TRACK.with(|t| t.get().unwrap());
        drop((list, queued, icons, payload));
        let stats = TRACK.with(|t| t.replace(None).unwrap());
        let (reads, decodes) = servers::metrics::counts();
        let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
        let cpu_ms = cpu.zip(cpu_ns()).map(|(a, b)| (b - a) as f64 / 1_000_000.0);
        println!(
            "servers={count} reads={reads} decodes={decodes} ping_targets={} wall_ms={wall_ms:.2} cpu_ms={cpu_ms:?} allocs={} peak_bytes={} retained_bytes={} after_drop_bytes={} ipc_bytes={ipc}",
            count.min(50),
            stats.calls,
            stats.peak,
            before_drop.live,
            stats.live
        );
        assert_eq!((reads, decodes), (2, 50));
        assert!(
            stats.peak < data.len() + 1024 * 1024,
            "no retained full NBT/PNG tree"
        );
        assert!(
            before_drop.live < 1024 * 1024,
            "page payload remains bounded as the list grows"
        );
        assert_eq!(stats.live, 0, "all measured allocations must be released");
    }
}
