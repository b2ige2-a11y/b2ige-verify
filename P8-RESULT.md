# P8 결과 — B2IGE Verify Bench

Benchmark measurements below refer only to **B2IGE Verify Bench v1 — 31 explicit cases**, on the benchmark corpus.

**P8 COMPLETE / LOCAL RELEASE GATE PASS. P9 시작 가능, 미시작.**

P8 전체를 단일 Phase로 구현했다. P9 작업, commit, push, deploy, publish는 수행하지 않았다.

## 구현 범위

- `verify_cli::bench` 모듈: 기존 CLI verified report/Agent projection을 재사용한다.
  별도 crate 간 순환 의존성이나 새로운 verdict checker 없이 public execution →
  schema 검증 → product verified load → report verified load 경로를 실행한다.
- 독립 BenchmarkCase / BenchmarkCaseResult / BenchmarkSummary / BenchmarkRunResult **v1**,
  Classification 전용 enum, JSON schemas, canonical serialization/hash, human report.
  기존 P1–P7 제품 계약·판정·schema는 변경하지 않았다.
- 독립적으로 작성한 **31개** label/config/source identity와 실제 process·SQLite·Docker corpus.
  Expected label을 현재 output으로 생성하지 않으며 Behavior baseline 자동 승인·갱신이 없다.
- `b2ige bench`, 제품별 command, `--case`, `--output human|json`, `--save`,
  `--reverse`, `compare`, `schemas`. 빈/부분 실행은 전체 release gate를 통과하지 못한다.
  제품별 command는 완전한 해당 제품의 scoped gate를 별도로 반환한다.
- bounded 실행, fresh case workspace/SQLite/private suite/container, explicit replay/reproduction,
  local reduction, noise·generation·fault·committed-state·leakage·quality receipt 측정.
- 로컬 runner는 release build와 전체 정방향/역방향 실행, 두 gate 및 semantic equality를 확인한다.
  저장은 새 directory만 허용하며 baseline snapshot을 자동 덮어쓰지 않는다.

## 실제 release corpus 결과

| 제품 | Cases | PASS | FAIL | INCONCLUSIVE | ERROR | Benchmark gate |
|---|---:|---:|---:|---:|---:|---|
| behavior | 9 | 1 | 7 | 1 | 0 | PASS |
| sideeffect | 13 | 4 | 6 | 2 | 1 | PASS |
| blindtest | 9 | 2 | 3 | 2 | 2 | PASS |
| **전체** | **31** | **7** | **16** | **5** | **3** | **PASS** |

여기서 제품 FAIL 16개는 independently labeled known bugs의 정상 검출이다.
INCONCLUSIVE와 ERROR도 불완전/인프라 negative fixture의 독립 기대값과 일치한다.
Harness error와 expected mismatch는 모두 **0**이다.

| 필수/주요 지표 | 실제 결과 |
|---|---:|
| known_bug_false_pass | 0 / 16 |
| known_unsafe_false_pass | 0 / 6 |
| missing_evidence_pass | 0 / 6 |
| partial_hidden_suite_pass | 0 / 1 |
| isolation_unattested_pass | 0 / 1 |
| safe_control_false_fail | 0 / 7 |
| correct_blindtest_false_fail | 0 / 2 |
| evidence_validity | 31 / 31 |
| verified_reload | 31 / 31 |
| verified_reproduction | 16 / 16 |
| deterministic_replay | 5 / 5 |
| local_minimization | 7 / 7 |
| Mutation adequacy on benchmark corpus | 2 / 2 |
| suite_quality_receipt_accuracy | 4 / 4 |
| false PASS / false FAIL | 0 / 0 |
| target hidden canary leakage / Agent private-value leakage | 0 / 0 |

### Behavior

- 실제 divergence **7/7**, preserving control **1/1**, false divergence **0**.
- Counter baseline **INCONCLUSIVE**; unstable stdout에도 stable stderr change **FAIL**.
  Noise handling **2/2**.
