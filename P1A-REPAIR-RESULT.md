# P1A REPAIR RESULT

**R-01~R-06 수정 및 회귀 검증 완료. P1B 미시작.**

2026-09-14 · Rust 1.98.1 · aarch64-apple-darwin.
독립 리뷰 보고서는 당시의 실패 기록으로 보존했다. 이번 결과는 repair 검증이며,
별도 독립 재리뷰를 수행했다고 주장하지 않는다. 커밋·push·배포는 하지 않았다.

## 수정 내용

| Finding | 수정 및 확인 결과 |
| --- | --- |
| R-01 | effect 단위로 domain을 검사해 정상 domain의 commit을 유지한다. 혼합 domain은 불충분 사유를 기록하며, 이미 관측된 중복은 FAIL이다. 기존 재현은 PASS에서 FAIL로 바뀌었다. provider/operation/idempotency 변형과 PARTIAL에서도 중복을 보존한다. |
| R-02 | P1A의 지원 correlation rule을 `["idempotency_identity"]`로 제한했다. customer_id, 다른 필드·추가 필드·중복 필드·빈 목록은 ERROR다. |
| R-03 | 별도 `approved_checker_bindings`에 artifact 전체 hash와 executable Claim 전체 hash를 묶은 domain-separated 승인 identity를 요구한다. artifact 승인과 정확한 checker binding이 모두 필요하다. 기존 artifact를 둔 채 predicate를 false로 바꾸면 INCONCLUSIVE이며, 다른 artifact에 기존 binding을 재사용해도 차단된다. 평가와 rebind는 승인을 생성하지 않는다. |
| R-04 | 유효성 검사 전 replay는 unavailable로 시작한다. `ReplayInputs::assess` 자체도 RunContext와 required observers를 받아 metadata·plan/config/seed/fault schedule 일치를 확인한다. target은 content-addressed SHA-256 또는 전체 Git object ID만 허용하며 main/HEAD/tag/short ref는 unavailable이다. |
| R-05 | serde_jcs를 serde_json_canonicalizer 0.3.2로 교체했다. serde-value로 float를 보존해 중첩 NaN/Infinity를 JSON null 변환 전에 거부한다. Unicode 정렬, ECMAScript 숫자 표기, 재파싱 안정성 vector를 통과했다. exact seed/counter/budget/order는 2^53-1 이하로 제한해 반올림된 hash identity 충돌을 막는다. |
| R-06 | hash와 checker equality가 `canonical_bytes`를 공유한다. 중첩 JSON의 1/1.0, 0/-0도 같은 canonical 의미로 비교하며, boolean true와 숫자 1은 계속 구별한다. |

