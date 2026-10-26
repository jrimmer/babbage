//! Drives the real binary: a nested headless Hyprland, a temporary data folder per app, the app's control
//! socket for reading state, `wtype` for real keystrokes and `grim` for pixels.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command as Process, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, bail, ensure};
use serde_json::Value;

/// A line of stderr that means the app crashed.
const PANIC_MARKERS: [&str; 2] = ["panicked", "already being updated"];
const GRIM_TIMEOUT: Duration = Duration::from_secs(15);

pub struct Harness {
    pub repo: PathBuf,
    bin: PathBuf,
    nested: Nested,
    pointer: Helper,
    ime: Option<Helper>,
    work: tempfile::TempDir,
    pub logs: PathBuf,
    launches: usize,
}

struct Nested {
    script: PathBuf,
    dir: PathBuf,
    wayland: String,
    sig: String,
    stopped: bool,
}

impl Nested {
    fn start(script: PathBuf, dir: PathBuf) -> anyhow::Result<Nested> {
        let out = Process::new(&script).arg("start").arg(&dir).output()?;
        ensure!(
            out.status.success(),
            "nested Hyprland: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let env = String::from_utf8_lossy(&out.stdout).into_owned();
        let get = |k: &str| {
            env.lines()
                .find_map(|l| l.strip_prefix(k))
                .map(str::to_owned)
                .with_context(|| format!("no {k} from nested-hyprland.sh"))
        };
        ensure!(
            get("runtime=").is_err(),
            "standalone compositors are not supported by these checks"
        );
        Ok(Nested {
            script,
            dir,
            wayland: get("wayland=")?,
            sig: get("sig=")?,
            stopped: false,
        })
    }

    fn stop(&mut self) {
        if std::mem::replace(&mut self.stopped, true) {
            return;
        }
        let pid = std::fs::read_to_string(self.dir.join("nested.env"))
            .ok()
            .and_then(|e| {
                e.lines()
                    .find_map(|l| l.strip_prefix("pid=").map(str::to_owned))
            });
        let mut stop = Process::new(&self.script);
        stop.arg("stop").arg(&self.dir);
        let _ = run_with_timeout(stop, GRIM_TIMEOUT);
        // A compositor that stopped answering ignores the exit request, so the one this run started is
        // ended by its own pid.
        if let Some(pid) = pid
            && Path::new(&format!("/proc/{pid}")).exists()
        {
            let _ = Process::new("kill").args(["-KILL", &pid]).status();
        }
    }
}

impl Drop for Nested {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A virtual mouse (`tools/pointer.py`) that lives as long as the nested compositor, so its seat always
/// reports a pointer and the app listens for the mouse from its first frame.
/// A helper process that drives the nested compositor over its own protocol (the virtual pointer, the input
/// method): one command per line on its stdin, `ok` back on its stdout.
struct Helper {
    name: &'static str,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Helper {
    fn start(name: &'static str, script: &Path, args: &[String]) -> anyhow::Result<Helper> {
        let mut child = Process::new("python3")
            .arg("-I")
            .arg(script)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("python3 {}", script.display()))?;
        let stdin = child.stdin.take().context("no stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("no stdout")?);
        let mut helper = Helper {
            name,
            child,
            stdin,
            stdout,
        };
        let first = helper.read_line()?;
        ensure!(first == "ready", "{name} said {first:?}");
        Ok(helper)
    }

    fn read_line(&mut self) -> anyhow::Result<String> {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line)?;
        ensure!(n > 0, "{} exited", self.name);
        Ok(line.trim().to_owned())
    }