- Base input PASS를 확인한 generated-only discovery **2/2**; 같은 spec의 생성 identity/order **2/2**.
- Reducible failure: explicit assignments **2 → 1**, 실제 재실행으로 signature 유지 및 local minimality 검증.
- FAIL reproduction **7/7**. 별도 exact process replay 측정 **5/5**.
  Stability/reduction 경로의 자동 replay를 측정했다고 주장하지 않는다.

### SideEffect

- SAFE normal/retry/duplicate delivery/kill-after-commit+retry **4/4 PASS**.
- UNSAFE retry/delivery/kill committed duplicate **3/3**, ordered/atomic/lost violation **3/3** 검출.
- Observer unavailable 및 미실행 kill fault **INCONCLUSIVE**. Required evidence 제거 시 loader **ERROR**.
- Committed observation confirmation **12/13**; observer-unavailable control 때문에 1개가 불완전하다.
  Requested fault coverage **18/20**; commit 없는 kill+retry control은 완전 실행으로 인정하지 않는다.
- Reproduction **6/6**, local schedule minimization **6/6**.
  Duplicate schedules는 **2 → 2**, relationship/lost schedules는 **1 → 1**이다.
  더 작은 schedule을 만들었다고 과장하지 않는다. 자동 replay CLI는 unavailable이다.

### BlindTest

- 실제 Docker correct 및 filesystem/environment leakage probe **PASS**.
  Mutant A/B와 no-op은 **FAIL**, partial suite/timeout은 **INCONCLUSIVE**,
  isolation을 attest할 수 없는 absent image 및 hidden evidence 제거는 **ERROR**.
- **Mutation adequacy on benchmark corpus: 2/2**. 일반적인 mutation score가 아니다.
- Correct + 두 mutant + no-op의 suite quality receipt와 실제 verified child 결과 **4/4** 일치.
- FAIL의 같은 sealed suite/immutable image 재실행 **3/3**; 자동 replay CLI 및 reducer는 unavailable.
- Target canary leak **0**, 기존 typed Agent projection의 private canary/metadata/path,
  hidden IDs 및 concrete argument/environment value leak **0**.

## 실행 시간·baseline

- Host: **macos-aarch64**, **rustc 1.98.1 (48a229cea 2026-09-01)**, Docker **29.5.2**.
- 정방향 전체 실행 **45.703초**, 역방향 **44.196초**.
  다른 workspace 테스트와 일부 동시 실행한 로컬 관측이며 성능 기준으로 사용하지 않는다.
- 정/역방향 **31/31 unchanged**, 두 full release gate PASS.
- Corpus hash: `sha256:a27f578338c6b24d9c8c337a697b4eb1156f3d864fd1de189922ac40e997b778`.
- Semantic hash: `sha256:fe4f3c2b57f18233c7d910e2d6d2a00eed83fa5f6b2e58d078dc74997ac995cb`.
- [Machine baseline](benchmarks/baseline-v1/result.json),
  [Human report](benchmarks/baseline-v1/report.txt),
  [Reverse snapshot](benchmarks/baseline-v1/reverse-result.json),
  [Comparison](benchmarks/baseline-v1/comparison.json).

Snapshot은 측정 결과이며 expected golden이 아니다. 시간·물리 경로·private nonce·runtime artifact hash는
별도로 기록하고 semantic hash에서는 제외한다. Case/source definition과 실제 verdict/metrics/coverage/
reduction size는 비교에 남긴다. 다른 corpus version은 자동 비교를 거부한다.

## 검증

- `cargo fmt --check`: **PASS**.
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (workspace typecheck 포함).
- `RUST_TEST_THREADS=4 cargo test --workspace`: **375/375 PASS**, failed/ignored **0**.
  기존 **358개 삭제·수정·약화 없음**. 신규 **17개**: harness unit/실행 self-check 14,
  실제 SQLite/Docker/zero-selection integration 3.
