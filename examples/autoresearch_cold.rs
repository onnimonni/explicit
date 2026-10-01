//! Full-build cold-cache benchmark. Private sources stay in place; only counts are printed.
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use explicit::config::Config;
use explicit::engine::{self, Options};
use explicit::links::remote::{self, CheckOpts, RemoteStatus, Target};
use explicit::rules::FileCtx;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    let root = PathBuf::from(std::env::var("EXPLICIT_COLD_CORPUS")?);
    let cache = tempfile::tempdir()?;
    let mut config = Config::discover(&root)?;
    config.links.remote = false;
    config.links.offline = true;
    let files = engine::discover(&[root], &config)?;
    assert!(!files.is_empty(), "empty corpus");
    let ws = engine::build_workspace(&files, &config);
    let load_ms = started.elapsed().as_secs_f64() * 1000.0;
    let opts = Options {
        cache_dir: Some(cache.path().to_path_buf()),
        link_cache_dir: Some(cache.path().to_path_buf()),
        ..Options::default()
    };
    let check_started = Instant::now();
    let (cold, cold_stats) = engine::check_with_stats(&ws, &files, &config, &opts);
    let local_ms = started.elapsed().as_secs_f64() * 1000.0;
    let file_ms = check_started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(cold_stats.hits, 0);
    assert_eq!(cold_stats.misses, ws.files.len());
    let warm_started = Instant::now();
    let (warm, warm_stats) = engine::check_with_stats(&ws, &files, &config, &opts);
    let warm_ms = warm_started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(warm_stats.hits, ws.files.len());
    assert_eq!(warm_stats.misses, 0);
    assert_eq!(serde_json::to_vec(&cold)?, serde_json::to_vec(&warm)?);

    let mut targets = HashSet::new();
    for a in ws.files.values() {
        targets.extend(remote::urls(&FileCtx { a, config: &config }));
    }
    let mut hosts = HashMap::<String, usize>::new();
    let mut unique = HashSet::new();
    for t in &targets {
        if !config.link_excludes().iter().any(|re| re.is_match(&t.url)) && unique.insert(&t.url) {
            let authority = t
                .url
                .split_once("://")
                .unwrap()
                .1
                .split(['/', '?', '#'])
                .next()
                .unwrap();
            *hosts.entry(authority.to_string()).or_default() += 1;
        }
    }
    println!("METRIC corpus_discovered={}", files.len());
    println!("METRIC corpus_files={}", ws.files.len());
    println!("METRIC corpus_diagnostics={}", cold.len());
    println!("METRIC remote_urls={}", unique.len());
    println!("METRIC remote_hosts={}", hosts.len());
    println!(
        "METRIC busiest_host_urls={}",
        hosts.values().max().unwrap_or(&0)
    );
    println!("METRIC discovery_ms={load_ms:.3}");
    println!("METRIC file_checks_ms={file_ms:.3}");
    println!("METRIC local_cold_ms={local_ms:.3}");
    println!("METRIC local_warm_ms={warm_ms:.3}");
    let remote_ms = remote_workload()?;
    println!("METRIC remote_cold_ms={remote_ms:.3}");
    println!("METRIC cold_start_ms={:.3}", local_ms + remote_ms);
    Ok(())
}

fn remote_workload() -> Result<f64, Box<dyn Error>> {
    let requests = Arc::new(AtomicUsize::new(0));
    let mut urls = Vec::new();
    // Eight origins; 128 unique resources; fixed latency; HEAD success and GET fallback.
    for _ in 0..8 {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let base = format!("http://{}", listener.local_addr()?);
        for i in 0..16 {
            let kind = match i % 4 {
                0 => "missing",
                1 => "nohead",
                _ => "ok",
            };
            urls.push(Target::link(format!("{base}/{kind}/{i}")));
        }
        let requests = requests.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let requests = requests.clone();
                std::thread::spawn(move || {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut first = String::new();
                    reader.read_line(&mut first).unwrap();
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                            break;
                        }
                    }
                    requests.fetch_add(1, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(100));
                    let mut parts = first.split_whitespace();
                    let head = parts.next() == Some("HEAD");
                    let path = parts.next().unwrap();
                    let status = if path.starts_with("/missing/") {
                        404
                    } else if head && path.starts_with("/nohead/") {
                        405
                    } else {
                        200
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .unwrap();
                });
            }
        });
    }
    let cache = tempfile::tempdir()?;
    let mut config = Config::default();
    config.links.allow_private = true;
    config.links.exclude.clear();
    config.links.accept_status.clear();
    let opts = CheckOpts {
        cache_path: Some(cache.path().join("links.json")),
        ..Default::default()
    };
    let start = Instant::now();
    let result = remote::check_all_with(&urls, &config, &opts);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(result.len(), urls.len());
    for t in &urls {
        let expected = if t.url.contains("/missing/") {
            RemoteStatus::HttpError(404)
        } else {
            RemoteStatus::Ok
        };
        assert_eq!(result[&t.url], expected);
    }
    let cold_requests = requests.load(Ordering::Relaxed);
    assert_eq!(cold_requests, 192);
    assert_eq!(remote::check_all_with(&urls, &config, &opts), result);
    assert_eq!(
        requests.load(Ordering::Relaxed),
        cold_requests,
        "warm cache requested network"
    );
    println!("METRIC fixture_requests={cold_requests}");
    Ok(elapsed)
}
