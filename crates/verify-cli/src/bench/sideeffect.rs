use super::*;
use verify_core::sideeffect::execute;
use verify_core::{
    behavior::{executable_identity, snapshot_identity},
    sideeffect::*,
};
pub fn run(case: &BenchmarkCase, root: &Path, r: &mut BenchmarkCaseResult) -> io::Result<()> {
    let t = Instant::now();
    let mode = case.benchmark_case_id.split('.').next_back().unwrap();
    let executable = std::env::var_os("B2IGE_BENCH_EFFECT_FIXTURE")
        .map(PathBuf::from)
        .unwrap_or(
            std::env::current_exe()?
                .parent()
                .ok_or_else(|| invalid("no binary directory"))?
                .join("p5-effect-fixture"),
        );
    let executable = fs::canonicalize(executable)?;
    let fixture = root.join("fixture");
    fs::create_dir(&fixture)?;
    rusqlite::Connection::open(fixture.join("ledger.db")).map_err(io::Error::other)?.execute_batch("CREATE TABLE ledger(seq INTEGER PRIMARY KEY AUTOINCREMENT,external_id TEXT NOT NULL UNIQUE,idem TEXT NOT NULL,correlation TEXT NOT NULL,operation TEXT NOT NULL);").map_err(io::Error::other)?;
    let schedule = if mode.contains("kill") || mode == "fault" {
        vec!["KILL_AFTER_COMMIT", "RETRY"]
    } else if mode.contains("delivery") {
        vec!["NONE", "DUPLICATE_DELIVERY"]
    } else if mode.contains("retry") {
        vec!["NONE", "RETRY"]
    } else {
        vec!["NONE"]
    };
    let effect = |name: &str| json!({"effect_id":name,"provider":"local_provider","adapter":"sqlite","operation":name,"identity":{"external_identity":"provider_operation_external_id","idempotency_identity":"bench","correlation_identity":"bench"},"expectation":"exactly_once","authoritative_observer":"ledger"});
    let mut effects = vec![effect("payment")];
    let mut relations = vec![];
    if matches!(mode, "ordered" | "atomic") {
        effects.push(effect("email"));
        relations.push(if mode == "ordered" {
            json!({"semantics":"ordered_after","effect":"payment","after":"email"})
        } else {
            json!({"semantics":"atomic_with","effect":"payment","other":"email"})
        });
    }
    let c: SideEffectContract = serde_json::from_value(json!({
        "schema_version":"1","contract_id":"bench_effects_v1","operation":{"operation_id":"checkout","idempotency_identity":"bench","correlation_identity":"bench"},
        "trigger":{"executable":executable,"executable_hash":executable_identity(&executable)?,"args":[],"environment":{"MODE":if mode.starts_with("unsafe"){"unsafe"}else{"safe"},"KEY":"bench","CORRELATION":"bench","HOLD":if mode.contains("kill"){"1"}else{"0"},"OPERATIONS":if matches!(mode,"fault"|"lost"){""}else if mode=="ordered"{"payment,email"}else{"payment"}},"fixture":{"source":fixture,"snapshot_identity":snapshot_identity(Some(&fixture))?},"timeout_ms":5000},
        "effects":effects,"relations":relations,"fault_schedules":[{"schedule_id":"bounded","primitives":schedule}],"exploration_budget":{"max_schedules":1,"max_attempts":8,"reduction_executions":16},
        "required_observers":[{"observer_id":"ledger","db_path":if mode=="observer"{"missing.db"}else{"ledger.db"},"table":"ledger","external_effect_id_column":"external_id","idempotency_column":"idem","correlation_column":"correlation","operation_column":"operation","commit_order_column":"seq","authoritative_source":"durable_append_only_committed_state"}]
    }))?;
    write(&root.join("contract.json"), &c)?;
    r.actual_config_hash = Some(canonical_hash(&c)?);
    r.actual_fixture_hash = Some(c.trigger.executable_hash.clone());
    let store = EvidenceStore::new(root.join("runs"));
    r.setup_ms = ms(t);
    let t = Instant::now();
    let raw = execute(&c, &store, "result")?;
    let loaded = load(&store, "result")?;
    validate(&result_schema(), &loaded)?;
    r.actual_verdict = Some(loaded.verdict);
    r.evidence_valid = Some(true);
    r.verified_reload = Some(raw == loaded);
    r.result_refs.push(reference(&store, "result"));
    r.observation_coverage = Rate::new(
        loaded.schedules.iter().filter(|s| s.complete).count(),
        c.fault_schedules.len(),
    );
    let actions: usize = loaded
        .schedules
        .iter()
        .map(|s| s.schedule.primitives.len())
        .sum();
    let executed: usize = loaded
        .schedules
        .iter()
        .map(|s| {
            if s.complete {
                s.schedule.primitives.len()
            } else {
                0
            }
        })
        .sum();
    r.metrics.insert(
        "requested_fault_execution_coverage".into(),
        Rate::new(executed, actions),
    );
    // Counts are from verified SQLite snapshots/committed identities, never attempt counts.
    let confirmed = loaded
        .schedules
        .iter()
        .flat_map(|s| s.committed.iter())
        .count();
    r.metrics.insert(
        "committed_effect_confirmation_coverage".into(),
        Rate::new(
            usize::from(
                mode != "observer" && loaded.schedules.iter().all(|s| !s.initial.is_empty()),
            ),
            1,
        ),
    );
    if case.category == "duplicate" {
        let proven = loaded.violations.iter().any(|v| {
            v.kind == ViolationKind::DuplicateCommittedEffect && !v.evidence_refs.is_empty()
        }) && confirmed > 1;
        r.metrics.insert(
            "duplicate_detection".into(),
            Rate::new(usize::from(proven), 1),
        );
        if !proven {
            return Err(invalid(
                "duplicate benchmark requires committed effect proof",
            ));
        }
    }
    if matches!(mode, "ordered" | "atomic" | "lost") {
        let kind = match mode {
            "ordered" => ViolationKind::CommitOrder,
            "atomic" => ViolationKind::HalfCommit,
            _ => ViolationKind::MissingCommittedEffect,
        };
        let proven = loaded
            .violations
            .iter()
            .any(|v| v.kind == kind && !v.evidence_refs.is_empty());
        r.metrics.insert(
            "relationship_or_lost_detection".into(),
            Rate::new(usize::from(proven), 1),
        );
        if !proven {
            return Err(invalid("required relationship violation not proven"));
        }
    }
    if let Some(x) = &loaded.counterexample {
        let verified = if let Some(a) = &x.reproduction {
            let repro = load(&store, &a.artifact_id)?;
            Some(
                repro.verdict == Verdict::Fail
                    && repro.violations.iter().any(|v| {
                        v.kind == x.signature.kind && v.effect_ids == x.signature.effect_ids
                    }),
            )
        } else {
            None
        };
        r.reproduction=Reproduction{semantics:"Fresh SQLite reset and retained fault schedule; committed violation signature. Automatic replay CLI unavailable".into(),exists:x.reproduction.is_some(),verified,reduction_available:true,locally_minimized:Some(x.status==ReductionStatus::LocallyMinimized),original_size:Some(x.original_schedule.primitives.len()),reduced_size:Some(x.retained_schedule.primitives.len()),refs:x.reproduction.iter().map(|a|reference(&store,&a.artifact_id)).collect(),..Default::default()};
        r.metrics.insert(
            "reduction_success".into(),
            Rate::new(
                usize::from(x.status == ReductionStatus::LocallyMinimized),
                1,
            ),
        );
    }
    if mode == "corrupt" {
        corrupt_evidence(&store, "result")?;
        let (observed, rejected) = loader_observation(load(&store, "result").map(|v| v.verdict));
        r.negative_evidence_rejected =
            Some(rejected && crate::load(&store, "result", &empty_auth()).is_err());
        r.actual_verdict = Some(observed);
        r.product_outcome = Some("VERIFIED_LOADER_AFTER_EVIDENCE_REMOVAL".into());
        r.limitations.push("ERROR is the verified-load boundary error; the intact pre-corruption run was validated separately".into());
    } else if crate::load(&store, "result", &empty_auth())?
        .document()
        .verdict
        != loaded.verdict
    {
        return Err(invalid("verified report mismatch"));
    }
    r.execution_ms = ms(t);
    Ok(())
}
