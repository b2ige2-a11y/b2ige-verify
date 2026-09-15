# P1A INDEPENDENT GATE

## VERDICT

**FAIL**

**P1B: DO NOT START.** 기존 테스트가 모두 통과하는 구현에서 SideEffect의
False PASS 경로를 재현했다. BLOCKER 2건, HIGH 4건, MEDIUM 2건을 기록한다.
이는 독립 gate review이며 production 수정, 기능 추가, P1B 구현은 수행하지 않았다.

- 검토일: 2026-09-14, Asia/Seoul.
- 검토 기준 commit: `0b2f6cf57fa75ee292071884fc3f606c8ed46caf`.
- 시작 시 `git status --short` 출력 없음: 기존 변경 없는 worktree에서 검증했다.
- 환경: Rust 1.98.1, `aarch64-apple-darwin`.
- 먼저 요청된 P0/P1A 문서 전체를 읽고, 이어서 4개 crate의 구현, contracts,
  conformance 테스트 전체, 5개 schema와 10개 fixture, fixture generator를 검토했다.
  추가로 CANONICALIZATION, SAFETY, P0-CONFORMANCE도 확인했다.
- 신뢰된 합성 policy/observer/preflight 입력이라는 명시적 한계를 인정했다.
  공격자가 이 경계를 이미 장악했다고 가정하는 것만으로 인증 취약점을 주장하지 않는다.

## Re-run results

추가 테스트를 작성하기 **전**, 요청된 명령을 순서대로 실행했다.

| Check | 실제 결과 |
| --- | --- |
| `cargo fmt --check` | PASS, exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, exit 0, 경고 없음 |
| `cargo test --workspace` | PASS, conformance 48/48, 실패·ignored 0 |
| `cargo build --workspace --release` | PASS, exit 0 |

기존 P1A RESULT의 실행 결과와 불일치는 없었다. 이 결과는 semantic correctness를
보증하지 않는다. 같은 구현에 아래의 독립 입력을 적용하면 계약 위반이 드러난다.

추가 테스트 작성 후 fmt/clippy도 통과했다.
`cargo test --workspace --no-fail-fast`는 **exit 101**이다.
기존 conformance는 계속 48/48 통과하고, 독립 테스트는 **21개 중 8 통과 / 13 실패 /
0 ignored**다. 전체는 69개 중 56 통과 / 13 실패다.
production 코드와 dependency가 그대로이므로 release build 결과는 최초 검증 결과를 사용한다.

## Findings

### R-01 — 다른 identity 하나가 포함되면 입증된 중복 commit을 버리고 PASS

- **ID:** R-01
- **Severity:** BLOCKER
- **Location:** [commit 집계](crates/verify-core/src/lib.rs:313),
  [재현 IG-01](crates/verify-core/tests/independent_gate.rs:52).
- **Attack:** 신뢰된 policy는 그대로 둔다. 같은 run/source/claim/trust의 첫 기록은
  purchase-1의 payment-1, 둘째 기록은 purchase-1의 payment-2를 포함한다.
  이때 FAIL임을 먼저 확인한다. 둘째 기록에 different-purchase의 other-payment를
  하나 추가하고 정상 payload hash를 계산한다. 모든 identity 문자열은 유효하다.
- **Expected:** 이미 두 in-domain commit이 있으므로 FAIL. snapshot domain을 엄격히
  단일 identity로 제한한다면 malformed observer output을 ERROR로 거부해도 된다.
  어느 해석에서도 PASS는 불가하다.
- **Observed:** `effects.iter().any(...)`가 domain 불일치 하나를 찾으면 둘째 기록 전체를
  `continue`한다. payment-2도 함께 사라져 count=1, **PASS**, reasons=[], counterexamples=[]가 된다.
  harness schema도 이 입력을 승인한다.
- **Why it matters:** 정상 identity의 위반 증거에 다른 항목을 추가하면 위반이 사라진다.
  observer의 범위 오류를 verifier ERROR로 처리하지 않고 product 성공으로 바꾼다.
  다른 run evidence 주입이나 authority label 위조가 필요하지 않다.

### R-02 — 선언한 correlation field를 관측하지 않아도 exactly_once PASS

