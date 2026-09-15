# P3B 결과 — Deterministic Case Generator

- **Generator semantics:** 승인된 base와 명시 domain에서 input만 변경한다.
  Spec/suite schema v1, spec hash·parent·assignment·input에 결합된 deterministic
  case ID와 순서를 보존한다. 기존 P2 exact engine으로 각 BEFORE/AFTER를 실제 실행한다.
  Executable, approval, policy, timeout, fixture contract, observers와 P3A semantics는 유지한다.
- **Generation modes:** ONE_AT_A_TIME은 dimension→candidate 순서, CARTESIAN은 마지막
  dimension이 가장 빠르게 변한다. Base 자동 포함·truncation·자동 deduplication은 없다.
  빈 입력, 중복 target/candidate/결과 입력, 잘못된 index, budget 초과는 실행 전에 거부한다.
- **신규/전체 테스트:** 신규 **22개**, 기존 162개 보존, **184/184 PASS**, failed/ignored 0.
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo build --workspace --release` 모두 최종 PASS.
  Test gate는 **RUST_TEST_THREADS=4** 환경에서 실행했다. 기본 병렬도 최초 실행은
  짧은 timeout의 정상 실행까지 지연되어 기존 P3A 1개·P3B 2개가 실패했다.
  테스트 내용·timeout·판정 기준을 바꾸지 않고 병렬도만 제한하여 전체 통과했다.
- **Aggregate verdict 규칙:** generation 무결성 오류는 ERROR(Err).
  검증된 children은 FAIL > ERROR > INCONCLUSIVE > PASS. Expected cases 전체가 존재하고
  모두 완전하게 실행되어 NO_DIVERGENCE_FOUND일 때만 PASS다.
  FAIL+INCONCLUSIVE/ERROR는 FAIL을 유지하고 incomplete coverage를 기록한다.
- **False-PASS 방어:** atomic 저장 후 load 시 spec에서 ordered cases를 재생성하고,
  실제 child comparison/run/evidence 및 전체 experiment/input/hash를 검증한다.
  마지막 실행 이후 모든 children을 재검증하며 counts·coverage·verdict를 재계산한다.
  누락·손상·변조·재해시한 허위 PASS/집계·spec mismatch는 verified PASS로 로드되지 않는다.
- **남은 제한:** 명시 domain의 bounded testing, trusted local Unix와 기존 P2 제한이다.
  Hash는 인증이 아니며 cross-platform 실행 검증은 하지 않았다. P3A 자동 적용, fuzzing,
  mutation, LLM generation, reducer, 자동 paired replay는 없다. FAIL 증거와 replayability
  unavailable 사유는 child에 보존한다.
- **P3C 시작 가능 여부:** **P3B LOCAL GATE PASS**, 선행 조건 충족.
  **P3C 미시작**, 별도 요청 필요. Commit/push/deploy도 수행하지 않았다.
