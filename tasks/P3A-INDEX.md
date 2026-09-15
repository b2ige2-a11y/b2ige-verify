# P3A — Baseline Stability / Noise Profiler

## Stability semantics

- `behavior::stability::profile`: 명시한 BEFORE를 실제 P1 acquisition으로 반복한다.
  기본 5회, 허용 1–1000회, STABLE 인정 최소 2회다. 요청 횟수 전체가 필요하다.
- 고유 run ID와 최초 fixture snapshot의 fresh workspace를 매번 사용한다.
  args/env/timeout/fixture/seed가 같고 executable SHA-256을 매 실행 전에 재검증한다.
  Candidate는 profile의 baseline-only experiment에 포함하지 않는다.
- exit/signal/stdout/stderr raw bytes 각각을 분류한다. 완전한 모든 반복값이
  같으면 STABLE, 다르면 UNSTABLE이다. 2회 미만 또는 한 번이라도 timeout,
  capture incomplete, runner failure가 있으면 보수적으로 네 항목 모두 INCOMPLETE다.
  실제 bytes로 판정하고 artifact에는 관측 값/hash 집합을 기록한다.
- `compare`: 저장 profile ID/hash로 검증 후 fresh candidate 1회를 실행한다.
  Profile 불완전 → INCONCLUSIVE; candidate 불완전 → INCONCLUSIVE;
  candidate runner failure → ERROR; stable observable 차이 → DIVERGENCE_PROVEN.
  확실한 stable 차이가 없고 UNSTABLE이 하나라도 있으면 INCONCLUSIVE다.
  네 항목 모두 STABLE이며 candidate와 같을 때만 NO_DIVERGENCE_FOUND/PASS다.
- Profile 없는 호출은 기존 `behavior::execute/load` 그대로다. 기존 P1/P2 schema와
  exact semantics를 유지한다. 신규 `BaselineStabilityProfile`과
  `StabilityComparisonResult`는 각각 **schema v1**이다. 별도 JSON Schema는
  `stability_schema` example의 `profile`/`comparison` 인자로 재생성할 수 있다.
- 기존 approval/artifact/checker pin을 검증하며 baseline을 자동 승인·갱신하지
  않는다. P3A의 P1 acquisition에는 외부 안정성 주장 대신 `stable: false`를 기록한다.

## 테스트 및 local gate

2026-09-15, Darwin arm64 / Rust 1.98.1: **LOCAL GATE PASS**.

| 검증 | 결과 |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, 경고 0 |
| `cargo test --workspace` | 162/162 PASS, failed/ignored 0 |
| `cargo build --workspace --release` | PASS |

기존 138개 보존, **P3A 24개 추가**. 실제 executable로 A–J, stderr/exit/signal
불안정, fresh snapshot, 실행 파일 재검증, schema/receipt, 재해시·증거 손상을 검증했다.

## False-PASS 방어 / verifier 자체 검토

- 증거 누락: `load_profile/load_comparison`이 모든 저장 acquisition을 P1으로
  재구성하고 분류·결과를 재계산한다. 임의 관측값/STABLE 객체를 받는 공개 비교
  경로가 없다. 잘못된 profile/run/receipt는 Err(ERROR)이며 exact fallback은 없다.
- 관측 실패: timeout/불완전 capture는 PASS나 FAIL로 바꾸지 않는다.
  I/O·계약·저장 실패는 Err(ERROR, 완성 artifact 보장 없음)로 끝난다.
- 비결정성: UNSTABLE을 ignore/mask/normalization으로 바꾸지 않는다.
  불안정 집합 중 한 값과 candidate가 같아도 PASS가 될 수 없다.
- Baseline poisoning: 승인·target·case·input·seed·설정·profile hash를 검증한다.
  요청 횟수는 각 acquisition plan에도 결합하여 outlier를 버리고 횟수를 낮춘
  재해시 profile을 거부한다. Baseline 자동 승인/갱신은 없다.
- 비밀 경계: trusted local Unix를 유지하며 격리나 secrecy는 주장하지 않는다.
- 재현 정보: 입력·experiment/config hash·run/evidence 연결과 차이를 기록한다.
  모든 FAIL에는 `replayability: unavailable` 및 재실행 방법·제한을 명시한다.

## 남은 제한

STABLE은 bounded observation이며 미래 결정성 보장이 아니다. Stale 여부는
identity/config/hash로 판별하고 시간 기반 만료 정책은 없다. Host/runtime/library/
외부 상태·절대 cwd/inode/ctime은 통제하지 않는다. 기존 snapshot 및 stream당 1 MiB
한도를 유지한다. 동일 사용자의 악의적 전체 evidence 위조나 hash-to-spawn 교체를
방어하는 인증/격리는 없고 hash는 인증이 아니다. 자동 paired replay도 없다.
이 검증은 구현 자체 검토이며 독립 감사나 exhaustive proof가 아니다.

## P3B 시작 가능 여부

P3A local 선행 gate는 충족했다. **P3B/P3C 미시작, 별도 명시적 요청 필요.**
Generator/fuzzing/mutation/reducer 및 commit/push/deploy는 수행하지 않았다.

결과: [P3A-RESULT.md](../P3A-RESULT.md), gate: [CURRENT.md](../state/CURRENT.md).
