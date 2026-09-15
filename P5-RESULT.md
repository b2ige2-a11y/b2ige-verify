# P5 결과 — SideEffect Proof MVP

**P5 COMPLETE / LOCAL GATE PASS. P6 시작 가능, 미시작. Commit/push/deploy 미수행.**

## 구현

- P5 전체를 하나의 단계로 완료: SideEffectContract / History / ExperimentResult /
  FaultScheduleResult v1, attempt/effect identity 분리, fault scheduler, SQLite observer,
  checker, 실제 재실행 reducer, P4 Human/Agent/CLI/UI 통합.
- Semantics: `exactly_once`, `at_most_once`, `at_least_once`, `never`, `ordered_after`,
  `atomic_with`. Unsupported semantics/config은 ERROR.
- Fault: `NONE`, `RETRY`, `DUPLICATE_DELIVERY`, `KILL_AFTER_COMMIT`.
  Kill은 첫 action + 바로 다음 retry로 제한. 커밋 snapshot, signal 전달, SIGKILL 종료,
  retry 실행을 모두 검증한다. Timeout 종료는 scheduled fault 성공으로 세지 않는다.
- SQLite committed ledger를 online backup하여 WAL commit도 보존. 실제 DB bytes를 재조회하고
  schema/identity/order/append-only 상태를 확인한다. 같은 external ID 재관측은 한 commit.
- P1 process runner와 EvidenceStore, P2 fixture snapshot 복사 재사용. P3 reducer 의미 불변.
  새로운 controlled runtime 타입에 signal 전달 여부를 추가; 기존 process wire fields 불변.
- FAIL signature를 실제 재실행하며 schedule action 삭제/kill 제거. 모든 유효한 단일 삭제를
  확인한 때만 `LOCALLY_MINIMIZED`; budget 소진/미재현은 별도 상태와 원본 유지.
- `b2ige sideeffect verify <contract>`, `report <id>`, `--output agent|json`, `--open`.
  Exit code 0/1/2/3 유지. SideEffect는 Behavior authorization 불필요. Replay CLI 미제공.
- ReportDocument/AgentReport/projection v2. 이전 v1 schemas 보존. Agent는 expected/observed,
  effect IDs/counts, reproduction 및 evidence refs allowlist만 사용하며 DB/env/raw history 제외.

## 실제 corpus

| Case | Attempts | Committed | 판정 |
|---|---:|---:|---|
| S1 SAFE normal | 1 | 1 | PASS |
| S2 SAFE retry | 2 | 1 | PASS |
| S3 SAFE commit → kill → retry | 2 | 1 | PASS |
| U1 UNSAFE retry | 2 | 2 | FAIL |
| U2 UNSAFE duplicate delivery | 2 | 2 | FAIL |
| U3 UNSAFE commit → kill → retry | 2 | 2 | FAIL |
| N1 observer unavailable | 2 | unknown | INCONCLUSIVE |
| N2 requested kill 미실행 | 2 | 0 | INCONCLUSIVE |
| N3 corrupted SQLite | 1 | unknown | ERROR |

UNSAFE 3개는 실제 재실행으로 violation을 보존하고 local reduction을 완료했다.
Known unsafe false PASS **0**, missing committed evidence PASS **0**,
unexecuted fault PASS **0**, SAFE false FAIL **0**.

실제 Rust executable + SQLite corpus:
[summary](.b2ige/p5-final-corpus/summary.json), [inputs/evidence](.b2ige/p5-final-corpus).
최종 release CLI로 S3/U3/N1/N2/N3를 별도 실행하여 0/1/2/2/3 확인:
[CLI summary](.b2ige/p5-cli-summary.json).

## 검증 gate

- 신규 **44** (core 38 + CLI/report 6), 기존 **228** 보존 → **272/272 PASS**.
  Failed/ignored 0. 마지막 UI 정리 후 CLI/report **28/28** 재검증 PASS.
- `cargo fmt --check` PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` PASS.
- `RUST_TEST_THREADS=4 cargo test --workspace` PASS.
- `cargo build --workspace --release` PASS.
- Rust compiler/clippy/release build가 typecheck/lint/production build 역할을 수행한다.
- 실제 release `--open` + agent-browser: 1280px desktop / 390px mobile,
  evidence/timeline/raw 펼치기, verified child 이동·복귀, PASS와 INCONCLUSIVE 표시 확인.
  Console error 0, horizontal overflow 0, expanded desktop/mobile axe violation 0.
  Read-only viewer로 입력 form 없음.
- [Desktop](.b2ige/p5-desktop.png), [Mobile](.b2ige/p5-mobile.png),
  [Inconclusive](.b2ige/p5-inconclusive-mobile.png).

## False-PASS 방어 / verifier 검토

- Output verdict/violation cache를 신뢰하지 않고 contract→schedule→child runtime→SQLite→history
  →checker를 재검증한다. Parent/child/evidence/history의 hash와 run binding을 검사한다.
- 해시와 references까지 다시 맞춘 row/DB/history/retry/kill/minimality 변조 테스트도 거부.
  Output verdict만 변경·재해시하면 원래 evidence의 판정을 재계산한다.
- Observer failure는 ERROR/INCONCLUSIVE이며 request 횟수·target exit로 제품 FAIL을 추정하지 않는다.
- Nondeterminism: attempt 간 byte 비교나 wall-clock ordering을 쓰지 않는다. 실제 외부 ID와
  ledger sequence만 사용하며 reduction signature는 임의 생성 external ID 값과 무관하다.
- Baseline poisoning: 원본 fixture/executable hash 확인, schedule마다 fresh cwd, 초기 matching
  commit 존재 시 ERROR. 자동 baseline 갱신 없음.
- Secret 접근: trusted local Unix/isolation none. 숨겨진 grader/secrecy를 주장하지 않는다.
- 모든 FAIL은 실제 rerun artifact와 연결되거나 replayability unavailable 이유를 가진다.

## 남은 제한

SQLite append-only ledger / local Unix / 순차 duplicate delivery / bounded exploration에 한정.
Atomic은 완료 시점의 양쪽 effect 존재 여부이며 transaction atomicity의 전면 증명이 아니다.
Ordering은 같은 ledger의 유일 증가 integer sequence 필요. Kill은 첫 action에서만 지원한다.
Detached/hostile writers, wholesale artifact rewriting 인증, 전역 최소성은 제공하지 않는다.
DB snapshot 8 MiB 및 명시적 실행 budget 적용. Mandatory paid API **NONE**.
Stripe/PostgreSQL/cloud/LLM/MCP/P6 BlindTest 확장 없음.

계약과 사용법: [SIDEEFFECT.md](docs/SIDEEFFECT.md) · [P5-INDEX.md](tasks/P5-INDEX.md).
