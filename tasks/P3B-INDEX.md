# P3B — Deterministic Case Generator

- **Generator semantics:** `behavior::generation::{generate, execute, load}`를 추가했다.
  GenerationSpec **v1**은 generation ID, 승인된 base experiment, mode, ordered
  dimensions/candidates, max_cases를 기록한다. Argument index와 environment만
  변경하며 target executable/hash, approval, policy, timeout, fixture contract,
  observers, seed는 유지한다. 변경 입력에 맞춰 BEFORE/AFTER input hash만 재결합한다.
  Generated ID는 spec hash·parent identity·assignment·input hash에 결합한다.
  Assignment에는 dimension/candidate index, target, value를 보존한다.
  각 case를 기존 P2 exact `execute`로 BEFORE/AFTER 실제 acquisition한다.
  신규 suite/spec JSON Schema v1과 재생성 example을 추가했고 P2/P3A는 변경하지 않았다.
- **Generation modes:** ONE_AT_A_TIME은 dimension 순서→candidate 순서로 base에서
  하나씩 바꾼다. CARTESIAN은 마지막 dimension이 가장 빠르게 변한다.
  Base는 자동 포함하지 않는다. 명시한 base와 같은 candidate는 허용하지만,
  동일한 결과 입력이 중복 생성되면 전체를 거부한다. 빈 generation/dimension,
  중복 target/candidate, 잘못된 index/env/NUL, max_cases 초과·산술 overflow는
  실행 전에 거부한다. Truncation이나 자동 deduplication은 없다.
- **신규/전체 테스트:** P3B **22개 추가**, 기존 162개 보존, 전체 184개.
  A–P 및 우선순위, 이전 evidence 손상, 재해시한 child PASS, atomic receipt/schema를
  실제 executable로 검증했다. fmt/clippy/release build 및 전체 184/184 tests PASS.
  Test gate는 RUST_TEST_THREADS=4에서 통과했다. 기본 병렬도 최초 timeout 경합 실패와
  재검증 조건은 P3B-RESULT.md에 기록했다.
- **Aggregate verdict 규칙:** generation 무결성 실패는 Err(ERROR, 완성 suite 보장 없음).
  검증된 children에서는 FAIL > ERROR > INCONCLUSIVE > PASS 순이다. Expected case
  전체가 존재하고 완전한 두 acquisition으로 모두 NO_DIVERGENCE_FOUND일 때만 PASS다.
  FAIL+INCONCLUSIVE/ERROR는 FAIL을 유지하며 coverage.complete=false를 기록한다.
- **False-PASS 방어:** 기존 EvidenceStore의 reserve/derived receipt/atomic complete를
  사용한다. 마지막 실행 후 모든 child를 다시 읽는다. Load는 spec에서 ordered cases와
  child IDs를 재생성하고 P2 loader로 run/evidence를 검증한다. 두 run이 있으면 negative
  outcome도 P3B 내부에서 재계산한다. Child hash·전체 experiment·input·assignment·spec,
  counts·coverage·verdict·receipt를 대조한다. 일부 누락·손상·변조나 재해시한 허위 집계는
  Err(ERROR)이며 verified PASS가 아니다. Baseline 자동 승인·갱신은 없다.
- **남은 제한:** 명시 domain의 bounded enumeration이며 동등성/exhaustive proof가 아니다.
  P2 external stability assertion을 사용하고 P3A를 자동 적용하지 않는다. Trusted local
  Unix·snapshot/stream/host/runtime 제한과 hash의 비인증 경계를 유지한다. FAIL의 실제
  divergence와 run/evidence/replayability 사유는 child artifact에 보존한다. Paired replay,
  fuzzing, mutation, inference, LLM generation, reducer는 없다. Cross-platform 실행 검증은
  하지 않았으며 동일 직렬화 spec의 순서/ID/hash는 입력 데이터만으로 계산한다.
- **P3C 시작 가능 여부:** **P3B LOCAL GATE PASS**, 선행 조건 충족.
  P3C는 별도 명시적 요청이 필요하며 이번 작업에서는 시작하지 않는다.
  Commit/push/deploy도 수행하지 않는다.