- `cargo build --workspace --release`: **PASS**.
- 최종 release 바이너리 전체 benchmark 정방향/역방향 각각 **31/31 PASS**.
  Behavior **9/9**, SideEffect **13/13**, BlindTest **9/9**, 두 full release gate **PASS**.
- Machine JSON schema validation: **PASS**. 네 v1 schema를 `schemas/`에 생성했다.
- 저장된 canonical SHA-256 및 두 snapshot의 31-case inventory/semantic equality 재확인: **PASS**.
  저장 당시 31개 primary result reference가 모두 존재했다. 두 deliberate-corruption control의
  required evidence는 의도적으로 제거되어 원본 artifact를 유효하다고 주장하지 않는다.
- [전체 tests](.b2ige/p8-tests.log), [clippy](.b2ige/p8-clippy.log),
  [release build 및 정/역방향 runner](.b2ige/p8-release-verified.log),
  [최종 comparison](benchmarks/baseline-v1/comparison.json).

JSON은 각 case 및 전체 run에서 생성 schema로 실제 검증한다. Product schema와 evidence는
각 제품 loader에서 다시 확인한다. Missing evidence controls는 intact artifact의 검증/재로드 후
required evidence를 제거하고 거부 여부를 별도로 측정한다. 거부가 발생해야만 ERROR boundary로
기록하며, loader가 잘못 PASS를 반환하면 그대로 false PASS로 집계하는 self-check도 추가했다.

## 필수 review

1. **Missing evidence → PASS?** 누락·중복·0건·부분 결과와 missing measurement는 gate 차단.
   Intentional removal에서 실제 loader 결과를 보존하여 예상 밖 PASS도 감추지 않는다.
2. **Observer 실패 → product FAIL?** 기존 제품 판정 유지. Runner 자체 실패는 null verdict +
   harness error로 기록하고 gate를 막는다. Missing/corrupt store의 ERROR는 verified-load boundary다.
3. **Nondeterminism?** Counter fixture와 기존 stability checker를 실제 실행한다.
   Unstable 영역을 임의 정규화하지 않고 stable divergence를 보존한다. 두 case order 결과가 같다.
4. **Baseline poisoning?** 독립 label과 명시적 reference source 승인만 사용한다.
   Candidate 출력으로 expected label이나 Behavior baseline을 갱신하지 않는다.
5. **Agent secret access?** Raw hidden suite/oracle/canary는 private controller store에만 유지.
   각 Docker target에는 해당 공개 구현과 현재 입력만 전달한다. Same-user host 비보호를 명시한다.
6. **재현?** 모든 known FAIL의 실제 재현 **16/16**과 refs를 남긴다.
   Assignment/schedule local minimality와 BlindTest 일반 reproduction을 구분한다.

## 한계와 다음 단계

- 명시적 **31-case 소규모 corpus**, 2개 known mutants다. Exhaustive proof, 전체 mutation engine,
  arbitrary fuzzing, 경쟁제품 순위, 실제 외부 provider 분포를 대표하지 않는다.
- 정확한 hidden inputs 및 raw evidence는 로컬 private temporary roots에 있다.
  Baseline JSON은 portable evidence bundle이나 인증서가 아니며, OS cleanup 뒤 live reload는 불가능하다.
  Snapshot 자체의 기록과 원래 evidence의 현재 접근 가능성을 혼동하지 않는다.
- 전체 observation coverage는 제품별 서로 다른 관측 단위의 집계다. 자동 replay를 지원하지 않거나
  측정하지 않은 경로는 unavailable/null; 분모가 없는 비율은 N/A다.
- SQLite local provider, Unix process와 Docker isolation 범위만 사용한다. Isolation unavailable
  case는 absent image로 attestation 불가 경로를 실행하며 더 강한 isolation을 주장하지 않는다.
- 웹 UI 변경 없음. 새로운 browser 검증이 필요하거나 수행됐다고 주장하지 않는다.
- **P8 완료 gate를 `state/CURRENT.md`에 기록했다. P9 시작 가능하나 미시작.**
  Commit/push/deploy/publish 모두 미수행.
