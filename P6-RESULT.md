# P6 결과 — BlindTest MVP

**P6 COMPLETE / LOCAL GATE PASS. P7 시작 가능, 미시작. Commit/push/deploy 미수행.**

## 구현 범위

- P6 전체를 한 단계로 구현: Requirement, approved executable invariant binding,
  sealed hidden suite/manifest/cases/oracle, Docker runtime/inspect attestation,
  authoritative checker/verified loader, suite quality receipt, Human/Agent/CLI/UI.
- Predicate: `exit_equals`, `exit_not_equals`, `stdout_equals`, `stderr_equals`,
  `stdout_not_contains`, `stderr_not_contains`. 최대 16개 AND, byte-exact 비교.
  Free text/LLM은 verdict predicate가 아니다. Candidate/reviewed 판정 근거 거부.
- 승인된 requirement hash + invariant artifact hash + fixed executable checker binding
  일치 필수. Suite approval/provenance와 전체 case hash inventory도 필수.
- Sealed root와 원시 evidence store는 agent workspace 밖. Canonical path overlap,
  symlink 및 suite hard-link alias 거부. 현재 case 입력만 target에 전달.
- Linux image는 immutable SHA-256 identity로 resolve 후 실행. P1 bounded process
  runner/store 재사용. Host checker가 complete stdout/stderr/exit를 판정한다.
- 실제 Docker inspect로 network none, mount 없음, non-root, read-only, tmpfs,
  cap-drop ALL, no-new-privileges, namespace/device/resource 설정과 lifecycle 검증.
  Timeout/capture overflow/OOM/부분 실행은 PASS가 될 수 없다.
- 저장된 verdict/violation/coverage/leakage/quality를 재계산한다. 결과만 PASS로 바꾸고
  재해시해도 실제 FAIL 유지. Missing source evidence 및 unsafe inspect 재해시 거부.
- Agent는 공개 semantics/안전한 observed category/reproduction/opaque evidence refs만
  제공. Human local view는 hidden case/input/oracle/raw evidence를 별도로 펼칠 수 있다.
  두 view의 authoritative verdict는 같다. Agent output의 human viewer 열기는 거부.
- `blindtest doctor`, `verify`, `validate-suite`, 기존 `report`, `--output agent`,
  trusted `--open` 통합. Exit 0/1/2/3, argument misuse 64 유지.
- P0 invariant v1 **byte-for-byte 보존**, executable invariant v2 및 독립 v1 artifact
  schemas 추가. Report/Agent/projection v3; 이전 v1/v2 schemas 보존. P1–P5 semantics 불변.

## 실제 Docker corpus / release CLI

| Case | 실제 판정 / exit |
|---|---|
| Correct | PASS / 0 |
| Mutant A: expiry check 제거 | FAIL / 1 |
| Mutant B: expiry condition 반전 | FAIL / 1 |
| No-op | FAIL / 1 |
| Filesystem/env/mount/cwd/proc 탐색 target | PASS / 0, private canary 미획득 |
| Partial hidden suite | INCONCLUSIVE / 2 |
| Target timeout | INCONCLUSIVE / 2 |
| Output capture overflow | INCONCLUSIVE / 2 |
| Image unavailable | ERROR / 3 |
| Docker unavailable | ERROR / 3 |
| Hidden suite unavailable / corruption | ERROR / 3 |
| Required isolation unavailable | ERROR / 3 |
| 실제 저장된 hidden evidence 파일 누락 | ERROR / 3 |

각 corpus target은 별도 실제 Docker image로 실행했다. Correct/mutants/no-op/probe는
각 3개 case를 완수했다. Self-validation receipt는 correct/두 mutants/no-op/probe/
verifier-unavailable의 **6개 actual child run**을 검증하고 모두 expected outcome과 일치했다.
일반 suite는 reference solution 없이 PASS 가능하며 receipt 미제공 시
**self-validation not supplied**를 명시한다. Bounded corpus 이상의 품질 주장은 없다.

실제 corpus와 private store:
private local corpus (not distributed).
[Release CLI gate summary](.b2ige/p6-cli-summary.json),
private local quality receipt (not distributed).
Raw artifacts는 trusted controller용이며 agent workspace로 복사하지 않는다.

## 검증 gate

