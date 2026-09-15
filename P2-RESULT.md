# P2 결과 — Behavior Differential MVP

## 구현한 Behavior semantics

- 명시한 case 하나를 기존 P1 acquisition으로 BEFORE/AFTER 각각 실제 실행한다.
  서로 다른 run ID와 직접 관측 evidence를 저장한다.
- 두 executable의 실제 SHA-256, P1A baseline 승인·출처·artifact pin·checker
  binding, observation contract와 동일 입력 hash를 실행 전에 검증한다.
- 같은 args·explicit env·timeout·fixture·비교 정책을 사용한다. 최초 fixture
  snapshot에서 별도 fresh workspace를 생성하여 BEFORE 변경이 AFTER에 전파되지 않는다.
- **exit code, terminating signal, stdout raw bytes, stderr raw bytes**를 정확히 비교한다.
  동일한 non-zero exit도 차이가 없으면 NO_DIVERGENCE_FOUND다.
- DIVERGENCE_PROVEN→FAIL, NO_DIVERGENCE_FOUND→PASS,
  INCONCLUSIVE→INCONCLUSIVE, ERROR→ERROR로 매핑한다.
  PASS 의미는 **NO DIVERGENCE FOUND within tested behavior space**다.
- timeout·불완전 capture는 INCONCLUSIVE, spawn/acquisition/store 실패·손상·
  identity 불일치는 ERROR다. 미지원 필수 observer는 실행하지 않고 INCONCLUSIVE다.
- versioned `BehaviorComparisonResult` v1에 입력·대상·baseline·실험·설정 hash,
  양쪽 run/evidence 연결과 실제 차이를 기록하고 기존 Store로 원자적으로 저장한다.
  기존 artifact는 덮어쓰지 않는다. `behavior::load`는 연결된 저장 evidence를
  다시 검증하고 양성 결과를 재계산한다.

## Integration test 결과

**실제 executable 통합 테스트 28/28 PASS. 요청 A–L 모두 충족.**

| 범위 | 결과 |
| --- | --- |
| A, E: 같은 출력·exit / 같은 non-zero exit | NO_DIVERGENCE_FOUND / PASS |
| B, C, D, F: stdout / stderr / exit / signal 차이 | DIVERGENCE_PROVEN / FAIL |
| G: 양쪽 또는 한쪽 timeout, capture overflow | INCONCLUSIVE; PASS/FAIL 금지 |
| H: spawn / 한쪽 acquisition 실패 | ERROR |
| I, J: 입력 hash / baseline 승인·target·binding 불일치 | 실행 전 거부, ERROR |
| K: BEFORE cwd·파일·원본 fixture 변경 | AFTER는 최초 snapshot의 fresh workspace에서 실행 |
| L: run ID·result/evidence hash·schema·atomic commit 연결 | 실제 저장 artifact 대조 PASS |

추가로 missing/corrupt/pending evidence, mutable/Git/missing/변경된 executable,
미지원 observer, 중복 ID·기존 workspace, raw bytes·literal args·explicit env를 확인했다.
전체 hash를 일관되게 다시 계산해도 필수 terminal observation 누락, timeout,
spawn 실패, 한쪽 run 누락을 PASS로 바꾸거나 실제 차이 없는 FAIL을 만들 수 없는지 검증했다.

## 전체 검증

Darwin arm64 / Rust 1.98.1, 2026-09-14.

| 명령 | 결과 |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, warning 0 |
| `cargo test --workspace` | **138/138 PASS**, failed/ignored 0 |
| `cargo build --workspace --release` | PASS |

**기존 P1 110개 + P2 28개 = 138개.** 기존 P1 테스트·fixture·schema를 삭제하거나
변경하지 않았다. P1 코드는 두 내부 검증 함수의 `pub(crate)` 공개 범위만 조정했다.
P1 capture overflow의 기존 ERROR evidence/verdict도 그대로 유지하며, P2 outcome에서만
불완전 capture로 분류한다. 새 dependency 및 Cargo.lock 변경 없음.

## 변경 범위

- `crates/verify-core/src/behavior.rs`, `crates/verify-core/src/behavior/workspace.rs`:
  P2 엔진·reset·검증·저장.
- `crates/verify-core/src/lib.rs`, `crates/verify-core/src/acquisition.rs`:
  모듈 export·기존 내부 검증 재사용.
- `crates/verify-core/Cargo.toml`, `crates/verify-core/tests/behavior_differential.rs`,
  `tests/fixtures/behavior*.rs`: 별도 컴파일한 실제 BEFORE/AFTER fixture와 통합 테스트.
- `crates/verify-core/examples/behavior_schema.rs`,
  `schemas/behavior-comparison-result.schema.json`: 신규 artifact schema v1.
- `tasks/P2-INDEX.md`, `P2-RESULT.md`, `state/CURRENT.md`: 계약·검증·phase gate 기록.

## 남은 제한

신뢰된 local Unix process만 지원한다. stdin은 null, stream 한도는 P1과 동일한 각 1 MiB다.
fixture는 64 MiB / 4096 entries / depth 64 이내이며 symlink·특수 파일을 거부한다.
파일 내용·경로·permission·directory를 복원하고 access/modified time을 Unix epoch로 맞춘다.
절대 cwd·inode·ctime·소유자·확장 속성·라이브러리·외부 상태까지 동일하게 보장하지 않는다.
같은 사용자에 의한 hash 확인과 spawn 사이의 악의적 교체나 sandbox/secrecy는 지원하지 않는다.

Network/DB/외부 서비스 reset·관측, filesystem behavior 비교, noise normalization·learner,
generation·mutation·fuzzing·reduction·stateful exploration은 구현하지 않았다.
baseline stability는 기존 P1A 방식의 외부 정책 선언이며 P2가 측정한 안정성이 아니다.
단 한 번씩의 명시적 실행에서 관측한 차이를 증명하며 동등성·일반 회귀 원인을 증명하지 않는다.

paired snapshot replay executor는 미구현이다. 모든 FAIL에
`replayability: unavailable`과 이유·새 비교를 위한 기록 입력 재사용 안내를 남긴다.
P1C replay만으로는 workspace reset이 수행되지 않는다.

## P3 시작 가능 여부

**P2 로컬 완료 gate PASS. P3 착수의 로컬 선행 조건은 충족했다.**
구현 자체 검토와 제한된 테스트 결과이며 독립 감사나 exhaustive proof를 의미하지 않는다.
**P3는 시작하지 않았다. 다음 명시적 요청이 필요하다.**
UI/MCP/LLM/cloud 작업 및 commit/push/deploy도 수행하지 않았다.