    fn send(&mut self, command: &str) -> anyhow::Result<()> {
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()?;
        let answer = self.read_line()?;
        ensure!(answer == "ok", "{}: {command:?} gave {answer:?}", self.name);
        Ok(())
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        // Only the helper this harness started.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct App {
    name: String,
    pid: u32,
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    err_log: PathBuf,
    pub data_dir: PathBuf,
    sock: PathBuf,
    finished: bool,
}

fn hyprctl_on(sig: &str, args: &[&str]) -> anyhow::Result<Vec<u8>> {
    let out = Process::new("hyprctl")
        .env("HYPRLAND_INSTANCE_SIGNATURE", sig)
        .args(args)
        .output()?;
    ensure!(out.status.success(), "hyprctl {args:?} failed");
    Ok(out.stdout)
}

fn alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Runs `cmd` with stdout piped, killing it after `timeout`. `None` means it was killed.
fn run_with_timeout(
    mut cmd: Process,
    timeout: Duration,
) -> anyhow::Result<Option<std::process::Output>> {
    let mut child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    // A PPM of the whole output is bigger than a pipe, so stdout is drained on its own thread.
    let mut stdout = child.stdout.take().context("stdout")?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).map(|_| buf)
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break Some(s);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = reader
        .join()
        .map_err(|_| anyhow::anyhow!("reader panicked"))??;
    Ok(status.map(|status| std::process::Output {
        status,
        stdout,
        stderr: Vec::new(),
    }))
}

/// A screenshot of the headless output.
pub struct Shot {
    pub w: usize,
    pub h: usize,
    data: Vec<u8>,
}

impl Shot {
    fn parse(bytes: &[u8]) -> anyhow::Result<Shot> {
        let mut fields = Vec::new();
        let mut i = 0;
        while fields.len() < 4 {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'#') {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            ensure!(start < i, "truncated PPM header");
            fields.push(std::str::from_utf8(&bytes[start..i])?);
        }
        ensure!(
            fields[0] == "P6" && fields[3] == "255",
            "unexpected PPM format"
        );
        let (w, h): (usize, usize) = (fields[1].parse()?, fields[2].parse()?);
        let data = bytes[i + 1..].to_vec();
        ensure!(data.len() >= w * h * 3, "truncated PPM data");
        Ok(Shot { w, h, data })
    }

    pub fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.w + x) * 3;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }

    /// The brightest pixel (by luminance) in the rectangle, clipped to the screen. `None` if the
    /// rectangle is empty or off screen.
    pub fn brightest(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Option<[u8; 3]> {
        let (xa, xb) = (x0.max(0.) as usize, (x1.max(0.) as usize).min(self.w));
        let (ya, yb) = (y0.max(0.) as usize, (y1.max(0.) as usize).min(self.h));
        let mut best: Option<([u8; 3], f32)> = None;
        for y in ya..yb {
            for x in xa..xb {
                let p = self.pixel(x, y);
                let l = luminance(p);
                if best.is_none_or(|(_, bl)| l > bl) {
                    best = Some((p, l));
                }
            }
        }
        best.map(|(p, _)| p)
    }
}

pub fn luminance(p: [u8; 3]) -> f32 {
    0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32
}

impl Harness {
    pub fn new(repo: PathBuf) -> anyhow::Result<Harness> {
        let bin = repo.join("target/release/babbage");
        ensure!(bin.exists(), "{} is missing; run bin/build", bin.display());
        // Under the repository's target folder: /tmp is a small shared tmpfs.
        let base = repo.join("target/accept-work");
        std::fs::create_dir_all(&base)?;
        let work = tempfile::Builder::new().prefix("run").tempdir_in(&base)?;
        let nested = Nested::start(
            repo.join("tools/nested-hyprland.sh"),
            work.path().join("nested"),
        )?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let logs = repo.join(format!("target/accept-logs/{stamp}"));
        std::fs::create_dir_all(&logs)?;
        let pointer = {
            let monitors: Value =
                serde_json::from_slice(&hyprctl_on(&nested.sig, &["-j", "monitors"])?)?;
            let (mut right, mut bottom) = (0f64, 0f64);
            for m in monitors.as_array().context("no monitors")? {
                let scale = m["scale"].as_f64().unwrap_or(1.);
                right = right
                    .max(m["x"].as_f64().unwrap_or(0.) + m["width"].as_f64().unwrap_or(0.) / scale);
                bottom = bottom.max(
                    m["y"].as_f64().unwrap_or(0.) + m["height"].as_f64().unwrap_or(0.) / scale,
                );
            }
            Helper::start(
                "pointer.py",
                &repo.join("tools/pointer.py"),
                &[
                    nested.wayland.clone(),
                    (right.ceil() as u32).to_string(),
                    (bottom.ceil() as u32).to_string(),
                ],
            )?
        };
        Ok(Harness {
            repo,
            bin,
            nested,
            pointer,
            ime: None,
            work,
            logs,
            launches: 0,
        })
    }

    fn hyprctl(&self, sig: &str, args: &[&str]) -> anyhow::Result<Vec<u8>> {
        hyprctl_on(sig, args)
    }