- **기존 273 + 신규 66 = 339/339 PASS**, failed/ignored **0**.
  신규 core **54**, CLI/report **12**. 기존 test 삭제/완화 없음.
- `cargo fmt --check` **PASS**.
- `cargo clippy --workspace --all-targets -- -D warnings` **PASS**.
- `RUST_TEST_THREADS=4 cargo test --workspace` **PASS**.
- `cargo build --workspace --release` **PASS**.
- 실제 Docker integration 및 release CLI/report tests **PASS**.
- agent-browser: 1280px desktop / 390px mobile, Evidence/Isolation/Hidden details/Raw
  펼치기, Source report 이동, PASS/FAIL/INCONCLUSIVE/ERROR 표시 확인.
  Console error **0**, horizontal overflow **0**, collapsed/expanded axe violation **0**.
  Viewer는 read-only이며 입력 form 없음.
- [전체 test log](.b2ige/p6-tests.log), [Desktop](.b2ige/p6-desktop.png),
  [Mobile](.b2ige/p6-mobile.png), [PASS](.b2ige/p6-pass.png),
  [INCONCLUSIVE mobile](.b2ige/p6-inconclusive-mobile.png).

| Bounded gate metric | 결과 |
|---|---:|
| Correct false FAIL | 0 |
| Known mutant false PASS | 0 |
| No-op false PASS | 0 |
| Missing hidden evidence → PASS | 0 |
| Partial hidden suite → PASS | 0 |
| Isolation not attested → PASS | 0 |
| Agent leakage canary count | 0 |

Attack tests는 case/evidence 누락, oracle missing/corrupt, candidate/reviewed,
approval/checker mismatch, suite hash mismatch, workspace/symlink/hard-link,
sealed/socket mount, network/root/privilege/capability/security/resource settings,
missing inspect fields, actual probing, malicious public labels, partial/timeout/overflow,
Docker unavailable, verdict-only rehash, Human/Agent verdict 일치를 포함한다.
위 수치는 알려진 corpus와 실행한 negative controls 범위이며 exhaustive proof가 아니다.

## Verifier review

1. **Missing evidence → PASS?** 전체 inventory와 source evidence 필수; 부분 실행은
   INCONCLUSIVE, corruption/setup failure는 ERROR. Nullable Docker 값과 필드 누락을 구분한다.
2. **Observer failure → product failure?** Complete capture/target exit가 있어야 predicate
   FAIL. Timeout/overflow/OOM은 coverage 부족; Docker init/isolation failure는 ERROR.
3. **Nondeterminism → false divergence?** Byte predicates와 입력으로 전달한 logical time만
   비교한다. Wall-clock timestamp나 run 간 byte divergence를 판정에 사용하지 않는다.
4. **Baseline poisoning?** Behavior baseline을 변경하지 않는다. Private suite approval/hash,
   invariant/checker binding 및 public source snapshot/immutable image를 기록·검증한다.
5. **Agent secret access?** Target에 host mount/network/socket 없음. Agent projection에
   원시 output/inventory/path 없음. 같은 host-user의 접근 권한을 막았다고 주장하지 않는다.
6. **Failure reproduction?** Exact case args/env/fixture/oracle, immutable image, capture와
   isolation evidence 보존. 공개 Reproduction 제공; automatic replay CLI는 명시적으로
   `replayability: unavailable`이며 이유를 기록한다. Minimality 주장 없음.

## 플랫폼과 남은 제한

macOS arm64 + Docker Desktop **4.76.0**, Engine **29.5.2**, Linux/arm64 containers.
검증한 수준은 **DOCKER_ISOLATION**이며 **HARDENED_LINUX가 아니다**.
같은 host user/admin/root, Docker VM/daemon compromise, kernel/runtime exploit,
side-channel, mathematically perfect secrecy는 범위 밖이다. Workspace/build identity는
기록하지만 reproducible-build 증명은 아니다. Hash는 wholesale host evidence rewrite의
인증 수단이 아니다. Mandatory paid API **NONE**.

[BLINDTEST.md](docs/BLINDTEST.md) · [THREAT-MODEL.md](docs/THREAT-MODEL.md) ·
[P6-INDEX.md](tasks/P6-INDEX.md). **P7 구현 및 commit/push/deploy는 수행하지 않았다.**
