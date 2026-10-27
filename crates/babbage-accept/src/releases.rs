//! A release server on 127.0.0.1 shaped like Forgejo's, so the update checks never touch the network. It lists
//! releases for one repository, serves an asset and its checksum for each, and counts the requests it gets (a
//! check that must stay offline can then be shown to have asked nothing).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, ensure};
use serde_json::json;

pub struct Releases {
    /// `http://127.0.0.1:PORT`
    pub base: String,
    addr: std::net::SocketAddr,
    paths: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
}

/// One release the server lists.
pub struct Listed<'a> {
    pub version: &'a str,
    pub notes: &'a str,
}

fn sha256_of(bytes: &[u8]) -> anyhow::Result<String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("run sha256sum")?;
    child
        .stdin
        .take()
        .context("sha256sum stdin")?
        .write_all(bytes)?;
    let out = child.wait_with_output()?;
    ensure!(out.status.success(), "sha256sum failed");
    Ok(String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .context("a checksum")?
        .to_string())
}

/// The AppImage file name for `version` that suits this machine.
pub fn asset_name(version: &str) -> String {
    format!("babbage-{version}-{}.AppImage", std::env::consts::ARCH)
}

impl Releases {
    /// Starts serving `releases` (newest first) of `repo` (`owner/repo`); every release carries `asset` as its
    /// AppImage.
    pub fn start(repo: &str, releases: &[Listed], asset: &[u8]) -> anyhow::Result<Releases> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let base = format!("http://{addr}");
        let sum = sha256_of(asset)?;
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut listed = Vec::new();
        for r in releases {
            let name = asset_name(r.version);
            let dir = format!("/{repo}/releases/download/v{}", r.version);
            files.push((format!("{dir}/{name}"), asset.to_vec()));
            files.push((
                format!("{dir}/{name}.sha256"),
                format!("{sum}  {name}\n").into_bytes(),
            ));
            listed.push(json!({
                "tag_name": format!("v{}", r.version),
                "name": format!("Babbage {}", r.version),
                "body": format!("{}\n\n## Install\n\nBOILERPLATE: download the file for your system.\n", r.notes),
                "draft": false,
                "prerelease": false,
                "published_at": "2026-10-26T12:00:00Z",
                "html_url": format!("{base}/{repo}/releases/tag/v{}", r.version),
                "assets": [
                    {"name": name, "size": asset.len(), "browser_download_url": format!("{base}{dir}/{name}")},
                    {"name": format!("{name}.sha256"), "size": 90, "browser_download_url": format!("{base}{dir}/{name}.sha256")},
                ],
            }));
        }
        let api = format!("/api/v1/repos/{repo}/releases");
        files.push((api, json!(listed).to_string().into_bytes()));
        let paths = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        {
            let (paths, stop) = (paths.clone(), stop.clone());
            std::thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let (files, paths) = (files.clone(), paths.clone());
                    std::thread::spawn(move || serve(stream, &files, &paths));
                }
            });
        }
        Ok(Releases {
            base,
            addr,
            paths,
            stop,
        })
    }

    /// Every path asked for so far.
    pub fn requests(&self) -> Vec<String> {
        self.paths.lock().expect("lock").clone()
    }

    /// How many times the list of releases was asked for.
    pub fn list_requests(&self) -> usize {
        self.requests()
            .iter()
            .filter(|p| p.starts_with("/api/"))
            .count()
    }
}

impl Drop for Releases {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
    }
}

fn serve(mut stream: TcpStream, files: &[(String, Vec<u8>)], paths: &Mutex<Vec<String>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 65_536 {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let text = String::from_utf8_lossy(&buf).into_owned();
    let target = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");
    let path = target.split('?').next().unwrap_or("/").to_string();
    paths.lock().expect("lock").push(path.clone());
    let (status, body): (&str, &[u8]) = match files.iter().find(|(p, _)| *p == path) {
        Some((_, body)) => ("200 OK", body),
        None => ("404 Not Found", b"not found"),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}