    fn nested_json(&self, what: &str) -> anyhow::Result<Value> {
        Ok(serde_json::from_slice(
            &self.hyprctl(&self.nested.sig, &["-j", what])?,
        )?)
    }

    /// Starts the real binary inside the nested compositor with a fresh data folder (or `data`, to
    /// reopen one).
    pub fn launch(
        &mut self,
        name: &str,
        data: Option<&Path>,
        env: &[(&str, &str)],
    ) -> anyhow::Result<App> {
        self.launches += 1;
        let n = self.launches;
        let dir = self.work.path().join(format!("app{n}"));
        std::fs::create_dir_all(&dir)?;
        let data_dir = match data {
            Some(d) => d.to_path_buf(),
            None => dir.join("sheets"),
        };
        // A unix socket path is limited to about 100 bytes, which the work folder's path exceeds, so the
        // socket lives in the runtime dir (a per-user tmpfs) under a name that is this run's own.
        let rt = std::env::var_os("XDG_RUNTIME_DIR")
            .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from);
        let sock = rt.join(format!("babbage-accept-{}-{n}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock);
        let pid_file = dir.join("app.pid");
        let home = dir.join("home");
        std::fs::create_dir_all(&home)?;
        let err_log = self.logs.join(format!("{name}-{n}.err"));
        let out_log = self.logs.join(format!("{name}-{n}.out"));
        let q = |p: &Path| shell_quote(&p.to_string_lossy());
        let mut exports = String::new();
        for (k, v) in env {
            ensure!(
                !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "bad variable name {k:?}"
            );
            exports.push_str(&format!("export {k}={}\n", shell_quote(v)));
        }
        // A fake HOME and no session bus: the app can reach neither the user's files nor the desktop.
        // The theme file does not exist, so the built-in colours are drawn and pixels never depend on
        // the user's theme.
        let script = format!(
            "#!/bin/bash\necho $$ > {pid}\nexport HOME={home}\nexport BABBAGE_DATA_DIR={data}\nexport BABBAGE_THEME_FILE={theme}\nexport XDG_STATE_HOME={home}/state\nexport XDG_CONFIG_HOME={home}/config\nexport XDG_DATA_HOME={home}/data\nexport DBUS_SESSION_BUS_ADDRESS={bus}\n{exports}exec {bin} --control {sock} > {out} 2> {err}\n",
            pid = q(&pid_file),
            home = q(&home),
            data = q(&data_dir),
            theme = q(&dir.join("no-theme.toml")),
            bus = shell_quote(&format!("unix:path={}", dir.join("no-bus").display())),
            bin = q(&self.bin),
            sock = q(&sock),
            out = q(&out_log),
            err = q(&err_log),
        );
        let wrapper = dir.join("app.sh");
        std::fs::write(&wrapper, script)?;
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))?;
        let status = Process::new(&self.nested.script)
            .arg("exec")
            .arg(&self.nested.dir)
            .arg(&wrapper)
            .status()?;
        ensure!(
            status.success(),
            "could not exec the app in the nested compositor"
        );
        let start = Instant::now();
        let stream = loop {
            if let Ok(s) = UnixStream::connect(&sock) {
                break s;
            }
            if start.elapsed() > Duration::from_secs(30) {
                let err = std::fs::read_to_string(&err_log).unwrap_or_default();
                bail!("the app did not open its control socket in 30 s\n{err}");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let pid: u32 = std::fs::read_to_string(&pid_file)?.trim().parse()?;
        let reader = BufReader::new(stream.try_clone()?);
        let app = App {
            name: name.to_owned(),
            pid,
            stream,
            reader,
            err_log,
            data_dir,
            sock: sock.clone(),
            finished: false,
        };
        // The window must be mapped before keys can reach it.
        let start = Instant::now();
        while self.window_of(app.pid).is_err() {
            ensure!(
                start.elapsed() < Duration::from_secs(20),
                "the app's window never appeared"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        self.focus(&app)?;
        Ok(app)
    }

    fn window_of(&self, pid: u32) -> anyhow::Result<Value> {
        let clients = self.nested_json("clients")?;
        clients
            .as_array()
            .and_then(|c| c.iter().find(|c| c["pid"].as_u64() == Some(u64::from(pid))))
            .cloned()
            .context("no window for the app yet")
    }

    /// Starts the scripted input method (once). It owns the seat's input method until the run ends, so only the
    /// checks that compose text start it.
    fn ime(&mut self) -> anyhow::Result<&mut Helper> {
        if self.ime.is_none() {
            self.ime = Some(Helper::start(
                "ime.py",
                &self.repo.join("tools/ime.py"),
                std::slice::from_ref(&self.nested.wayland),
            )?);
        }
        Ok(self.ime.as_mut().expect("just started"))
    }

    /// Waits until a text field has enabled text input, then shows `text` as the composition.
    pub fn compose(&mut self, text: &str) -> anyhow::Result<()> {
        let ime = self.ime()?;
        ime.send("wait")?;
        ime.send(&format!("preedit {text}"))
    }

    /// Ends the composition by committing `text`.
    pub fn commit_composition(&mut self, text: &str) -> anyhow::Result<()> {
        self.ime()?.send(&format!("commit {text}"))
    }

    /// Takes the composition away without committing anything.
    pub fn clear_composition(&mut self) -> anyhow::Result<()> {
        self.ime()?.send("clear")
    }

    /// Floats the app's window and gives it an exact size in layout points (it is tiled to the whole output
    /// otherwise, so it cannot be resized).
    pub fn resize(&self, app: &App, width: u32, height: u32) -> anyhow::Result<()> {
        let float = format!(
            "hl.dsp.window.float({{ action = \"enable\", window = \"pid:{}\" }})",
            app.pid
        );
        let said_float = self.hyprctl(&self.nested.sig, &["dispatch", &float])?;
        let resize = format!(
            "hl.dsp.window.resize({{ x = {width}, y = {height}, window = \"pid:{}\" }})",
            app.pid
        );
        let said_resize = self.hyprctl(&self.nested.sig, &["dispatch", &resize])?;
        let window = self.window_of(app.pid)?;
        ensure!(
            window["floating"] == true
                && window["size"][0]
                    .as_f64()
                    .is_some_and(|w| (w - f64::from(width)).abs() <= 2.),
            "the window did not take the size: float said {:?}, resize said {:?}, the window is floating {} at size {}",
            String::from_utf8_lossy(&said_float),
            String::from_utf8_lossy(&said_resize),
            window["floating"],
            window["size"]
        );
        Ok(())
    }

    /// Gives the app's window the keyboard focus.
    pub fn focus(&self, app: &App) -> anyhow::Result<()> {
        let lua = format!("hl.dsp.focus({{ window = \"pid:{}\" }})", app.pid);
        self.hyprctl(&self.nested.sig, &["dispatch", &lua])?;
        Ok(())
    }

    fn wtype(&self, args: &[&str]) -> anyhow::Result<()> {
        let out = Process::new("wtype")
            .env("WAYLAND_DISPLAY", &self.nested.wayland)
            .args(args)
            .output()?;
        ensure!(
            out.status.success(),
            "wtype {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(())
    }

    /// Types `text` as keystrokes into the focused window.
    pub fn type_text(&self, text: &str) -> anyhow::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        self.wtype(&["--", text])
    }

    /// Presses a named key (`Return`, `BackSpace`, `Left`, ...) with the modifiers held (`ctrl`, `shift`,
    /// `alt`).
    pub fn press(&self, mods: &[&str], key: &str) -> anyhow::Result<()> {
        let mut args: Vec<&str> = Vec::new();
        for m in mods {
            args.extend(["-M", m]);
        }
        args.extend(["-k", key]);
        for m in mods.iter().rev() {
            args.extend(["-m", m]);
        }
        self.wtype(&args)
    }

    /// Types a line and presses Return after it.
    pub fn type_line(&self, text: &str) -> anyhow::Result<()> {
        self.type_text(text)?;
        self.press(&[], "Return")
    }

    fn grim(&self, args: &[&str]) -> anyhow::Result<Vec<u8>> {
        for _ in 0..2 {
            let mut grim = Process::new("grim");
            grim.env("WAYLAND_DISPLAY", &self.nested.wayland).args(args);
            if let Some(out) = run_with_timeout(grim, GRIM_TIMEOUT)? {
                ensure!(out.status.success(), "grim failed");
                return Ok(out.stdout);
            }
        }
        bail!("grim hung twice for {GRIM_TIMEOUT:?}")
    }

    /// Clears the compositor's startup banner, which would cover the window's corner.
    fn dismiss_banner(&self) -> anyhow::Result<()> {
        self.hyprctl(&self.nested.sig, &["dismissnotify"])?;
        // The headless output redraws at 60 Hz.
        std::thread::sleep(Duration::from_millis(120));
        Ok(())
    }

    /// The screen as pixels.
    pub fn shot(&self) -> anyhow::Result<Shot> {
        self.dismiss_banner()?;
        Shot::parse(&self.grim(&["-t", "ppm", "-o", "HEADLESS-2", "-"])?)
    }

    /// Saves a PNG of the screen at `out`.
    pub fn save_shot(&self, out: &Path) -> anyhow::Result<()> {
        self.dismiss_banner()?;
        if let Some(d) = out.parent() {
            std::fs::create_dir_all(d)?;
        }
        self.grim(&["-o", "HEADLESS-2", &out.to_string_lossy()])?;
        Ok(())
    }

    /// Clicks at (`x`, `y`), in points from the window's top-left corner, with the left or right button,
    /// `count` times in quick succession (two is a double click).
    pub fn click(
        &mut self,
        app: &App,
        x: f32,
        y: f32,
        button: &str,
        count: u32,
    ) -> anyhow::Result<()> {
        let w = self.window_of(app.pid)?;
        let (ax, ay) = (
            w["at"][0].as_f64().unwrap_or(0.),
            w["at"][1].as_f64().unwrap_or(0.),
        );
        self.pointer.send(&format!(
            "click {} {} {button} {count}",
            ax + f64::from(x),
            ay + f64::from(y)
        ))
    }

    /// Presses the left button at (`x0`, `y0`), moves to (`x1`, `y1`) in steps and releases it; points are from the
    /// window's top-left corner.
    pub fn drag(&mut self, app: &App, from: (f32, f32), to: (f32, f32)) -> anyhow::Result<()> {
        let w = self.window_of(app.pid)?;
        let (ax, ay) = (
            w["at"][0].as_f64().unwrap_or(0.),
            w["at"][1].as_f64().unwrap_or(0.),
        );
        self.pointer.send(&format!(
            "drag {} {} {} {}",
            ax + f64::from(from.0),
            ay + f64::from(from.1),
            ax + f64::from(to.0),
            ay + f64::from(to.1)
        ))
    }

    /// Turns the wheel `notches` times over (`x`, `y`): positive scrolls down.
    pub fn scroll(&mut self, app: &App, x: f32, y: f32, notches: i32) -> anyhow::Result<()> {
        let w = self.window_of(app.pid)?;
        let (ax, ay) = (
            w["at"][0].as_f64().unwrap_or(0.),
            w["at"][1].as_f64().unwrap_or(0.),
        );
        self.pointer.send(&format!(
            "scroll {} {} {notches}",
            ax + f64::from(x),
            ay + f64::from(y)
        ))
    }

    /// Where the app's window starts on the screenshot, in screen pixels.
    pub fn window_origin(&self, app: &App) -> anyhow::Result<(f32, f32)> {
        let w = self.window_of(app.pid)?;
        let monitors = self.nested_json("monitors")?;
        let m = monitors
            .as_array()
            .and_then(|m| m.iter().find(|m| m["name"] == "HEADLESS-2"))
            .context("no HEADLESS-2 monitor")?;
        let scale = m["scale"].as_f64().unwrap_or(1.);
        let dx = w["at"][0].as_f64().unwrap_or(0.) - m["x"].as_f64().unwrap_or(0.);
        let dy = w["at"][1].as_f64().unwrap_or(0.) - m["y"].as_f64().unwrap_or(0.);
        Ok(((dx * scale) as f32, (dy * scale) as f32))
    }

    /// The windows of class `class` on the user's own desktop (the oldest Hyprland instance, which is
    /// not the nested one). Zero is the point: nothing of ours reaches the real session.
    pub fn host_windows_of(&self, class: &str) -> anyhow::Result<usize> {
        let out = Process::new("hyprctl").args(["instances", "-j"]).output()?;
        let list: Value = serde_json::from_slice(&out.stdout)?;
        let mut hosts: Vec<(i64, String)> = list
            .as_array()
            .context("instances")?
            .iter()
            .filter(|i| i["instance"].as_str() != Some(self.nested.sig.as_str()))
            .map(|i| {
                (
                    i["time"].as_i64().unwrap_or(0),
                    i["instance"].as_str().unwrap_or("").to_owned(),
                )
            })
            .collect();
        hosts.sort();
        let Some((_, sig)) = hosts.first() else {
            return Ok(0);
        };
        let clients: Value = serde_json::from_slice(&self.hyprctl(sig, &["-j", "clients"])?)?;
        Ok(clients
            .as_array()
            .map_or(0, |c| c.iter().filter(|c| c["class"] == class).count()))
    }

    /// The nested compositor's own windows of class `class`.
    pub fn nested_windows_of(&self, class: &str) -> anyhow::Result<usize> {
        Ok(self
            .nested_json("clients")?
            .as_array()
            .map_or(0, |c| c.iter().filter(|c| c["class"] == class).count()))
    }

    /// Where a screenshot to keep goes.
    pub fn doc_shot(&self, name: &str) -> PathBuf {
        self.repo.join("docs/shots").join(name)
    }
}

impl App {
    pub fn stderr(&self) -> String {
        std::fs::read_to_string(&self.err_log).unwrap_or_default()
    }

    fn request(&mut self, line: &str) -> anyhow::Result<Value> {
        self.stream
            .set_read_timeout(Some(Duration::from_secs(30)))?;
        self.stream.write_all(format!("{line}\n").as_bytes())?;
        let mut reply = String::new();
        self.reader
            .read_line(&mut reply)
            .with_context(|| format!("{}: no reply to {line}", self.name))?;
        ensure!(
            !reply.is_empty(),
            "{}: the app closed the socket",
            self.name
        );
        let v: Value = serde_json::from_str(&reply)?;
        if let Some(e) = v.get("error") {
            bail!("{}: {e}", self.name);
        }
        Ok(v)
    }

    /// What the window shows now.
    pub fn state(&mut self) -> anyhow::Result<Value> {
        self.request(r#"{"cmd":"state"}"#)
    }

    /// Polls `state` until `done` holds, or fails with the last state after `timeout`.
    pub fn wait_for(
        &mut self,
        what: &str,
        timeout: Duration,
        mut done: impl FnMut(&Value) -> bool,
    ) -> anyhow::Result<Value> {
        let start = Instant::now();
        loop {
            let s = self.state()?;
            if done(&s) {
                return Ok(s);
            }
            if start.elapsed() > timeout {
                let lines: Vec<String> = s["lines"]
                    .as_array()
                    .map(|l| {
                        l.iter()
                            .map(|l| format!("{:?} => {} {}", l["text"], l["answer"], l["error"]))
                            .collect()
                    })
                    .unwrap_or_default();
                bail!(
                    "timed out waiting for {what}; lines: {lines:#?}; current {}; total {}; sheets {}",
                    s["current"],
                    s["total"],
                    s["sheets"]
                );
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    }

    fn check_stderr(&self) -> anyhow::Result<()> {
        let err = self.stderr();
        for m in PANIC_MARKERS {
            ensure!(
                !err.contains(m),
                "{}: stderr contains {m:?} ({})",
                self.name,
                self.err_log.display()
            );
        }
        Ok(())
    }

    /// Kills the app with SIGKILL, as a crash or a power cut would, by its own pid.
    pub fn kill_hard(mut self) -> anyhow::Result<()> {
        self.finished = true;
        let _ = Process::new("kill")
            .args(["-KILL", &self.pid.to_string()])
            .status();
        let start = Instant::now();
        while alive(self.pid) && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        ensure!(!alive(self.pid), "the app survived SIGKILL");
        self.check_stderr()
    }

    /// Waits for the app to exit on its own (after Ctrl+Q).
    pub fn wait_exit(mut self, timeout: Duration) -> anyhow::Result<()> {
        self.finished = true;
        let start = Instant::now();
        while alive(self.pid) && start.elapsed() < timeout {
            std::thread::sleep(Duration::from_millis(50));
        }
        if alive(self.pid) {
            let _ = Process::new("kill").arg(self.pid.to_string()).status();
            bail!("{}: the app did not exit within {timeout:?}", self.name);
        }
        self.check_stderr()
    }

    /// Asks the app to quit through the hook, waits, and fails if it panicked.
    pub fn finish(mut self) -> anyhow::Result<()> {
        let _ = self.request(r#"{"cmd":"quit"}"#);
        self.wait_exit(Duration::from_secs(8))
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.sock);
        // Only the app this harness started, by its own pid.
        if !self.finished && alive(self.pid) {
            let _ = Process::new("kill").arg(self.pid.to_string()).status();
        }
    }
}