- **ID:** R-02
- **Severity:** BLOCKER
- **Location:** [SideEffect policy 검사](crates/verify-core/src/lib.rs:162),
  [실제 집계](crates/verify-core/src/lib.rs:305),
  [재현 IG-02](crates/verify-core/tests/independent_gate.rs:83).
- **Attack:** effect의 `identity.correlation_fields`를 `["customer_id"]`로 선언한다.
  plan/replay hash는 정상 갱신한다. 증거 모델에는 customer_id가 없으며 기존
  provider/operation/idempotency_identity/external_effect_id만 존재한다.
- **Expected:** P1A에서 해석할 수 없는 identity 계약은 ERROR로 거부해야 한다.
  선언된 domain을 관측하지 못했다는 판정이라도 PASS는 금지해야 한다.
- **Observed:** correlation_fields의 문자열이 비어 있지 않은지만 검사한다.
  checker는 항상 고정된 세 필드로 필터링하며 **PASS**한다.
- **Why it matters:** trusted policy라도 구현이 지원하지 않는 필수 요구사항을 선언할 수 있다.
  "policy가 trusted"라는 전제는 그 요구사항을 무시할 권한이 아니다.
  P1A는 제한된 exactly_once를 지원한다고 밝혔지만 지원 가능한 correlation field 집합을
  강제하지 않아 선언한 scope보다 좁게 검사한다.

### R-03 — 승인 artifact pin과 실제 executable predicate가 연결되지 않음

- **ID:** R-03
- **Severity:** HIGH
- **Location:** [policy 유효성·승인 검사](crates/verify-core/src/lib.rs:172),
  [IG-03](crates/verify-core/tests/independent_gate.rs:98),
  [IG-04](crates/verify-core/tests/independent_gate.rs:122).
- **Attack:** 승인된 invariant의 statement는 `state must equal true`로 유지한다.
  observed=false인 FAIL 대조군에서 claim의 `Equals.expected`만 false로 변경하고
  plan hash를 갱신한다. 승인 artifact와 승인 pin은 건드리지 않는다.
  Behavior에서도 같은 변경을 baseline 및 observation_contract_hash 변경 없이 수행한다.
- **Expected:** 승인된 계약과 연결되지 않은 executable checker를 그 계약의 승인 결과로
  취급하지 않아야 한다. predicate를 포함한 기계 계약을 pin에 묶거나, 별도로 승인된
  plan과의 연결을 계약으로 명시해야 한다. 자연어를 LLM으로 판정하라는 요구가 아니다.
- **Observed:** 두 경우 모두 FAIL에서 **PASS**로 바뀐다. BlindTest는 invariant ID와
  observer 이름만 claim과 비교한다. Behavior의 observation_contract_hash는 형식만 검사하며
  현재 검사 대상 객체의 hash와 비교하지 않는다.
- **Why it matters:** artifact 자체의 변조는 잘 막지만, 그 artifact가 승인한 검사와 실제
  수행한 검사가 같다는 보장은 없다. 이는 trusted policy 작성기/컴파일러의 semantic
  오류를 드러내는 재현이며, **외부 공격자가 고정 policy를 우회했다는 증거는 아니다**.
  실제 Behavior engine이나 인증 서비스 부재 자체를 결함으로 세지 않았다.

### R-04 — 재현할 target·observer 정보를 확보하지 못해도 replay available

- **ID:** R-04
- **Severity:** HIGH
- **Location:** [ReplayInputs::assess](crates/verify-replay/src/lib.rs:23),
  [replayability 선계산](crates/verify-core/src/lib.rs:203),
  [IG-05·06](crates/verify-core/tests/independent_gate.rs:145).
- **Attack:** FAIL fixture의 plan/context/replay target을 모두 `main`으로 맞춘다.
  별도 변형에서는 required observer version을 모두 삭제한다.
- **Expected:** resolved immutable target identity가 없으면 replay unavailable 또는 명시적인
  더 약한 metadata-only 상태. 필수 observer metadata가 빠진 run도 available을 주장하면 안 된다.
- **Observed:** 첫 경우 `FAIL + replayability: available`; 둘째는 `ERROR + available`이다.
  replay 결과는 RunContext 유효성 검사 전에 계산되며 observer_versions를 검사하지 않는다.
  target은 비어 있지 않은 문자열인지와 문자열 일치만 확인한다.
