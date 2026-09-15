//! Docker CLI acquisition reuses the P1 bounded process observer. Only actual
//! inspect responses attest settings; no user-provided success flag is accepted.
use super::*;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};
use verify_runner::process::{observe, ProcessObservation, ProcessSpec};

pub struct Docker {
    executable: PathBuf,
    environment: BTreeMap<String, String>,
}
impl Docker {
    pub fn discover() -> io::Result<Self> {
        let executable = if let Some(p) = std::env::var_os("B2IGE_DOCKER") {
            PathBuf::from(p)
        } else {
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .map(|p| p.join("docker"))
                .find(|p| p.is_file())
                .ok_or_else(|| invalid("Docker unavailable"))?
        };
        if !executable.is_absolute() || !executable.is_file() {
            return Err(invalid("Docker unavailable"));
        }
        let environment = [
            "HOME",
            "PATH",
            "DOCKER_CONFIG",
            "DOCKER_CONTEXT",
            "DOCKER_HOST",
        ]
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
        .collect();
        let docker = Self {
            executable,
            environment,
        };
        if std::env::var("DOCKER_HOST").is_ok_and(|v| !v.starts_with("unix://")) {
            return Err(invalid("P6 requires a local Unix Docker endpoint"));
        }
        let context = docker.json(&["context".into(), "inspect".into()])?;
        if !context[0]["Endpoints"]["docker"]["Host"]
            .as_str()
            .is_some_and(|s| s.starts_with("unix://"))
        {
            return Err(invalid("P6 requires local Docker context"));
        }
        let version = docker.json(&["version".into(), "--format".into(), "{{json .}}".into()])?;
        if version["Server"]["Os"] != "linux" {
            return Err(invalid("Linux Docker engine unavailable"));
        }
        Ok(docker)
    }
    fn call(&self, args: &[String], timeout_ms: u64) -> ProcessObservation {
        observe(&ProcessSpec {
            executable: self.executable.clone(),
            args: args.to_vec(),
            working_directory: PathBuf::from("/"),
            environment: self.environment.clone(),
            timeout_ms,
        })
    }
    fn success(&self, args: &[String]) -> io::Result<Vec<u8>> {
        let o = self.call(args, 30_000);
        if !o.started || o.timed_out || o.runner_failure.is_some() || o.exit_code != Some(0) {
            return Err(invalid("Docker operation failed"));
        }
        Ok(o.stdout)
    }
    fn json(&self, args: &[String]) -> io::Result<Value> {
        Ok(serde_json::from_slice(&self.success(args)?)?)
    }
    fn inspect(&self, id: &str) -> io::Result<Value> {
        self.json(&["container".into(), "inspect".into(), id.into()])?
            .as_array()
            .filter(|v| v.len() == 1)
            .map(|v| v[0].clone())
            .ok_or_else(|| invalid("container inspect unavailable"))
    }
    pub fn resolve_image(&self, input: &str) -> io::Result<ImageIdentity> {
        let values = self.json(&["image".into(), "inspect".into(), "--".into(), input.into()])?;
        let inspect = values
            .as_array()
            .filter(|v| v.len() == 1)
            .map(|v| v[0].clone())
            .ok_or_else(|| invalid("immutable image resolution failed"))?;
        let image = ImageIdentity {
            image_id: inspect["Id"].as_str().unwrap_or_default().into(),
            platform: format!(
                "{}/{}",
                inspect["Os"].as_str().unwrap_or_default(),
                inspect["Architecture"].as_str().unwrap_or_default()
            ),
            inspect,
        };
        validate_image(&image)?;
        Ok(image)
    }
    pub fn execute(
        &self,
        c: &BlindTestConfig,
        image: &ImageIdentity,
        case: &BlindTestHiddenCase,
    ) -> io::Result<HiddenCaseExecution> {
        let b = &c.target.bounds;
        let mut args: Vec<String> = [
            "create",
            "--network",
            "none",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--user",
            "65532:65532",
            "--ipc",
            "private",
            "--cgroupns",
            "private",
            "--shm-size",
            "8388608",
            "--workdir",
            "/tmp",
            "--log-driver",
            "none",
            "--restart",
            "no",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        args.extend([
            "--pids-limit".into(),
            b.pids.to_string(),
            "--memory".into(),
            b.memory_bytes.to_string(),
            "--memory-swap".into(),
            b.memory_bytes.to_string(),
            "--cpus".into(),
            format!(
                "{}.{:09}",
                b.nano_cpus / 1_000_000_000,
                b.nano_cpus % 1_000_000_000
            ),
            "--tmpfs".into(),
            format!("/tmp:{}", tmpfs(b)),
            "--entrypoint".into(),
            c.target.command.clone(),
        ]);
        for (key, value) in input_env(c, case)? {
            args.extend(["--env".into(), format!("{key}={value}")]);
        }
        args.push(image.image_id.clone());
        args.extend(c.target.args.clone());
        args.extend(case.args.clone());
        let created = String::from_utf8(self.success(&args)?)
            .map_err(|_| invalid("invalid container identity"))?;
        let id = created.trim().to_string();
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("invalid container identity"));
        }
        let mut guard = ContainerGuard {
            docker: self,
            id: id.clone(),
            removed: false,
        };
        let before = self.inspect(&id)?;
        validate_settings(&before, c, image, case)?;
        if before["State"]["Status"] != "created" {
            return Err(invalid("container was already executed"));
        }
        let capture = self.call(
            &["start".into(), "--attach".into(), id.clone()],
            b.timeout_ms,
        );
        if capture.timed_out || capture.runner_failure.is_some() || capture.signal.is_some() {
            let _ = self.success(&["kill".into(), id.clone()]);
        }
        let after = self.inspect(&id)?;
        let attestation = BlindTestIsolationAttestation {
            schema_version: "1".into(),
            required: c.required_isolation,
            observed: IsolationLevel::DockerIsolation,
            controller_platform: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
            container_id: id.clone(),
            before,
            after,
        };
        validate_attestation(&attestation, c, image, case)?;
        self.success(&["rm".into(), "--force".into(), id])?;
        guard.removed = true;
        Ok(HiddenCaseExecution {
            case_id: case.case_id.clone(),
            image_id: image.image_id.clone(),
            attestation,
            capture,
            cleanup_confirmed: true,
        })
    }
}
struct ContainerGuard<'a> {
    docker: &'a Docker,
    id: String,
    removed: bool,
}
impl Drop for ContainerGuard<'_> {
    fn drop(&mut self) {
        if !self.removed {
            let _ = self
                .docker
                .success(&["rm".into(), "--force".into(), self.id.clone()]);
        }
    }
}
fn empty(v: &Value) -> bool {
    v.is_null()
        || v.as_array().is_some_and(Vec::is_empty)
        || v.as_object().is_some_and(serde_json::Map::is_empty)
}
fn tmpfs(b: &ResourceBounds) -> String {
    format!("rw,noexec,nosuid,nodev,size={}", b.tmpfs_bytes)
}
fn input_env(
    c: &BlindTestConfig,
    case: &BlindTestHiddenCase,
) -> io::Result<BTreeMap<String, String>> {
    let mut env = c.target.environment.clone();
    env.extend(case.environment.clone());
    if let Some(f) = &case.fixture {
        env.insert("B2IGE_FIXTURE_NAME".into(), f.name.clone());
        env.insert(
            "B2IGE_FIXTURE_BYTES_JSON".into(),
            serde_json::to_string(&f.bytes)?,
        );
    }
    Ok(env)
}
fn env_map(v: &Value) -> io::Result<BTreeMap<String, String>> {
    let list = v
        .as_array()
        .ok_or_else(|| invalid("image/container environment missing"))?;
    let mut out = BTreeMap::new();
    for e in list {
        let (key, val) = e
            .as_str()
            .and_then(|s| s.split_once('='))
            .ok_or_else(|| invalid("invalid environment entry"))?;
        if out.insert(key.into(), val.into()).is_some() {
            return Err(invalid("duplicate environment entry"));
        }
    }
    Ok(out)
}
pub fn validate_image(image: &ImageIdentity) -> io::Result<()> {
    // Image config omits optional Volumes on current OCI images. The actual
    // container Mounts/Volumes observations below must be present and empty.
    if !verify_evidence::valid_hash(&image.image_id)
        || image.inspect["Id"] != image.image_id
        || image.inspect["Os"] != "linux"
        || !image.inspect["Architecture"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
        || image.platform
            != format!(
                "linux/{}",
                image.inspect["Architecture"].as_str().unwrap_or_default()
            )
        || !empty(&image.inspect["Config"]["Volumes"])
        || !empty(&image.inspect["Config"]["OnBuild"])
    {
        return Err(invalid(
            "invalid immutable Linux image or undeclared volumes",
        ));
    }
    env_map(&image.inspect["Config"]["Env"])?;
    Ok(())
}
/// This checks observed settings, before AND after execution. No mounts at all are
/// needed in the MVP (public sources are built into the pinned image).
pub fn validate_settings(
    v: &Value,
    c: &BlindTestConfig,
    image: &ImageIdentity,
    case: &BlindTestHiddenCase,
) -> io::Result<()> {
    let h = &v["HostConfig"];
    let config = &v["Config"];
    let b = &c.target.bounds;
    let mut expected_args = c.target.args.clone();
    expected_args.extend(case.args.clone());
    let mut env = env_map(&image.inspect["Config"]["Env"])?;
    env.extend(input_env(c, case)?);
    // Null is an explicit Docker value for several empty settings. An absent
    // verdict-critical field is missing evidence and must not mean empty/safe.
    let required_empty_fields = [
        "Binds",
        "VolumesFrom",
        "CapAdd",
        "Devices",
        "DeviceRequests",
        "DeviceCgroupRules",
        "GroupAdd",
        "Links",
        "ExtraHosts",
        "PortBindings",
    ];
    if !v["Mounts"].as_array().is_some_and(Vec::is_empty)
        || required_empty_fields
            .iter()
            .any(|key| h.get(*key).is_none())
        || config.get("Volumes").is_none()
        || v["NetworkSettings"].get("Ports").is_none()
        || v["Image"] != image.image_id
        || v["Platform"] != "linux"
        || v["Path"] != c.target.command
        || v["Args"] != json!(expected_args)
        || config["Image"] != image.image_id
        || config["Entrypoint"] != json!([c.target.command])
        || config["Cmd"] != json!(expected_args)
        || config["User"] != "65532:65532"
        || config["WorkingDir"] != "/tmp"
        || config["Tty"] != false
        || config["OpenStdin"] != false
        || config["AttachStdout"] != true
        || config["AttachStderr"] != true
        || !empty(&config["Volumes"])
        || env_map(&config["Env"])? != env
        || h["NetworkMode"] != "none"
        || h["Privileged"] != false
        || h["ReadonlyRootfs"] != true
        || !empty(&v["Mounts"])
        || !empty(&h["Mounts"])
        || !empty(&h["Binds"])
        || !empty(&h["VolumesFrom"])
        || h["CapDrop"] != json!(["ALL"])
        || !empty(&h["CapAdd"])
        || h["SecurityOpt"] != json!(["no-new-privileges"])
        || h["PidsLimit"] != b.pids
        || h["Memory"] != b.memory_bytes
        || h["MemorySwap"] != b.memory_bytes
        || h["NanoCpus"] != b.nano_cpus
        || h["Tmpfs"] != json!({"/tmp":tmpfs(b)})
        || h["ShmSize"] != 8388608
        || h["PidMode"] != ""
        || h["IpcMode"] != "private"
        || h["UTSMode"] != ""
        || h["UsernsMode"] != ""
        || h["CgroupnsMode"] != "private"
        || !empty(&h["Devices"])
        || !empty(&h["DeviceRequests"])
        || !empty(&h["DeviceCgroupRules"])
        || !empty(&h["GroupAdd"])
        || !empty(&h["Links"])
        || !empty(&h["ExtraHosts"])
        || !empty(&h["PortBindings"])
        || h["PublishAllPorts"] != false
        || h["RestartPolicy"]["Name"] != "no"
        || h["LogConfig"]["Type"] != "none"
        || !matches!(
            h.get("OomKillDisable"),
            Some(Value::Bool(false) | Value::Null)
        )
        || !h["ReadonlyPaths"]
            .as_array()
            .is_some_and(|v| v.contains(&json!("/proc/sys")))
        || !h["MaskedPaths"]
            .as_array()
            .is_some_and(|v| v.contains(&json!("/proc/kcore")))
        || !v["NetworkSettings"]["Networks"]
            .as_object()
            .is_some_and(|n| n.len() == 1 && n.contains_key("none"))
        || !empty(&v["NetworkSettings"]["Ports"])
    {
        return Err(invalid(
            "observed Docker isolation/target settings do not satisfy plan",
        ));
    }
    Ok(())
}
pub fn validate_attestation(
    a: &BlindTestIsolationAttestation,
    c: &BlindTestConfig,
    image: &ImageIdentity,
    case: &BlindTestHiddenCase,
) -> io::Result<()> {
    if a.schema_version != "1"
        || a.required != c.required_isolation
        || a.observed != IsolationLevel::DockerIsolation
        || a.required != a.observed
        || a.container_id.len() != 64
        || !a.container_id.bytes().all(|b| b.is_ascii_hexdigit())
        || a.before["Id"] != a.container_id
        || a.after["Id"] != a.container_id
        || a.before["State"]["Status"] != "created"
        || a.before["State"]["Running"] != false
        || a.after["State"]["Status"] != "exited"
        || a.after["State"]["Running"] != false
        || a.after["State"]["Error"] != ""
        || !a.after["State"]["StartedAt"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && !s.starts_with("0001-"))
        || !a.after["State"]["FinishedAt"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && !s.starts_with("0001-"))
        || !a.after["State"]["ExitCode"]
            .as_i64()
            .is_some_and(|e| (0..=255).contains(&e))
        || !nonempty(&a.controller_platform)
    {
        return Err(invalid(
            "actual Docker lifecycle/isolation evidence unavailable",
        ));
    }
    validate_settings(&a.before, c, image, case)?;
    validate_settings(&a.after, c, image, case)
}
pub fn doctor() -> DoctorReport {
    let docker = Docker::discover();
    let version = docker.as_ref().ok().and_then(|d| {
        d.json(&["version".into(), "--format".into(), "{{json .}}".into()])
            .ok()
    });
    let info = docker.as_ref().ok().and_then(|d| {
        d.json(&["info".into(), "--format".into(), "{{json .}}".into()])
            .ok()
    });
    let reported_security_options: Vec<String> = info
        .as_ref()
        .and_then(|v| v["SecurityOptions"].as_array())
        .map(|v| {
            v.iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let cgroup_version = info
        .as_ref()
        .and_then(|v| v["CgroupVersion"].as_str().map(str::to_owned));
    let capabilities = version.is_some()
        && cgroup_version
            .as_ref()
            .is_some_and(|v| v == "1" || v == "2")
        && reported_security_options
            .iter()
            .any(|v| v.starts_with("name=seccomp"));
    DoctorReport {
        schema_version: "1".into(),
        docker_available: version.is_some(),
        docker_version: version.as_ref().and_then(|v|v["Server"]["Version"].as_str().map(str::to_owned)),
        target_platform: version.as_ref().map(|v|format!("{}/{}",v["Server"]["Os"].as_str().unwrap_or_default(),v["Server"]["Arch"].as_str().unwrap_or_default())),
        supported_isolation: if capabilities {IsolationLevel::DockerIsolation} else {IsolationLevel::None},
        path_rules: "Canonical workspace, sealed root and private evidence store must not overlap. No container mounts. Suite files must be regular files without aliases.".into(),
        required_capabilities: vec![
            "Local Unix Docker endpoint with Linux engine".into(),
            "Immutable image inspect; container create/start/inspect/kill/rm".into(),
            "Network none; non-root; read-only; tmpfs; cap-drop ALL; no-new-privileges".into(),
            "Bounded memory, CPU, pids, timeout and 1 MiB per output stream".into(),
        ],
        reported_security_options,
        cgroup_version,
        runtime_capabilities_available: capabilities,
        attestation_required_per_run: true,
    }
}
