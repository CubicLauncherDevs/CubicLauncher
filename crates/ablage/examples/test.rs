use std::io::Read;

fn main() {
    let path = "/tmp/test_v2.crep";
    let mut repo = ablage::Repo::open(path);
    repo.put("hello", ablage::Entry { version: 1, fingerprint: 123, data: b"world".to_vec() });
    repo.put("test", ablage::Entry { version: 2, fingerprint: 456, data: b"data".to_vec() });
    repo.flush().unwrap();
    drop(repo);

    let repo2 = ablage::Repo::open(path);
    for k in repo2.keys() {
        let e = repo2.get(k).unwrap();
        println!("{}: v={} fp={} data={:?}", k, e.version, e.fingerprint, String::from_utf8_lossy(&e.data));
    }

    // also read raw to show binary structure
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    println!("file size: {} bytes", buf.len());
    println!("magic: {:?}", &buf[..4]);
    println!("version: {}", buf[4]);
    println!("schema_ver: {}", buf[5]);
    println!("entry count: {}", u32::from_le_bytes(buf[6..10].try_into().unwrap()));
}
