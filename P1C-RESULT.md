# P1C 결과

## 구현 내용

- 저장된 P1B run의 metadata·hash·evidence 연결을 실행 전에 검증한다.
- executable 바이트의 SHA-256을 immutable target identity와 대조하고,
  기록된 plan/seed/executable/args/cwd/env/timeout을 기존 acquisition/runner로 재실행한다.
- 새 run/evidence와 원 run ID·result/evidence hash 관계를 하나의 Store commit에 기록한다.
- 새 ReplayResult envelope v1의 REPRODUCED / EXECUTED_BUT_DIVERGED /
  UNAVAILABLE / ERROR를 제품 Verdict와 분리했다. 기존 schema와 PASS/FAIL 의미는 유지했다.

## Replay integration 결과

**실제 process 테스트 15/15 PASS. 요청 A–H 충족.**
동일 입력 재실행, 실행 횟수 증가·새 evidence, 서로 다른 run ID와 관계 저장,
mutable/missing target, 필수 metadata 누락, plan/config/seed/hash/version 불일치,
원 artifact 손상, executable 변경·누락, spawn failure, 중복 ID 거부를 확인했다.
stdout 변화는 EXECUTED_BUT_DIVERGED이며 제품 Verdict는 INCONCLUSIVE다.
non-zero exit도 동일하게 재현되면 REPRODUCED이며 제품 FAIL을 만들지 않는다.

## 전체 테스트 수

**110/110 PASS = 기존 P1A 77 + P1B 18 + P1C 15. 실패·ignored 0.**
Darwin arm64 / Rust 1.98.1에서 `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`cargo build --workspace --release` 모두 PASS. 기존 테스트·JSON fixture·schema 변경 없음.

## 변경 파일

- `Cargo.lock`, `crates/verify-core/Cargo.toml`
- `crates/verify-core/src/acquisition.rs`, `crates/verify-core/src/acquisition/replay.rs`
- `crates/verify-replay/src/lib.rs`
- `crates/verify-core/tests/replay_execution.rs`, `tests/fixtures/replay.rs`
- `tasks/P1C-INDEX.md`, `P1C-RESULT.md`, `state/CURRENT.md`

## 남은 제한

신뢰된 local Unix/P1B artifact와 executable SHA-256 identity만 지원한다.
Git identity는 executable과의 연결을 입증할 수 없어 UNAVAILABLE다.
Store hash는 출처 인증이 아니며 원 실행 당시의 executable 바이트를 소급 인증하지 않는다.
새 replay evidence는 반드시 공통 acquisition 경로에서 실제 실행으로 생성한다.
동일 사용자에 의한 hash 확인·spawn 사이 파일 교체 방어, 환경 snapshot/reset,
동적 라이브러리 고정, replay-of-replay는 제공하지 않는다. 출력 동일성·제품 실패 재현을
보장하지 않는다. 기존 stream 한도와 process-group cleanup 제한은 유지한다.

## P1 전체 완료 가능 여부

**요청된 P1A/P1B/P1C 구현 범위의 로컬 완료 조건 충족.**
독립 재리뷰나 별도의 P1 최종 승인을 수행했다고 주장하지 않는다.
P2는 시작하지 않았으며 commit/push/deploy도 하지 않았다.