JCS 라이브러리 선택은 [공식 crate 문서](https://docs.rs/serde_json_canonicalizer/0.3.2/serde_json_canonicalizer/)
및 설치된 소스를 확인한 뒤 로컬 regression vector로 검증했다.

## 테스트 보존 및 fixture migration

`independent_gate.rs`는 **바이트 단위로 변경하지 않았다**. 전후 SHA-256:

`cc4c5b9394cb25d32b0bd5cf18f3f36872f8b9c1ba3c6269d3ee494eacd1b1cb`

기존 48개 테스트 함수와 assertion도 유지했다. 변경한 공통 support와 10개 golden fixture는
새 승인 binding과 synthetic target의 content address를 명시적으로 등록한다.
golden verdict 기대값은 전부 그대로다. error fixture의 replay 기대값만 새 계약에 따라
unavailable로 정정했다. evaluator 출력으로 golden verdict를 자동 재생성하지 않았다.

독립 IG-09는 승인 변경 없이 true → 1 → 1.0을 사용한다. 이 테스트를 수정하지 않으면서
숫자 동등성까지 실제 checker에서 검사하도록 Behavior fixture에 **Equals(true)와 Equals(1)의
두 고정 승인**을 등록했다. false, 2, null, 임의 object는 승인하지 않았으며 추가 regression으로
차단을 검증했다. 이것은 고정된 테스트 승인 데이터이며 candidate 변경 후 자동 재승인이 아니다.

## 검증 결과

수정 전 독립 테스트는 8 통과 / 13 실패로 재현했다. 수정 후 아래 순서로 실행했고 모두 exit 0이다.

| 명령 | 결과 |
| --- | --- |
| `cargo test -p verify-core --test independent_gate` | **21/21 PASS** |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, 경고 없음 |
| `cargo test --workspace` | **77/77 PASS**: 기존 48 + 독립 21 + 신규 8; 실패·ignored 0 |
| `cargo build --workspace --release` | PASS |
| `git diff --check` | PASS |

R-01/R-02의 기존 False PASS 재현 및 추가 변형은 모두 차단됐다.
R-03~R-06 독립 regression과 새 승인 재사용·중첩 nonfinite·큰 seed 검사도 통과했다.
이는 실행한 bounded corpus의 결과이며 exhaustive correctness proof가 아니다.

## 변경 파일

| 범위 | 파일 |
| --- | --- |
| 판정·승인·입력 계약 | [verify-core/lib.rs](crates/verify-core/src/lib.rs), [contracts.rs](crates/verify-core/src/contracts.rs) |
| canonicalization·정수·중복 map 검사 | [verify-evidence/lib.rs](crates/verify-evidence/src/lib.rs), [verify-runner/lib.rs](crates/verify-runner/src/lib.rs) |
| replay metadata 검사 | [verify-replay/lib.rs](crates/verify-replay/src/lib.rs) |
| 의존성 | 루트 Cargo.toml·Cargo.lock, crates/verify-evidence/Cargo.toml, crates/verify-replay/Cargo.toml |
| 테스트·fixture | [repair_regressions.rs](crates/verify-core/tests/repair_regressions.rs), tests/conformance/support.rs, tests/fixtures/*.json 10개 |
| schema maintenance | crates/verify-core/examples/generate_fixtures.rs에 `--schema-only` 추가, schemas/conformance-fixture.schema.json 갱신 |
| 문서·상태 | docs/CANONICALIZATION.md, [P1A-SCHEMA-DECISION.md](docs/P1A-SCHEMA-DECISION.md), state/CURRENT.md, 이 결과 문서 |

Schema decision: 미공개 synthetic harness v1의 fail-closed 보정으로 **version 1을 유지**했다.
추가 승인 필드와 강화된 숫자/null 조건, 호환성 영향을 명시했다. version 2는 계속 unsupported다.
P0의 네 기존 artifact schema는 변경하지 않았다. 기존 잘못된 JCS hash를 허용하는 fallback은 없다.

## R-07/R-08 처리 및 deferred

- **R-07 재현 사례 해결:** raw typed coverage map의 duplicate key를 거부한다.
  observer_versions와 exploration_budget에도 같은 좁은 검사를 적용했다.
  **Deferred:** 임의 untrusted JSON 전체에 대한 중복 key parser, upstream Value 파싱에서
  이미 지워진 key의 복구/검출. 이번에 별도 범용 parser는 만들지 않았다.
- **R-08 재현 사례 해결:** baseline notes 등의 optional 필드는 absent와 null을 구분한다.
  harness의 exact integer maximum도 명시했다. IG-11/12 모두 통과한다.
  **Deferred:** 모든 legacy schema와 Rust의 acceptance를 완전히 같게 만드는 전면 작업.
  기존 구조 검증과 semantic validity의 차이는 유지한다.

## 남은 제한과 P1B 준비 여부

승인 binding은 신뢰된 synthetic 입력이다. human/policy 인증 서비스, 실제 baseline engine,
자연어 invariant 해석, provider 관측, sandbox 및 replay 실행은 구현하지 않았다.
content-addressed target 표기는 실제 artifact를 조회·실행했다는 증거가 아니다.
JCS JSON number는 binary64 의미이며, 큰 exact 수치는 문자열로 표현해야 한다.
observer completeness/freshness·관측 window 보장도 계속 trusted 입력 경계에 의존한다.

**P1A repair의 요청된 회귀 검증은 완료했다. 독립 재리뷰와 별도 명시적 작업 전에는
P1B를 시작하지 않는다.** 이번 작업에서 P1B, 커밋, 배포는 수행하지 않았다.
