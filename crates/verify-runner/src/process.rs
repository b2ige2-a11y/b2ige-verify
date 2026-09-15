//! CLI observer v1. Byte-exact streams, per-stream ordering only. Unix process
//! groups are cleanup, not a sandbox; deliberately detached descendants are outside
//! this boundary. Wall timestamps use Unix epoch nanoseconds; deadlines are monotonic.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const OBSERVER: &str = "cli_process";
pub const CAPTURE_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessSpec {
    /// Absolute executable path avoids implicit PATH/cwd resolution.
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    /// Environment is cleared before these explicit entries are installed.
    pub environment: BTreeMap<String, String>,
    pub timeout_ms: u64,
}
impl ProcessSpec {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.executable.is_absolute() || !self.working_directory.is_absolute() {
            return Err("executable and working directory must be absolute");
        }
        if self.timeout_ms == 0 || self.timeout_ms > verify_evidence::MAX_SAFE_INTEGER {
            return Err("invalid timeout");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessObservation {
    pub started: bool,
    pub started_at: String,
    pub ended_at: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub runner_failure: Option<String>,
}

pub fn timestamp() -> String {
    format!(
        "unix-ns:{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

/// Execute a single target. A non-zero target status is a successful observation.
pub fn observe(spec: &ProcessSpec) -> ProcessObservation {
    observe_controlled(spec, &mut || Ok(false)).observation
}

/// Optional local controller, polled only while the direct child is alive.
/// A request is not proof of termination: consumers must also verify SIGKILL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlledProcessObservation {
    pub observation: ProcessObservation,
    pub termination_requested: bool,
    pub termination_delivered: bool,
}

pub fn observe_controlled(
    spec: &ProcessSpec,
    controller: &mut dyn FnMut() -> std::io::Result<bool>,
) -> ControlledProcessObservation {
    let mut termination_requested = false;
    let mut termination_delivered = false;
    let mut out = ProcessObservation {
        started: false,
        started_at: timestamp(),
        ended_at: String::new(),
        exit_code: None,
        signal: None,
        stdout: vec![],
        stderr: vec![],
        timed_out: false,
        runner_failure: None,
    };
    let result = spec
        .validate()
        .map_err(std::io::Error::other)
        .and_then(|()| {
            execute(
                spec,
                &mut out,
                controller,
                &mut termination_requested,
                &mut termination_delivered,
            )
        });
    if let Err(error) = result {
        out.runner_failure = Some(error.to_string());
    }
    out.ended_at = timestamp();
    ControlledProcessObservation {
        observation: out,
        termination_requested,
        termination_delivered,
    }
}

#[cfg(not(unix))]
fn execute(
    _: &ProcessSpec,
    _: &mut ProcessObservation,
    _: &mut dyn FnMut() -> std::io::Result<bool>,
    _: &mut bool,
    _: &mut bool,
) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "P1B process cleanup backend requires Unix",
    ))
}

#[cfg(unix)]
fn execute(
    spec: &ProcessSpec,
    out: &mut ProcessObservation,
    controller: &mut dyn FnMut() -> std::io::Result<bool>,
    termination_requested: &mut bool,
    termination_delivered: &mut bool,
) -> std::io::Result<()> {
    use std::io::{self, Read};
    use std::os::fd::AsRawFd;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    struct Guard(Child, bool);
    impl Guard {
        fn terminate_group(&mut self) -> io::Result<bool> {
            if self.1 {
                return Ok(false);
            }
            // SAFETY: the child was spawned into its own process group; negative
            // PID addresses only that group. ESRCH means it has already exited.
            let result = unsafe { libc::kill(-(self.0.id() as i32), libc::SIGKILL) };
            if result == -1 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
            self.1 = true;
            Ok(result == 0)
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.terminate_group();
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn nonblocking(pipe: &impl AsRawFd) -> io::Result<()> {
        // SAFETY: the live pipe owns this valid descriptor; flags are preserved.
        let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
        if flags == -1
            || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                == -1
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn drain(pipe: &mut impl Read, bytes: &mut Vec<u8>) -> io::Result<bool> {
        let mut buffer = [0; 8192];
        // Bounded per iteration: a flooding stream cannot starve the deadline.
        for _ in 0..16 {
            match pipe.read(&mut buffer) {
                Ok(0) => return Ok(true),
                Ok(n) => {
                    if bytes.len() + n > CAPTURE_LIMIT {
                        return Err(io::Error::other(
                            "capture limit exceeded; observation incomplete",
                        ));
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(false)
    }

    let start = Instant::now();
    let mut child = Guard(
        Command::new(&spec.executable)
            .args(&spec.args)
            .current_dir(&spec.working_directory)
            .env_clear()
            .envs(&spec.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?,
        false,
    );
    out.started = true;
    let mut stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("stdout pipe missing"))?;
    let mut stderr = child
        .0
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("stderr pipe missing"))?;
    nonblocking(&stdout)?;
    nonblocking(&stderr)?;
    let mut status = None;
    let mut cleaned = false;
    loop {
        let stdout_done = drain(&mut stdout, &mut out.stdout)?;
        let stderr_done = drain(&mut stderr, &mut out.stderr)?;
        if status.is_none() {
            status = child.0.try_wait()?;
        }
        if status.is_some() && stdout_done && stderr_done {
            break;
        }
        if status.is_none() && !*termination_requested && controller()? {
            *termination_requested = true;
            // The observer may have taken time. Reap a target that completed
            // during that window; a request after completion is not an executed fault.
            status = child.0.try_wait()?;
            if status.is_none() {
                *termination_delivered = child.terminate_group()?;
            }
        }
        if start.elapsed() >= Duration::from_millis(spec.timeout_ms) {
            out.timed_out = true;
            child.terminate_group()?;
            status = Some(child.0.wait()?);
            // Drain finite kernel buffers after killing writers. A detached writer
            // must not keep the verifier blocked or masquerade as complete capture.
            if let Some(exit) = status {
                out.exit_code = exit.code();
                out.signal = exit.signal();
            }
            let cleanup_started = Instant::now();
            loop {
                let a = drain(&mut stdout, &mut out.stdout)?;
                let b = drain(&mut stderr, &mut out.stderr)?;
                if a && b {
                    break;
                }
                if cleanup_started.elapsed() >= Duration::from_millis(250) {
                    return Err(io::Error::other(
                        "capture did not close after timeout cleanup",
                    ));
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            break;
        }
        if status.is_some() && !cleaned {
            child.terminate_group()?;
            cleaned = true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    child.terminate_group()?;
    let status = status.ok_or_else(|| io::Error::other("missing exit status"))?;
    out.exit_code = status.code();
    out.signal = status.signal();
    Ok(())
}
