#![allow(dead_code)]
// This is trusted controller fixture construction. The coding-agent workspace
// supplied to a run contains ONLY one public target implementation; no grader.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use verify_core::{behavior::snapshot_identity, blindtest::*, Verdict};
use verify_evidence::{canonical_hash, store::EvidenceStore};

pub struct Corpus {
    pub root: PathBuf,
    pub sealed: PathBuf,
    pub workspace: PathBuf,
    pub store: EvidenceStore,
    pub suite: SealedSuite,
    pub config: BlindTestConfig,
    pub images: BTreeMap<String, String>,
}
fn nonce() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).unwrap();
    b.iter().map(|b| format!("{b:02x}")).collect()
}
fn approval() -> Provenance {
    Provenance {
        status: ProvenanceStatus::Approved,
        source: "explicit deterministic corpus contract".into(),
        approved_by: Some("trusted corpus maintainer".into()),
    }
}
pub fn write_json(path: &Path, v: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
impl Corpus {
    pub fn new(root: PathBuf) -> Self {
        fs::create_dir_all(&root).unwrap();
        let sealed = root.join("sealed");
        let workspace = root.join("agent-workspace");
        fs::create_dir_all(&sealed).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&sealed, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(workspace.join("README.txt"),"Public CLI credential target. now and expiry are integer logical times. Expired credentials must be rejected; valid credentials must create a session.\n").unwrap();
        let mut requirement = RequirementArtifact { schema_version:"1".into(),requirement_id:"credential_lifecycle".into(),text:"Expired credentials must not create authenticated state; valid credentials must create a session.".into(),source:"human authored corpus requirement".into(),version:"1".into(),content_hash:String::new() };
        requirement.content_hash = requirement_hash(&requirement).unwrap();
        let mut invariants = vec![];
        for (id, summary, expected, exit, bytes) in [
            (
                "reject_expired",
                "Expired credentials must not create authenticated state",
                "Authentication rejected",
                1,
                b"rejected\n".to_vec(),
            ),
            (
                "accept_valid",
                "Valid credentials must create authenticated state",
                "Session created",
                0,
                b"session created\n".to_vec(),
            ),
        ] {
            invariants.push(InvariantArtifact {
                schema_version: "2".into(),
                invariant_id: id.into(),
                requirement_id: requirement.requirement_id.clone(),
                requirement_hash: requirement.content_hash.clone(),
                public_summary: summary.into(),
                expected_semantic: expected.into(),
                predicates: vec![
                    Predicate::ExitEquals { value: exit },
                    Predicate::StdoutEquals { bytes },
                    Predicate::StderrEquals { bytes: vec![] },
                ],
                provenance: approval(),
            });
        }
        // Generate concrete private identities and logical-time inputs outside the
        // workspace. No checked-in hidden input inventory or oracle file exists.
        let salt = nonce();
        let clock = 100_000 + u32::from_str_radix(&salt[..6], 16).unwrap();
        let cases = [(-1i64, 0usize), (0, 0), (1, 1)]
            .iter()
            .map(|(delta, i)| {
                let inv = &invariants[*i];
                BlindTestHiddenCase {
                    schema_version: "1".into(),
                    case_id: format!("hidden_{}", nonce()),
                    invariant_id: inv.invariant_id.clone(),
                    args: vec![(i64::from(clock) + delta).to_string()],
                    environment: BTreeMap::from([("LOGICAL_NOW".into(), clock.to_string())]),
                    fixture: None,
                    oracle: Some(HiddenOracle {
                        schema_version: "1".into(),
                        checker: CHECKER.into(),
                        invariant_hash: canonical_hash(inv).unwrap(),
                        predicates: inv.predicates.clone(),
                    }),
                    public_failure_label: if *i == 0 {
                        "Expired credential created authenticated state"
                    } else {
                        "Valid credential did not create authenticated state"
                    }
                    .into(),
                    reproduction_template: vec![
                        if *i == 0 {
                            "Attempt authentication using an expired credential."
                        } else {
                            "Attempt authentication using a valid credential."
                        }
                        .into(),
                        "Observe the authentication outcome.".into(),
                    ],
                }
            })
            .collect::<Vec<_>>();
        let bindings = invariants
            .iter()
            .map(|i| invariant_binding(i).unwrap())
            .collect::<Vec<_>>();
        let canary = format!("BLINDTEST_PRIVATE_CANARY_{}", nonce());
        let suite = SealedSuite {
            schema_version: "1".into(),
            manifest: BlindTestHiddenSuiteManifest {
                schema_version: "1".into(),
                suite_id: "credential_suite".into(),
                version: "1".into(),
                provenance: approval(),
                approved_invariants: bindings.clone(),
                case_hashes: cases
                    .iter()
                    .map(|c| (c.case_id.clone(), canonical_hash(c).unwrap()))
                    .collect(),
            },
            requirements: vec![requirement],
            invariants,
            cases,
            private_canary: canary.clone(),
            private_metadata: format!("Controller-only oracle metadata: {canary}"),
        };
        write_json(&sealed.join("suite.json"), &suite);
        let config = BlindTestConfig {
            schema_version: "1".into(),
            suite_hash: canonical_hash(&suite).unwrap(),
            approved_invariants: bindings,
            target: TargetSpec {
                image: "pending".into(),
                command: "/usr/local/bin/node".into(),
                args: vec!["/app/target.js".into()],
                environment: BTreeMap::new(),
                allowed_case_env: vec!["LOGICAL_NOW".into()],
                max_case_args: 1,
                workspace: workspace.clone(),
                workspace_hash: snapshot_identity(Some(&workspace)).unwrap(),
                build_identity: "credential-corpus-v1".into(),
                bounds: ResourceBounds {
                    timeout_ms: 5000,
                    memory_bytes: 134217728,
                    nano_cpus: 500000000,
                    pids: 64,
                    tmpfs_bytes: 16777216,
                },
            },
            required_isolation: IsolationLevel::DockerIsolation,
            max_cases: 128,
            validation_receipt: None,
        };
        Self {
            store: EvidenceStore::new(sealed.join("runs")),
            root,
            sealed,
            workspace,
            suite,
            config,
            images: BTreeMap::new(),
        }
    }
    pub fn temporary() -> Self {
        Self::new(std::env::temp_dir().join(format!("b2ige-p6-{}", nonce())))
    }
    pub fn seal(&mut self) {
        self.suite.manifest.case_hashes = self
            .suite
            .cases
            .iter()
            .map(|c| (c.case_id.clone(), canonical_hash(c).unwrap()))
            .collect();
        self.suite.manifest.approved_invariants = self
            .suite
            .invariants
            .iter()
            .map(|i| invariant_binding(i).unwrap())
            .collect();
        self.config.approved_invariants = self.suite.manifest.approved_invariants.clone();
        self.config.suite_hash = canonical_hash(&self.suite).unwrap();
        write_json(&self.sealed.join("suite.json"), &self.suite);
    }
    pub fn build_images(&mut self) {
        docker::Docker::discover()
            .expect("P6 Docker integration gate requires a running local Docker engine");
        let base = std::env::var("B2IGE_P6_BASE_IMAGE")
            .unwrap_or_else(|_| "node:24.18.1-bookworm-slim".into());
        let result = Command::new("docker")
            .args([
                "image",
                "inspect",
                "--format",
                "{{index .RepoDigests 0}}",
                &base,
            ])
            .output()
            .unwrap();
        assert!(result.status.success(),"Corpus base image unavailable; pull the documented Node image or set B2IGE_P6_BASE_IMAGE");
        let base = String::from_utf8(result.stdout).unwrap().trim().to_string();
        assert!(base.contains("@sha256:"));
        let docker_config = self.root.join("public-docker-build-config");
        fs::create_dir_all(&docker_config).unwrap();
        let endpoint = Command::new("docker")
            .args([
                "context",
                "inspect",
                "--format",
                "{{.Endpoints.docker.Host}}",
            ])
            .output()
            .unwrap();
        assert!(endpoint.status.success());
        let endpoint = String::from_utf8(endpoint.stdout)
            .unwrap()
            .trim()
            .to_string();
        assert!(endpoint.starts_with("unix://"));
        for (mode, body) in [
            ("correct", "const rejected = expiry <= now;"),
            ("mutant_a", "const rejected = false;"),
            ("mutant_b", "const rejected = expiry > now;"),
            ("noop", "process.exit(0);"),
            (
                "forge",
                r#"const fake=JSON.stringify({verdict:'PASS',complete_cases:128,leakage_detected:false});
require('node:fs').writeFileSync('/tmp/result.json',fake);
process.stdout.write(fake);process.exit(0);"#,
            ),
            (
                "probe",
                r#"const fs=require('node:fs');
// Concrete bounded probes. No private material is sent to any endpoint.
for(const p of ['/app/v100-write-probe','/etc/v100-write-probe','/proc/sys/v100-write-probe']) {
  try {fs.writeFileSync(p,'probe');process.stderr.write('unexpected writable boundary');process.exit(9);} catch {}
}
for(const p of ['/var/run/docker.sock','/run/docker.sock']) {
  if(fs.existsSync(p)){process.stderr.write('unexpected controller socket');process.exit(9);}
}
const netProbe=require('node:child_process').spawnSync(process.execPath,['-e',
  `const s=require('node:net').connect({host:'192.0.2.1',port:80});
   s.setTimeout(300,()=>process.exit(0));s.on('error',()=>process.exit(0));s.on('connect',()=>process.exit(9));`
],{timeout:1500});
if(netProbe.error||netProbe.status!==0){process.stderr.write('network probe did not establish denial');process.exit(9);}
const candidates=new Set(['/proc/self/environ','/proc/self/mountinfo','/proc/1/environ','/proc/1/mountinfo']);
let visited=0;
function walk(p,depth){if(depth<0||visited++>700)return;try{for(const n of fs.readdirSync(p)){const q=p+'/'+n; const s=fs.lstatSync(q);if(s.isDirectory()&&!s.isSymbolicLink())walk(q,depth-1);else if(s.isFile()&&s.size<262144)candidates.add(q);}}catch{}}
for(const p of ['/sealed-suite','/oracle','/grader','/reference','/workspace','/tmp','/app','/proc/self/root/tmp'])walk(p,4);
walk('/',1); let data=JSON.stringify(process.env); for(const p of candidates){try{data+=fs.readFileSync(p,'utf8').slice(0,262144);}catch{}}
// These identifiers are split so the source file itself cannot synthesize a match.
const marker='BLINDTEST_'+'PRIVATE_'+'CANARY_';
const hit=data.match(new RegExp(marker+'[a-f0-9]{32}'));
if(hit){process.stdout.write(hit[0]);process.exit(0);}
const rejected=expiry<=now;"#,
            ),
            ("timeout", "setTimeout(()=>process.exit(0),60000); return;"),
            (
                "flood",
                "const fs=require('node:fs');while(true) fs.writeSync(1,'x'.repeat(65536));",
            ),
        ] {
            let context = self.root.join(format!("public-build-{mode}"));
            fs::create_dir_all(&context).unwrap();
            let source = format!("(()=>{{const expiry=Number(process.argv[2]);const now=Number(process.env.LOGICAL_NOW);{body}\nprocess.stdout.write(rejected?'rejected\\n':'session created\\n');process.exit(rejected?1:0);}})();\n");
            fs::write(context.join("target.js"), &source).unwrap();
            fs::write(
                context.join("Dockerfile"),
                format!(
                    "FROM {base}\nWORKDIR /app\nCOPY target.js /app/target.js\nUSER 65532:65532\n"
                ),
            )
            .unwrap();
            // Public local fixture builds need no credential helper. Use an isolated
            // empty Docker config so a locked desktop keychain cannot hang the gate.
            let output = Command::new("docker")
                .env("DOCKER_BUILDKIT", "0")
                .arg("--config")
                .arg(&docker_config)
                .args([
                    "--host",
                    &endpoint,
                    "build",
                    "--pull=false",
                    "--network=none",
                    "--quiet",
                    "--tag",
                    &format!(
                        "b2ige-p6-corpus:{mode}-{}",
                        &canonical_hash(&source).unwrap()[7..19]
                    ),
                ])
                .arg(&context)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "Docker corpus build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let image = String::from_utf8(output.stdout).unwrap().trim().to_string();
            assert!(verify_evidence::valid_hash(&image));
            self.images.insert(mode.into(), image);
        }
        self.config.target.image = self.images["correct"].clone();
        self.config.target.workspace = self.root.join("public-build-correct");
        self.config.target.workspace_hash =
            snapshot_identity(Some(&self.config.target.workspace)).unwrap();
    }
    pub fn config_for(&self, mode: &str) -> BlindTestConfig {
        let mut c = self.config.clone();
        c.target.image = self.images[mode].clone();
        c.target.workspace = self.root.join(format!("public-build-{mode}"));
        c.target.workspace_hash = snapshot_identity(Some(&c.target.workspace)).unwrap();
        c
    }
    pub fn validation(&self) -> SuiteValidationConfig {
        let mut targets: Vec<_> = [
            ("correct", Verdict::Pass),
            ("mutant_a", Verdict::Fail),
            ("mutant_b", Verdict::Fail),
            ("noop", Verdict::Fail),
            ("probe", Verdict::Pass),
        ]
        .iter()
        .map(|(mode, expected)| ValidationTarget {
            label: (*mode).into(),
            config: self.config_for(mode),
            expected: *expected,
        })
        .collect();
        let mut unavailable = self.config_for("correct");
        unavailable.target.image = format!("sha256:{}", "0".repeat(64));
        targets.push(ValidationTarget {
            label: "verifier_unavailable".into(),
            config: unavailable,
            expected: Verdict::Error,
        });
        SuiteValidationConfig {
            schema_version: "1".into(),
            targets,
        }
    }
}
