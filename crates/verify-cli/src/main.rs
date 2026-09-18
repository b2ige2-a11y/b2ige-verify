use std::{
    io,
    path::PathBuf,
    process::{Command, ExitCode},
};
use verify_cli::{load, pretty, resolve, viewer::Viewer};
use verify_core::behavior::BehaviorAuthorization;
use verify_evidence::store::EvidenceStore;
const USAGE: &str = "Usage: b2ige bench [behavior|sideeffect|blindtest] [--output human|json] [--save DIRECTORY] [--reverse] [--case ID]\n       b2ige init [--dry-run]\n       b2ige setup [--dry-run]\n       b2ige doctor [--config project.json]\n       b2ige behavior verify <config.json> [--authorization FILE] [--store PATH] [--output human|json|agent] [--protocol 1]\n       b2ige blindtest doctor\n       b2ige blindtest verify <config.json> [--output human|json|agent] [--protocol 1] [--open]\n       b2ige blindtest validate-suite <validation-config.json>\n       B2IGE_BLINDTEST_SEALED_ROOT points to the trusted private suite directory; default store is its runs directory.\n        b2ige sideeffect verify <contract.json> [--store PATH] [--output human|json|agent] [--protocol 1] [--open]\n       b2ige report <artifact-id|store/id/result.json> [--store PATH] [--authorization FILE] [--output human|json|agent] [--open]\nDefaults: --store .b2ige/runs --authorization .b2ige/authorization.json\nVerdict exit codes: 0 PASS, 1 FAIL, 2 INCONCLUSIVE, 3 ERROR. Argument misuse: 64.\n--open serves localhost until interrupted; output and verdict are emitted before serving.";
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "ci") {
        return ExitCode::from(verify_cli::ci::cli(&args[1..]));
    }
    if let Some(code) = verify_cli::adoption::cli(&args) {
        return ExitCode::from(code);
    }
    if args == ["--version"] {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args.first().is_some_and(|s| s == "bench") {
        return ExitCode::from(verify_cli::bench::cli(&args[1..]));
    }
    if args == ["--help"]
        || (args.len() == 2
            && [
                "report",
                "behavior",
                "sideeffect",
                "blindtest",
                "init",
                "setup",
                "doctor",
            ]
            .contains(&args[0].as_str())
            && args[1] == "--help")
    {
        println!(
            "{USAGE}\n{}\n{}",
            verify_cli::adoption::HELP,
            verify_cli::ci::HELP
        );
        return ExitCode::SUCCESS;
    }
    if args == ["blindtest", "doctor"] {
        let doctor = verify_core::blindtest::docker::doctor();
        println!("{}", pretty(&doctor));
        return ExitCode::from(
            if doctor.docker_available && doctor.runtime_capabilities_available {
                0
            } else {
                3
            },
        );
    }
    if args.first().is_some_and(|s| s == "ci-check") {
        if args.len() != 3 {
            return ExitCode::from(64);
        }
        let Ok(code) = args[2].parse::<i32>() else {
            return ExitCode::from(64);
        };
        let r = verify_cli::agent::ci_check(&std::fs::read(&args[1]).unwrap_or_default(), code);
        println!("{}", pretty(&r));
        return ExitCode::from(r.verdict.exit_code());
    }
    if args.first().is_some_and(|s| s == "init" || s == "setup") {
        let mut root = None;
        let mut dry = false;
        for arg in &args[1..] {
            if arg == "--dry-run" && !dry {
                dry = true;
            } else if !arg.starts_with('-') && root.is_none() {
                root = Some(arg.as_str());
            } else {
                return ExitCode::from(64);
            }
        }
        let setup = args[0] == "setup";
        let result = if setup {
            verify_cli::integration::setup(std::path::Path::new(root.unwrap_or(".")), dry)
        } else {
            verify_cli::integration::init(std::path::Path::new(root.unwrap_or(".")), dry)
        };
        return match result {
            Ok(v) => {
                println!("{}", pretty(&v));
                ExitCode::SUCCESS
            }
            Err(_) => {
                eprintln!(
                    "{} failed: existing configuration is never overwritten",
                    if setup { "Setup" } else { "Init" }
                );
                ExitCode::from(3)
            }
        };
    }
    if args.first().is_some_and(|s| s == "doctor") {
        let mut path = ".b2ige/project.json";
        let mut output = "json";
        let mut seen = std::collections::BTreeSet::new();
        for pair in args[1..].chunks(2) {
            if pair.len() != 2 || !seen.insert(pair[0].as_str()) {
                return ExitCode::from(64);
            }
            match pair[0].as_str() {
                "--config" | "--registry" => path = &pair[1],
                "--output" if ["human", "json"].contains(&pair[1].as_str()) => output = &pair[1],
                _ => return ExitCode::from(64),
            }
        }
        if seen.contains("--config") && seen.contains("--registry") {
            return ExitCode::from(64);
        }
        let v = verify_cli::integration::project_doctor(std::path::Path::new(path));
        let code = if v["ready"] == true { 0 } else { 3 };
        if output == "human" {
            print!("{}", verify_cli::adoption::doctor_human(&v));
        } else {
            println!("{}", pretty(&v));
        }
        return ExitCode::from(code);
    }
    let options = match parse(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::from(64);
        }
    };
    let machine_output = options.output != "human";
    let product = if options.behavior {
        verify_cli::agent::Product::Behavior
    } else if options.blindtest.is_some() {
        verify_cli::agent::Product::Blindtest
    } else if options.verify {
        verify_cli::agent::Product::Sideeffect
    } else {
        verify_cli::agent::Product::Behavior
    };
    let operation = if options.behavior || options.verify || options.blindtest.is_some() {
        verify_cli::agent::Operation::Verify
    } else {
        verify_cli::agent::Operation::Report
    };
    match run(options) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            if machine_output {
                println!(
                    "{}",
                    pretty(&verify_cli::agent::Response::error(product, operation))
                );
            } else {
                eprintln!("! ERROR\nVerification could not complete\n{e}");
            }
            ExitCode::from(3)
        }
    }
}
struct Options {
    input: String,
    verify: bool,
    behavior: bool,
    protocol: bool,
    blindtest: Option<String>,
    explicit_store: bool,
    store: PathBuf,
    authorization: PathBuf,
    output: String,
    open: bool,
}
fn parse(args: &[String]) -> Result<Options, &'static str> {
    let behavior = args.len() >= 3 && args[0] == "behavior" && args[1] == "verify";
    let verify = args.len() >= 3 && args[0] == "sideeffect" && args[1] == "verify";
    let blindtest = if args.len() >= 3
        && args[0] == "blindtest"
        && ["verify", "validate-suite"].contains(&args[1].as_str())
    {
        Some(args[1].clone())
    } else {
        None
    };
    let input_index = if behavior || verify || blindtest.is_some() {
        2
    } else {
        1
    };
    if args.len() <= input_index
        || (!behavior && !verify && blindtest.is_none() && args[0] != "report")
        || args[input_index].starts_with('-')
    {
        return Err("Expected report and source artifact");
    }
    let mut o = Options {
        input: args[input_index].clone(),
        verify,
        behavior,
        protocol: false,
        blindtest,
        explicit_store: false,
        store: ".b2ige/runs".into(),
        authorization: ".b2ige/authorization.json".into(),
        output: "human".into(),
        open: false,
    };
    let mut i = input_index + 1;
    let mut seen = std::collections::BTreeSet::new();
    while i < args.len() {
        let key = &args[i];
        if !seen.insert(key) {
            return Err("Duplicate option");
        }
        if key == "--open" {
            o.open = true;
            i += 1;
            continue;
        }
        let value = args
            .get(i + 1)
            .filter(|v| !v.starts_with("--"))
            .ok_or("Missing option value")?;
        match key.as_str() {
            "--protocol" if value == "1" => o.protocol = true,
            "--store" => {
                o.store = value.into();
                o.explicit_store = true;
            }
            "--authorization" => o.authorization = value.into(),
            "--output" if ["human", "json", "agent"].contains(&value.as_str()) => {
                o.output = value.clone()
            }
            _ => return Err("Unknown option or output format"),
        };
        i += 2;
    }
    if o.protocol && o.output != "agent" {
        return Err("Protocol v1 requires --output agent");
    }
    if o.output == "agent" && o.open {
        return Err("Agent output cannot open the trusted human viewer");
    }
    if o.blindtest.as_deref() == Some("validate-suite") && (o.open || o.output == "agent") {
        return Err("Suite validation receipts are trusted human artifacts");
    }
    Ok(o)
}
fn run(o: Options) -> io::Result<u8> {
    let (store, id) = if let Some(action) = &o.blindtest {
        let sealed = std::env::var_os("B2IGE_BLINDTEST_SEALED_ROOT").map(PathBuf::from)
            .ok_or_else(|| io::Error::other("Set the trusted controller B2IGE_BLINDTEST_SEALED_ROOT outside the agent workspace"))?;
        let store = EvidenceStore::new(if o.explicit_store {
            o.store.clone()
        } else {
            sealed.join("runs")
        });
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let id = format!("bt-{}-{nonce}", std::process::id());
        // Errors in this branch use a fixed public message. Private file locators,
        // serde excerpts and Docker errors never reach an agent terminal.
        let result = (|| -> io::Result<()> {
            if action == "validate-suite" {
                let c = verify_core::blindtest::read_config(std::path::Path::new(&o.input))?;
                let receipt = verify_core::blindtest::execute_validation(&c, &sealed, &store, &id)?;
                println!("{}", pretty(&receipt));
            } else {
                let c = verify_core::blindtest::read_config(std::path::Path::new(&o.input))?;
                verify_core::blindtest::execute(&c, &sealed, &store, &id)?;
            }
            Ok(())
        })();
        result.map_err(|_| io::Error::other("Blind verifier configuration, sealed evidence, storage, or Docker initialization failed"))?;
        if action == "validate-suite" {
            let receipt = verify_core::blindtest::load_validation(&store, &id)?;
            return Ok(if receipt.matched { 0 } else { 3 });
        }
        (store, id)
    } else if o.behavior {
        let c: verify_core::behavior::BehaviorExperiment =
            verify_cli::integration::read(std::path::Path::new(&o.input))?;
        let a: BehaviorAuthorization = verify_cli::integration::read(&o.authorization)?;
        let store = EvidenceStore::new(&o.store);
        let id = verify_cli::integration::nonce();
        verify_core::behavior::execute(&store, &o.store.join("work"), &id, &c, &a)?;
        (store, id)
    } else if o.verify {
        let contract: verify_core::sideeffect::SideEffectContract =
            serde_json::from_slice(&std::fs::read(&o.input)?)?;
        let store = EvidenceStore::new(&o.store);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let id = format!("se-{}-{nonce}", std::process::id());
        verify_core::sideeffect::execute(&contract, &store, &id)?;
        (store, id)
    } else {
        let (root, id) = resolve(&o.input, &o.store)?;
        (EvidenceStore::new(root), id)
    };
    let (hint, _) = store.load(&id)?;
    let auth: BehaviorAuthorization = if hint.get("sideeffect_result_id").is_some()
        || hint.get("blindtest_result_id").is_some()
    {
        BehaviorAuthorization {
            approved_baselines: Default::default(),
            approved_checker_bindings: Default::default(),
            baseline_stable: false,
        }
    } else {
        serde_json::from_slice(&std::fs::read(o.authorization)?)?
    };
    let report = load(&store, &id, &auth).map_err(|e| {
        if o.output == "agent" {
            io::Error::other("Source or required evidence could not be verified")
        } else {
            e
        }
    })?;
    match o.output.as_str() {
        "json" => println!("{}", pretty(report.document())),
        "agent" if o.protocol => println!(
            "{}",
            serde_json::to_string(&verify_cli::agent::Response::from_verified(
                &report,
                if o.behavior || o.verify || o.blindtest.is_some() {
                    verify_cli::agent::Operation::Verify
                } else {
                    verify_cli::agent::Operation::Report
                }
            ))?
        ),
        "agent" => println!("{}", serde_json::to_string(&report.agent())?),
        _ => print!("{}", report.human()),
    };
    let code = report.document().verdict.exit_code();
    if o.open {
        let viewer = Viewer::bind(store, auth, &id)?;
        let url = viewer.url()?;
        eprintln!("Local report: {url}\nStop viewer with Ctrl-C.");
        let opened = if cfg!(target_os = "macos") {
            Command::new("open").arg(&url).status()
        } else if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", "start", "", &url]).status()
        } else {
            Command::new("xdg-open").arg(&url).status()
        };
        if !opened.is_ok_and(|s| s.success()) {
            eprintln!("Browser did not open; use the local URL above.");
        }
        viewer.serve()?;
    }
    Ok(code)
}
