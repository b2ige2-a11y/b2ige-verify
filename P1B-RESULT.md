# P1B 결과

## 구현한 것

- 실제 OS process Runner: executable/args, cwd, 명시적 환경, timeout,
  stdout/stderr 원본 bytes, exit code/signal, 시작·종료·실패 상태.
- 기존 ExperimentPlan에서 실행 설정을 받아 실제 관측을 EvidenceBundle과
  RunContext에 연결하고 canonical result artifact를 기록한다.
- local filesystem Evidence Store: run 예약, evidence identity/source/claim 연결,
  JCS SHA-256, fsync + atomic no-replace write, result-last commit 및 무결성 검증.
- target non-zero exit는 정상 관측이다. spawn/capture 실패는 runner ERROR,
  timeout은 partial coverage다. checker 미실행으로 정상 수집도 INCONCLUSIVE이며
  PASS/FAIL 의미를 확장하지 않았다.
- 새 acquisition/bundle/store envelope는 각각 version 1로 분리했다.
  P0/P1A schema와 기존 계약은 유지했다. 상세 결정은 tasks/P1B-INDEX.md에 기록했다.

## 실제 process integration test 결과

**18/18 통과.** Rust executable fixture를 실제 child process로 실행했다.

- exit 0, stderr + exit 7, signal 종료, timeout, executable 없음을 구분했다.
- stdout/stderr 저장·재읽기, NUL/invalid UTF-8 보존, cwd/env/인자 전달을 확인했다.
- 양쪽 stream 대량 출력, capture 한도 초과의 명시적 실패를 확인했다.
- timeout 후 process group의 하위 process가 지연 파일을 생성하지 못함을 확인했다.
  초기 EOF 종료 지연 재현을 최대 250ms cleanup 대기로 수정하고 통과했다.
- run_id/claim/hash 연결, canonical roundtrip, 중복 run/evidence/result 거부,
  동시 예약 시 단일 성공을 확인했다.
- 부분 write·손상·누락·변조·비canonical·symlink evidence 및 result 변조를 거부했다.

## 전체 테스트 수

**95개 통과 = 기존 P1A 77개 + 신규 P1B 18개. 실패·ignored 0.**
기존 테스트·JSON fixture·schema에는 diff가 없다. 독립 테스트 파일 SHA-256도
`cc4c5b9394cb25d32b0bd5cf18f3f36872f8b9c1ba3c6269d3ee494eacd1b1cb`로 유지됐다.

Darwin arm64 / Rust 1.98.1에서 아래 모두 통과했다.

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace --release`
- `git diff --check`

## 변경 파일

- `.gitignore`, `Cargo.lock`
- `crates/verify-runner/Cargo.toml`, `src/lib.rs`, `src/process.rs`
- `crates/verify-evidence/src/lib.rs`, `src/store.rs`
- `crates/verify-core/Cargo.toml`, `src/lib.rs`, `src/acquisition.rs`, `tests/process_acquisition.rs`
- `tests/fixtures/process.rs`
- `tasks/P1B-INDEX.md`, `state/CURRENT.md`, `P1B-RESULT.md`

## 남은 제한

Unix의 신뢰된 local target 전용이며 보안 격리가 아니다. 의도적으로 process group을
벗어난 descendant까지 정리한다고 보장하지 않는다. 각 stream은 1 MiB까지이며 초과는
실패다. 환경 값은 명시적으로 저장된다. Linux는 이번 호스트에서 검증하지 않았고,
Windows backend는 없다. 원자적 저장은 hard-link/directory-fsync를 지원하는 local
filesystem을 요구한다. 해시는 동일 사용자 공격에 대한 인증 수단이 아니다.

Target revision의 실제 executable 일치 확인, reset, replay 실행, product checker는
구현하지 않았다. 실제 시간과 출력의 실행 간 byte 동일성은 보장하지 않는다.
P1A 독립 재리뷰를 수행했다고 주장하지 않으며, 이번 명시적 P1B 요청에 따라 진행했다.

## P1C 시작 가능 여부

**요청된 P1B 로컬 완료 조건 충족. P1C는 별도 요청 시 검토·시작 가능하며 현재 미시작.**
Behavior product 및 범위 밖 기능은 구현하지 않았다. commit/push/deploy도 하지 않았다.
