# P3C — Counterexample Reducer

- **Reducer algorithm:** `behavior::reduction::{execute, load}`. Argument index 순서 후
  environment 이름 순서로 assignment를 정렬하고 하나씩 삭제한다. 채택하면 처음부터
  재검사한다. 각 시도는 기존 P2 exact BEFORE/AFTER를 새 comparison/run/workspace에서
  실행한다. Base로부터 input만 복원하며 baseline/target/approval/policy는 바꾸지 않는다.
  Algorithm `behavior.assignment-deletion.v1`; 신규 CounterexampleReductionResult **v1**
  및 JSON Schema/example 추가. 기존 P2/P3A/P3B schema와 semantics 변경 없음.
- **Failure signature semantics:** 실제 DIVERGENCE_PROVEN의 ordered observable 종류
  전체가 같고 baseline identity·BEFORE/AFTER target identity·ProcessByteExactV1이 같아야
  채택한다. stdout/stderr bytes 및 exit/signal 값 자체는 동일할 필요가 없다.
  stdout 실패가 exit 실패로 바뀌거나 observable 종류가 추가/제거되면 채택하지 않는다.
- **최소성 의미:** 완료된 마지막 sweep에서 모든 단일 삭제가 같은 divergence를
  보존하지 않음이 검증된 경우에만 `minimal=true`, LOCALLY MINIMIZED COUNTEREXAMPLE.
  축소가 있으면 MINIMIZED, 없으면 UNREDUCIBLE. 빈 assignment는 삭제 대상이 없어 완료.
  Global minimum이나 모든 실행 환경에서의 증명이 아니다. 불확실한 마지막 sweep은
  INCONCLUSIVE/ERROR이며 최소성을 주장하지 않는다. Reducer status는 제품 Verdict가 아니다.
- **신규/전체 테스트:** P3C 22개 추가, 기존 184개 보존, **206/206 PASS**.
  fmt/clippy/전체 tests(RUST_TEST_THREADS=4)/release build 모두 PASS.
  실제 프로세스로 단일/조합 원인, 삭제 거절, timeout/spawn ERROR, 비FAIL/손상 source,
  budget, 결정론적 순서, signature 변경, fresh evidence, 재해시 변조, 기존 contract,
  삭제 재시작, 빈 assignment, atomic non-overwrite, schema를 검증한다.
- **Budget semantics:** budget 단위는 candidate P2 pair 호출 1회(프로세스 최대 2회 시작).
  ERROR/INCONCLUSIVE도 차감하며 source/load 검증은 실행하지 않는다. 0 허용.
  미검사 삭제가 남으면 BUDGET_EXHAUSTED, minimal=false, best proven result 보존.
  마지막 허용 호출에서 전체 검사를 완료하면 정상 완료 가능. 저장/무결성 오류는
  Err(ERROR)이며 완료 artifact를 보장하지 않는다.
- **False-proof 방어:** source suite hash/spec/generated assignment/child/signature 및
  실제 run/evidence를 실행 전에 검증한다. 원본 증거를 축소 입력에 복사하지 않는다.
  새 evidence를 가진 같은 signature의 DIVERGENCE_PROVEN만 채택한다. 실행 후 source와
  전체 trace를 다시 검증하고 atomic/non-overwrite 저장한다. Load는 동일 state machine으로
  candidate 순서·input·decision·acceptance·budget·최소성·최종 refs·hash/receipt를 재구성한다.
  양쪽 acquisition이 있는 negative 결과도 재계산한다. 증거 누락/observer 오류를 성공이나
  product failure로 승격하지 않는다. Baseline 자동 갱신과 secret isolation 주장은 없다.
- **남은 제한:** 명시 argument/environment assignment 삭제만 지원한다. 결정론적 결과는
  target/실행 환경 안정성을 전제로 하며 nondeterminism을 해결하지 않는다. 같은 observable
  종류의 다른 원인까지 구별하는 root-cause signature는 아니다. Trusted local Unix와 P2의
  host/stream/fixture 제한, hash 비인증 경계 및 paired replay unavailable은 유지된다.
  최종 입력은 기록된 experiment와 approved targets/fixture로 새 ID를 사용해 재실행 가능하나
  자동 paired replay는 없다. 축소를 채택하지 못한 경우 original refs를 명시적으로 유지한다.
- **P3 전체 완료 가능 여부:** **P3C LOCAL GATE PASS**, 기록된 P3A/P3B 선행 gate와
  함께 P3A–P3C 범위 **P3 LOCAL COMPLETE**. P4 미시작. Commit/push/deploy 미수행.
