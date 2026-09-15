# P3C 결과 — Counterexample Reducer

- **Reducer algorithm:** canonical argument index→environment 이름 순서로 단일 assignment
  삭제를 시도하고 채택 시 처음부터 재검사한다. 매 시도는 기존 P2 exact engine으로 새
  BEFORE/AFTER를 실행한다. Algorithm `behavior.assignment-deletion.v1`, 신규
  CounterexampleReductionResult/schema **v1**. 기존 P2/P3A/P3B semantics 변경 없음.
- **Failure signature semantics:** observable 종류 전체, baseline/BEFORE/AFTER identity,
  process comparison policy가 같아야 한다. Bytes/status 값은 달라도 되지만 stdout
  divergence가 exit divergence로 바뀌면 거절한다. 실제 DIVERGENCE_PROVEN만 채택한다.
- **최소성 의미:** 완료된 마지막 sweep의 모든 단일 삭제를 검증한 1-minimal만
  LOCALLY MINIMIZED COUNTEREXAMPLE로 기록한다. 축소 있음=MINIMIZED, 없음=UNREDUCIBLE.
  Global minimum을 주장하지 않으며 ERROR/INCONCLUSIVE가 남으면 minimal=false다.
- **신규/전체 테스트:** 신규 **22개**, 기존 184개 보존, **206/206 PASS**, failed/ignored 0.
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `RUST_TEST_THREADS=4 cargo test --workspace`, `cargo build --workspace --release` 모두 PASS.
- **Budget semantics:** candidate P2 pair 호출 횟수 제한(호출당 프로세스 최대 2회 시작),
  ERROR/INCONCLUSIVE도 차감, 0 허용. 미검사 삭제가 남으면 BUDGET_EXHAUSTED와
  minimal=false를 기록하며 best proven result를 보존한다. 마지막 호출로 검사가 모두
  끝나면 정상 완료 가능. Source/load 검증은 재실행 budget을 쓰지 않는다.
- **False-proof 방어:** source suite/spec/case/child/run/evidence 검증 후 시작한다.
  채택마다 새 실제 evidence를 사용한다. 실행 후 전체 trace를 재검증하고 atomic 및
  non-overwrite 저장한다. Load는 candidate 순서·experiment·결과·signature·채택 여부·
  budget·최소성·최종 refs를 재구성한다. 재해시한 assignment/minimal/status/signature/
  trace/refs 변조와 증거 손상은 거부한다. Baseline/target/approval/policy를 변경하지 않는다.
- **남은 제한:** 명시 input assignment 삭제만 지원. 결과 결정론성은 안정된 실행 환경을
  전제로 한다. Signature는 같은 observable 종류의 다른 근본 원인을 구별하지 않는다.
  Trusted local Unix, P2 host/stream/fixture 제한, hash 비인증 경계, paired replay unavailable
  유지. 저장/무결성 오류는 Err(ERROR)이며 완료 artifact를 보장하지 않는다.
- **P3 전체 완료 가능 여부:** **P3C LOCAL GATE PASS**. 기록된 P3A/P3B 선행 gate와 함께
  P3A–P3C 범위의 **P3 LOCAL COMPLETE**로 기록 가능하다. **P4 미시작**.
  Commit/push/deploy 미수행.
