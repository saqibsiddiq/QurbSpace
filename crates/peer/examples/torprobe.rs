//! How long it takes to reach an onion service through Tor from here, and
//! whether it can be reached at all: `cargo run --example torprobe --features
//! tor, <onion>`. Measures the path a phone away from home uses to reach its
//! computer (decision 0082). A fresh Tor state each run, in a temporary
//! directory, so the first figure includes fetching Tor's directory.

#[tokio::main]
async fn main() {
    let onion = std::env::args().nth(1).expect("an onion address");
    let dir = tempfile::tempdir().expect("a temporary directory");
    qurb_peer::tor::set_dir(dir.path().to_path_buf());
    for attempt in 1..=3 {
        let started = std::time::Instant::now();
        match tokio::time::timeout(std::time::Duration::from_secs(120), qurb_peer::tor::dial(&onion)).await {
            Ok(Ok(at)) => println!("attempt {attempt}: reached in {:.1} s (bridge at {at})", started.elapsed().as_secs_f64()),
            Ok(Err(e)) => println!("attempt {attempt}: failed after {:.1} s: {e}", started.elapsed().as_secs_f64()),
            Err(_) => println!("attempt {attempt}: nothing after 120 s"),
        }
    }
}
