# P3A 결과 — Baseline Stability / Noise Profiler

- **Stability semantics:** BEFORE를 기본 5회 실제 P1 acquisition으로 실행한다.
  고유 run ID, fresh snapshot workspace, 동일 입력/seed/실행 파일 identity를 사용한다.
  exit/signal/stdout raw bytes/stderr raw bytes를 STABLE/UNSTABLE/INCOMPLETE로 분류한다.
  최소 2회 미만 또는 한 번이라도 불완전하면 profile은 INCOMPLETE다.
  Stable 항목 차이는 DIVERGENCE_PROVEN, 불안정만 남거나 profile 불완전이면
  INCONCLUSIVE다. 네 항목 모두 stable·동일한 경우만 NO_DIVERGENCE_FOUND가 가능하다.
  별도 profile/comparison schema v1을 추가했고 기존 P2 exact mode는 유지했다.
- **테스트:** 신규 **24개**, 기존 138개 보존, 전체 **162/162 PASS**.
  A–J와 재해시·증거 손상 방어를 실제 executable로 검증했다.
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo build --workspace --release` 모두 PASS.
  Darwin arm64 / Rust 1.98.1, 2026-09-15; failed/ignored/warning 0.
- **False-PASS 방어:** 저장된 모든 P1 run/evidence를 다시 검증하고 분류·결과를
  재계산한다. 잘못된 profile identity/input/target/hash, 누락·손상·중복 run,
  재해시한 허위 STABLE/PASS, 반복 수를 낮춘 outlier 제거를 거부한다.
  UNSTABLE 값의 자동 ignore/정규화와 baseline 자동 갱신은 없다.
- **남은 제한:** bounded stability이며 미래 결정성·동등성 증명이 아니다.
  Trusted local Unix, 기존 snapshot/stream 한도와 host/runtime/외부 상태 제한을
  유지한다. Hash는 인증이 아니며 시간 기반 만료·격리·자동 paired replay는 없다.
  FAIL은 replayability unavailable 사유와 기록 입력을 이용한 재실행 안내를 남긴다.
- **P3B 시작 가능 여부:** P3A local gate PASS로 선행 조건은 충족했다.
  **P3B/P3C 미시작, 별도 요청 필요.** Commit/push/deploy도 수행하지 않았다.