- **Why it matters:** 외부 세계의 동일성을 요구하지 않아도, 움직이는 ref와 누락된 observer
  버전으로 원래 controllable experiment의 identity를 고정했다고 주장할 수 없다.
  `HEAD`도 같은 코드 경로다. 실제 replay 실행/재현 성공은 이번에도 수행하지 않았다.

### R-05 — RFC 8785 JCS를 주장하는 해시가 표준 입력에서 불일치

- **ID:** R-05
- **Severity:** HIGH
- **Location:** [canonical_hash](crates/verify-evidence/src/lib.rs:111),
  locked dependency `serde_jcs 0.1.0`의 `src/ser.rs` / `src/entry.rs`,
  [IG-07·08](crates/verify-core/tests/independent_gate.rs:176),
  [IG-19](crates/verify-core/tests/independent_gate.rs:435).
- **Attack:** escaped control character와 BMP/비-BMP 문자로 된 property 이름,
  정확히 binary64로 표현 가능한 `2^60`의 u64/f64 표현, public API의 NaN을 넣는다.
- **Expected:** unescaped UTF-16 code-unit 순서, ECMAScript 숫자 직렬화,
  NaN/Infinity 거부라는 [RFC 8785 §3.2.2–3.2.3](https://www.rfc-editor.org/info/rfc8785/#section-3.2.3)
  규칙과 일치해야 한다.
- **Observed:** dependency는 escaped UTF-8 key bytes를 BTreeMap으로 정렬한다.
  독립적으로 계산한 key-order vector의 예상 hash는
  `sha256:4bd52d82f332c2e5c7206abd57c74b45dd4f0fef63ab7af87b8bde4481e450e7`,
  실제는 `sha256:6dbfcd316c63a0484784bb474cbc26478376e1cb9687286389d8d224887fbdfe`다.
  같은 2^60 값도 u64와 f64 hash가 다르다. `canonical_hash(&f64::NAN)`은 Err가 아니라
  Ok를 반환한다. serde_json serializer가 NaN을 null로 직렬화해 formatter의 거부를 우회한다.
- **Why it matters:** 승인 pin, plan/config, evidence hash가 다른 JCS 구현과 호환된다는
  계약을 만족하지 못한다. SHA-256 충돌을 발견했다는 뜻은 아니다.
  raw JSON의 `NaN`은 parser가 거부한다. NaN 경로는 generic Rust public API에서 확인했다.

### R-06 — 같은 canonical plan hash와 같은 evidence인데 PASS/FAIL이 달라짐

- **ID:** R-06
- **Severity:** HIGH
- **Location:** [Equals 비교](crates/verify-core/src/lib.rs:294),
  [비교 결과 판정](crates/verify-core/src/lib.rs:335),
  [IG-09](crates/verify-core/tests/independent_gate.rs:196).
- **Attack:** expected=1, observed=1인 PASS를 만든다. 이후 expected를 JSON의 1.0으로
  바꾼다. plan_hash, replay plan, evidence는 그대로 둔다.
- **Expected:** canonical identity가 같은 계약은 동일한 의미로 비교되어야 한다.
  숫자 표현을 별도 타입으로 구분하려면 그 차이를 canonical 계약에 표현해야 한다.
- **Observed:** 실제로 plan canonical hash는 같고 run identity 검사도 통과하지만 **FAIL**이 된다.
  serde_json::Value equality는 정수와 부동소수점 저장형을 구분한다.
- **Why it matters:** hash가 식별한 experiment와 checker의 의미가 일치하지 않는다.
  재직렬화나 replay에서 가짜 divergence가 생길 수 있다. 재현된 현상은 false FAIL 및
  동일 hash에 대한 verdict 불일치이며, 실제로 서로 다른 수치가 같게 처리되는 공격과 구분한다.

### R-07 — duplicate JSON coverage key가 UNAVAILABLE을 지움

- **ID:** R-07
- **Severity:** MEDIUM
- **Location:** [Run.coverage](crates/verify-core/src/lib.rs:96),
  [fixture raw parsing](tests/conformance/fixtures.rs:44),
  [IG-10](crates/verify-core/tests/independent_gate.rs:219).
- **Attack:** raw JSON의 coverage에 `"state": {status: unavailable, ...}` 다음으로
  같은 `"state"`의 complete record를 넣는다.
- **Expected:** 중복 key를 raw-input 경계에서 거부하여 어느 레코드가 authoritative인지
  모호해지지 않게 해야 한다.
- **Observed:** `serde_json::from_str::<Run>`의 BTreeMap은 마지막 key를 유지한다.
  evaluate는 사라진 UNAVAILABLE을 볼 수 없고 PASS한다. 먼저 Value로 읽는 기존 fixture
  경로도 중복 key를 보존할 수 없다. deny_unknown_fields는 중복 map key 방어가 아니다.
- **Why it matters:** schema 검사 이전에 정보가 사라진다. [RFC 8785 §3.1](https://www.rfc-editor.org/info/rfc8785/#section-3.1)의
  duplicate property 금지와도 맞지 않는다. 현재 envelope가 trusted test input이고
  외부 import 서비스가 없으므로 이를 현재 배포된 untrusted-input 취약점으로 과장하지 않는다.

### R-08 — snapshot drift test가 legacy schema/실제 역직렬화 차이를 놓침

- **ID:** R-08
- **Severity:** MEDIUM
- **Location:** [legacy 검사 이전 roundtrip](tests/conformance/fixtures.rs:14),
  [snapshot 비교](tests/conformance/fixtures.rs:110),
  [Baseline Option 필드](crates/verify-core/src/contracts.rs:13),
  [harness seed](schemas/conformance-fixture.schema.json:881),
  [IG-11·12](crates/verify-core/tests/independent_gate.rs:240).
- **Attack:** baseline raw object에 `notes: null`을 넣는다. 별도 사례에서는 harness seed에
  `18446744073709551616`(2^64)을 넣어 schema와 Rust acceptance를 비교한다.
- **Expected:** 원본 제품 artifact를 legacy schema로 검사한 뒤 decode하거나 두 표현의
  차이를 명시해야 한다. harness numeric range는 실제 u64와 일치해야 한다.
- **Observed:** notes=null은 P0 schema에서 거부되지만 Rust는 None으로 받고 재직렬화 시
  삭제한다. 기존 테스트 경로가 검사하는 재직렬화된 baseline은 schema를 통과한다.
  2^64는 harness schema를 통과하고 Rust에서는 거부된다. `format: uint64`만 있고 maximum이 없다.
- **Why it matters:** JSON-004는 Rust에서 생성한 새 schema의 snapshot 변경만 검출한다.
  현재 snapshot 자체의 의미가 Rust/기존 P0 schema와 일치하는지는 증명하지 못한다.
  큰 seed는 Rust가 최종 거부하므로 이 사례 자체가 False PASS는 아니다.

## New adversarial tests

[독립 테스트 파일](crates/verify-core/tests/independent_gate.rs)
하나를 추가했다. Cargo의 자동 integration test 탐색을 사용하므로 manifest나 기존 테스트
entrypoint는 수정하지 않았다. 기존 golden 파일은 입력만 사용하며 expected verdict를
oracle로 사용하지 않는다. 기존 support.rs도 import하지 않는다.

| Test | 결과 | 의미 |
| --- | --- | --- |
| IG-01 | 실패 | 중복 commit + mixed domain → PASS, R-01 |
| IG-02 | 실패 | 미지원 correlation field → PASS, R-02 |
| IG-03 | 실패 | invariant pin과 predicate 분리, R-03 |
| IG-04 | 실패 | baseline pin과 checker 분리, R-03 |
| IG-05 | 실패 | main target이 replay available, R-04 |
| IG-06 | 실패 | observer version 누락에도 replay available, R-04 |
| IG-07 | 실패 | RFC key-order vector 불일치, R-05 |
| IG-08 | 실패 | Rust NaN hash가 Ok, R-05 |
| IG-09 | 실패 | 동일 canonical plan의 verdict 불일치, R-06 |
| IG-10 | 실패 | duplicate coverage key 허용, R-07 |
| IG-11 | 실패 | legacy-invalid null의 roundtrip 통과, R-08 |
| IG-12 | 실패 | schema/Rust u64 범위 차이, R-08 |
| IG-13 | 통과 | empty claims/observers/scope/trust, empty·zero budget 차단 |
| IG-14 | 통과 | 잘못된 run/source/claim, derived/advisory, 중복 ID·손상 hash 차단 |
| IG-15 | 통과 | attempts/commits A·B·C, 다른 run, direct_runtime 대조군 |
| IG-16 | 통과 | baseline/invariant 내용·ID·pin 변경 및 provenance 누락 차단 |
| IG-17 | 통과 | incomplete coverage, runner ERROR 우선순위, isolation preflight |
| IG-18 | 통과 | unknown observation/run·harness version, raw NaN 거부 |
| IG-19 | 실패 | 같은 binary64 정수의 JCS 표기 차이, R-05 |
| IG-20 | 통과 | 문서화된 synthetic authority/isolation 경계의 실제 동작 |
| IG-21 | 통과 | 충돌 evidence, 동일 order, 순서 역전, 다른 critical observer 누락 |

재실행:

```sh
cargo test -p verify-core --test independent_gate -- --nocapture
cargo test --workspace --no-fail-fast
```

실패 assertion은 gate finding의 재현 증거로 남겼다. ignored/should_panic을 사용하지 않았으며
실패를 성공으로 바꾸기 위해 구현이나 기대값을 수정하지 않았다. 13개 assertion 실패는
13개의 독립 취약점을 뜻하지 않는다. 원인별로 8개 finding으로 묶었다.

## False-PASS assessment

**False PASS가 존재한다.** R-01은 policy를 변경하지 않고도 재현되고, R-02는 선언된
identity 요구를 구현이 무시하는 경로다. 그러므로 현재 P1A gate를 유지할 수 없다.

검증 범위별 구분은 다음과 같다. "차단"은 읽은 코드와 실행한 한정 corpus에 대한 결과다.

| 공격 영역 | 확인 결과 및 한계 |
| --- | --- |
| contract violation + runner failure | ERROR 우선. 기존 V-006과 IG-17 확인. observer 고장을 target FAIL로 취급하지 않음 |
| FAIL + 다른 critical evidence 누락 | 지원되는 위반은 FAIL 유지. V-010, IG-21 |
| COMPLETE observer + 다른 critical UNAVAILABLE | 충분한 위반 없으면 INCONCLUSIVE. COV-003, IG-21 |
| empty requirements/claims/scope, zero exploration | 정상 plan hash로 재연결해도 ERROR. 단순 hash mismatch 때문에 거부된 테스트가 아님 |
| duplicate claim/evidence identity | claim ID 중복은 policy.valid에서, evidence ID 중복은 evaluate에서 ERROR. 별도 observation_id 필드는 없음 |
| unknown observation/coverage enum | serde/schema에서 거부. 알려진 다른 observation variant는 해당 predicate에서 필터링됨 |
| 같은 claim의 충돌 evidence, 순서 역전 | Equals는 모든 eligible value를 비교하여 위반 유지. order로 last-write-wins 하지 않음 |
| stale evidence | 다른 run_id는 ERROR. 같은 payload에 현재 run/source/trust를 재라벨링한 경우는 인증 범위 밖이며 별도 freshness 검증 없음 |
| evidence self-authority | trust/source/claim 라벨만 맞추면 충분한 evidence로 취급됨. 코드와 문서가 명시한 trusted observer 입력 경계 때문에 이 사실만을 BLOCKER로 세지 않음 |
| evidence hash 범위 | observation payload만 hash함. run/source/trust/related_claim_ids/order는 그 hash에 포함되지 않음. hash를 provenance 인증으로 사용할 수 없음 |
| derived/advisory | 그대로는 PASS/FAIL 근거가 되지 않음. authoritative wrapper로 재포장한 원래 출처는 현재 모델이 검증하지 않음 |
| quorum/coverage 중복 | quorum 구현 없음. duplicate evidence ID 거부. 별도 ID로 같은 external effect를 반복해도 set dedup. raw JSON key는 R-07 |
| observer policy 제거 | claim.observer를 required list에서만 빼면 ERROR. claim/scope까지 함께 변경한 새 trusted policy는 더 좁은 계약임. 원래 업무 요구 완전성까지 검증하지 않음 |
| claim의 복수 observer | Claim.observer는 단일 문자열. 서로 다른 claim으로 나눠 검사 가능하나 한 claim의 HTTP+DB 관계는 아직 직접 표현하지 못함 |
| PARTIAL/UNAVAILABLE/FAILED | 모두 PASS 차단. init/runner 실패는 ERROR, 실행 후 coverage 부족은 INCONCLUSIVE. partial에서도 관측된 commit 수가 상한을 넘으면 FAIL 가능 |
| SideEffect A | attempts=2, commit unknown → INCONCLUSIVE, IG-15 |
| SideEffect B | attempts=2, commits=1 → PASS, IG-15 |
| SideEffect C | attempts=1, commits=2 → FAIL, IG-15 |
| SideEffect D | 같은 external_effect_id는 반복 관측으로 dedup. schema decision과 SE-004에 명시. provider가 같은 ID를 다른 실체에 재사용하는지 입증하는 기능은 없음 |
| SideEffect E | 다른 run의 commit을 한 run에 넣으면 ERROR. IG-15 |
| SideEffect F | provider confirmation required에서 direct_runtime만으로는 INCONCLUSIVE. IG-15, SE-007 |
| SideEffect domain/window | mixed domain/지원하지 않는 correlation은 R-01/02. eventual_window_ms와 실제 관측 시간의 관계는 검사하지 않으며 COMPLETE라는 trusted attestation에 의존 |
| baseline label·artifact 변조 | approved만으로 부족. pin·actor·reason·stable 필요. revision/hash/artifact 변경 후 기존 pin 재사용은 차단. 실제 baseline payload와 checker 연결은 R-03 |
| empty baseline | 빈 ID/revision/불량 hash는 거부. 실제 observation dataset은 모델에 없어 빈 dataset 승인 여부까지 증명할 수 없음 |
| invariant label·artifact 변조 | authoritative만으로 부족. pin/source_ref 및 origin에 따른 승인자 필요. 빈 statement·ID 변경·provenance 누락 차단. 임의의 비어 있지 않은 자연어 문장은 parser가 검증하지 않음 |
| invariant version | 별도 version 필드 없음. unknown field는 거부. harness/run schema_version은 1만 허용; plan.version은 비어 있지 않은 식별 문자열이며 semantic dispatch 없음 |
| requested/observed isolation | plan.isolation과 assessment.available은 분리. validate가 requested만 반환하므로 available만 높여 출력 claim이 자동 상승하지 않음 |
| hardened claim | required/available/linux_backend를 모두 만족하는 synthetic 입력이면 runtime 증거 없이 hardened claim 가능. IG-20은 이를 명시적 테스트 한계로 확인 |
| unsupported platform/preflight | linux_backend=false, 보호 artifact 노출, available 부족이면 ERROR. free-text platform_runtime를 실제 backend와 교차 검증하는 기능은 없음 |
| replay 필수 입력 | seed/plan/config/config hash/fault schedule/target 누락 및 흔한 불일치 → unavailable. 기존 RP-001/002 확인 |
| replay 다른 run 복사 | plan/config/seed/target 등이 다르면 unavailable. 같은 controllable 입력이면 run_id가 달라도 구별하지 않음. 실험 입력 재사용 계약에서 이것만을 취약점으로 세지 않음 |
| empty replay inputs | 전체 누락/빈 plan은 unavailable. standalone assess는 nonempty object만 검사하는 예비검사이며 core의 hash binding보다 약함 |
| fault schedule | 명시적 []는 no-fault 계획으로 허용. 누락은 unavailable. core에서 trusted config의 schedule과 비교 |
| corrupt hash/unknown version | verdict는 ERROR 또는 decode 거부. replay availability가 별도 선계산되는 약점은 R-04; 같은 canonical hash의 다른 평가 의미는 R-06 |

## Test quality assessment

기존 테스트는 public `evaluate`, `run_result`, `validate_fixture`, canonical_hash를 호출한다.
직접 verdict assertion이 있고, 실행된 panic이 성공으로 처리되는 구조나 `should_panic`에
의존한 PASS는 발견하지 못했다. 네 negative fixture가 먼저 harness schema를 통과하는지도
검사하므로 모두 단순 schema reject만 시험하는 것은 아니다.

다만 다음 한계는 분명하다.

1. fixture generator는 `expected = evaluate(...)`로 현재 구현에서 golden output을 생성한다.
   그 뒤 전체 Evaluation equality를 검사하는 fixture test는 독립 oracle가 아니다.
   자동 재생성은 없지만 수동 재생성이 잘못된 해석을 고정할 수 있다.
2. 모든 기존 cases는 같은 support.rs의 작은 happy path에서 시작한다. R-01의
   mixed-domain record, R-02의 다른 correlation 필드, Unicode/numeric hash 경계는 빠져 있다.
3. V-008은 이름에 scope까지 포함하지만 claims만 비우며 plan hash도 재연결하지 않는다.
   policy empty guard가 없어도 hash mismatch로 ERROR가 될 수 있다.
   IG-13은 각 empty/zero 조건을 분리하고 hash를 재연결했다.
4. E-006은 ASCII key 순서와 true/false만 확인한다. 실제 RFC 정렬/숫자 표준 vector는 없다.
5. JSON-004는 새 schema snapshot과 생성물의 일치만 검증한다. JSON-005의 legacy negative는
   대부분 `{}` 거부와 creator enum 한 사례다. R-08 같은 acceptance drift는 놓친다.
6. B/BT pin tests는 metadata 변경을 검사하지만 승인 내용과 executable predicate의 연결은
   검사하지 않는다. replay test도 mutable ref와 observer version 누락 시 availability를
   검사하지 않는다.
7. 표준 cargo 결과는 branch coverage를 측정하지 않는다. 전체 dead-code/unreachable-branch
   부재를 증명하지 않았다. 특히 valid policy 이후 비교 가능한 observation variant가
   필터링되는 경로를 안전하다고 가정해서는 안 된다.

## Scope honesty

P1A는 **trusted synthetic conformance harness**다. crate-level 문서,
P1A-CONFORMANCE, SCHEMA-DECISION 및 Evaluation.limitations에서 이 점을 분명히 밝힌다.
따라서 실제 인증 서비스, human approval 검증, provider observer, sandbox가 없다는
이유만으로 gate를 떨어뜨린 것은 아니다.

현재 구현은 다음을 증명하지 않는다.

- source/trust/run 라벨의 진위, 원천 evidence의 freshness, 숨겨진 derived/advisory provenance.
- 실제 process execution, observer COMPLETE 주장, exploration budget 달성,
  provider의 commit identity 유일성 및 관측 window 충족.
- baseline dataset의 내용·안정성·정확성, 자연어 invariant와 executable checker의 의미 일치.
- Linux sandbox 실행 여부, hidden grader 비노출, macOS/Windows의 동등한 격리.
- replay 실행, failure reproduction, 외부 세계 상태의 동일성.
- 문자열 timestamp의 형식·시간 역전·order 유일성. RunContext는 timestamp의 nonempty만 검사한다.
- SHA-256로 서명이나 인증을 대체하는 것, 완전한 proof 또는 exhaustive adversarial coverage.

이러한 한계 표시는 대체로 정직하다. 그러나 명시적으로 구현했다고 주장한 evidence 집계,
identity scope, JCS 및 replay metadata의 의미에는 위 finding이 남는다.
P1A-RESULT의 bounded-corpus PASS는 이번 독립 gate PASS로 해석할 수 없다.

## P1B recommendation

**DO NOT START**

별도 repair task에서 R-01/R-02를 먼저 해결하고, HIGH 항목의 계약 의미와 검증을 정리한 후
독립 gate를 다시 실행해야 한다. schema-visible 해결에는 기존 규칙대로 명시적 version
decision이 필요하다. 이번 리뷰는 schema나 production을 자동 수정하지 않았다.

이번 변경은 이 보고서, 독립 테스트 파일, 그리고 독립 FAIL을 반영한
`state/CURRENT.md`의 gate 기록뿐이다. 기존 구현/fixture/기존 테스트와 prior implementation의
검증 기록은 보존했다. 신규 commit, push, 배포는 수행하지 않았다.
