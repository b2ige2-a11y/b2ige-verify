# SideEffect Proof v1

## 검증 경계

`attempt != committed effect`. P1 process runner가 target을 실행하고 별도 SQLite observer가
**durable append-only committed ledger**를 온라인 백업한다. 요청 로그·stdout·exit code로
commit 수를 추정하지 않는다. 이 ledger라는 선언은 사용자가 검토하는 contract의 신뢰 경계다.
외부 Stripe/paid API/네트워크 proxy/새 runner/LLM/BlindTest는 사용하지 않는다.

`SideEffectContract`는 logical operation ID, idempotency/correlation identity, executable hash,
explicit args/env, fixture content/mode hash, timeout, effect definitions, SQLite observers,
관계형 semantics, ordered fault schedules 및 exploration/reduction budget을 고정한다.
Target cwd는 P2 snapshot 복사를 재사용해 schedule마다 새로 만든다. 한 schedule 내에는 같은
cwd/input으로 순차 실행한다. 초기 ledger에 같은 operation의 commit이 있으면 ERROR다.
Fixture 원본이나 executable hash가 달라지면 실행을 거부하며 baseline을 갱신하지 않는다.

## SQLite observer

각 observer는 fixture 기준 상대 DB path, 실제 table, external ID/idempotency/correlation/
operation discriminator/commit order column을 명시한다. 다섯 column은 서로 달라야 한다.
단일 provider/operation/external ID가 commit identity이며 attempt ID로 합치지 않는다.
SQLite online backup은 WAL에 commit된 페이지도 포함한다. 로드 시 백업 bytes를 임시 DB로
열어 integrity/schema/row identity를 다시 확인한다. Observer는 원본 DB에 쓰지 않는다.

- 표는 append-only ledger여야 한다. 후속 완전한 관측에서 기존 row가 없어지거나 바뀌면 ERROR.
- external ID는 operation 안에서 유일해야 한다. 같은 row의 여러 snapshot 관측은 한 commit.
- sequence는 table 안에서 유일하고 양수인 증가 정수이며 JS-safe integer 범위를 지킨다.
- `ordered_after`는 같은 DB/table/sequence domain만 허용한다. wall-clock은 판정에 쓰지 않는다.
- 각 관측 snapshot은 일관된 committed state다. 실행 중 모든 DB transition을 포착한다는 주장은 없다.
- P5 제한: DB backup 8 MiB, matching rows 4096, observer 16, effect/schedule/action 각각 64,
  attempt budget 4096, reduction budget 128, attempt timeout 60초. Snapshot busy는 bounded retry
  후 unavailable. Missing DB/권한/busy는 INCONCLUSIVE, schema/data/corruption은 ERROR.
- Detached writer, hostile target, 외부 mutable provider 및 append-only를 어기는 transient
  create/delete state는 지원하지 않는다. Secret isolation이나 hash authentication은 없다.

온라인 백업 구현은 [rusqlite Backup API](https://docs.rs/rusqlite/0.40.2/rusqlite/backup/struct.Backup.html)를 사용한다.

## Fault schedule

모든 primitive는 정확히 하나의 process attempt를 나타낸다.

| Primitive | 실행 및 입증 조건 |
|---|---|
| NONE | 정상 시작·완료 |
| RETRY | 앞선 attempt 이후 같은 input/cwd/operation 재시도 |
| DUPLICATE_DELIVERY | 앞선 attempt 이후 동일 input을 다시 전달; P5는 순차 delivery |
| KILL_AFTER_COMMIT | 첫 attempt의 커밋을 authoritative snapshot에서 관측한 후 process group 종료 |

KILL_AFTER_COMMIT은 **첫 action이며 바로 다음 RETRY가 있어야 한다**. 이전 attempt의 commit을
새 attempt의 commit으로 오인하지 않도록 v1에서 이 조합만 허용한다. 예: `[KILL_AFTER_COMMIT, RETRY]`.
다른 kill 배치, standalone RETRY, unsupported semantics는 ERROR다. NONE 이후 RETRY를 여러 번
추가할 수 있다. 성공 증거는 termination requested + 실제 signal delivery + pre-kill committed snapshot + 실제 SIGKILL
종료 + retry started/completed다. Commit을 못 봤거나 target이 먼저 종료되면 fault complete=false.
Timeout/signal 종료도 history에는 target_killed로 남지만 scheduled fault 실행으로 세지 않는다. Process completion이 이 local trigger의 응답 경계다. Target non-zero exit는 verifier ERROR가 아니다.

각 schedule은 별도 초기화된 experiment이며 append-only logical history를 가진다. Event ID는
전체 결과에서 유일하고 order는 schedule 내부의 인과 순서다. History는 operation/attempt start,
commit 관측, completion/kill, retry/duplicate delivery, observer failure, completion을 포함한다.
최종 count는 history event 수가 아닌 모든 검증된 snapshot의 unique committed identities다.

## Checker

| Semantics | 충족 조건 / 위반 |
|---|---|
| exactly_once | committed=1; 0 또는 >1 위반 |
| at_most_once | committed≤1; >1 위반 |
| at_least_once | committed≥1; 완전한 관측에서 0 위반 |
| never | committed=0; >0 위반 |
| ordered_after(A,B) | B 이후 A; A가 B보다 앞서거나 완전한 관측에서 B 없이 A가 있으면 위반 |
| atomic_with(A,B) | 완료 상태에서 둘 다 존재하거나 둘 다 없음; 한쪽만 있으면 위반 |

Atomic은 완료 상태의 존재 동시성을 검사하며 transaction-level atomicity나 중간 상태의
불가시성을 증명하지 않는다. Absence/half-commit은 필요한 observer와 schedule이 완료되어야
입증한다. 이미 관측된 duplicate/forbidden/order violation은 충분한 evidence로 FAIL이 가능하다.
Infrastructure ERROR가 우선하고, 입증된 FAIL, INCONCLUSIVE, PASS 순서로 판정한다.
PASS는 전체 required schedules/actions, 초기/최종 observer, history가 완전할 때만 가능하다.
Budget 소진, 미실행 retry/kill, unknown commit은 PASS가 될 수 없다.

## Artifact와 재검증

P1 EvidenceStore의 atomic/non-overwrite commit을 재사용한다. Parent에는 contract, initial
snapshots, history, child references, reduction trials를 따로 Evidence로 저장한다. Child에는
identity/contract hash/process input/runtime와 SQLite backups가 각각 Evidence로 저장된다.
Load는 parent/child/evidence/run binding과 hash, required action 수, 실제 SQLite contents,
append-only rows, history 및 executed faults를 검증하고 checker를 다시 실행한다. Result의
verdict/violation/reason은 cache일 뿐이다. Verdict를 바꾸고 manifest를 재해시해도 다른 검증
판정을 만들 수 없다. Hash를 전부 조작하는 호스트 공격에 대한 인증 증거는 제공하지 않는다.
원본 source fixture가 없어져도 저장된 evidence 검증은 가능하지만 새 reproduction 실행은
원래 executable/fixture hash가 맞아야 한다.

## Counterexample

FAIL의 첫 violation signature(effect IDs + violation kind)를 원본 schedule로 실제 재실행한다.
보존되면 action 삭제와 kill 제거를 시도하고 매번 새 fixture/process/SQLite evidence로 다시
검사한다. 첫 action 삭제로 retry/delivery가 선두가 되면 NONE으로 정규화한다. 유효한 nonempty
schedule에 대한 deterministic single-action deletion neighborhood가 모두 소진된 경우에만
`LOCALLY_MINIMIZED`. 이는 전역 최소성/전체 fault 공간의 증명이 아니다. Budget 소진은
`BUDGET_EXHAUSTED`; 불안정하여 원본도 보존하지 못하면 `UNREPRODUCED`. 원본은 항상 유지한다.
재실행 artifact가 없으면 unavailable 이유를 기록한다. 자동 replay CLI는 제공하지 않는다.
P3 assignment reducer는 변경하지 않는다.

## CLI / corpus

```sh
cargo build --workspace --release
cargo run -p verify-core --example sideeffect_corpus -- \
  /absolute/new-corpus-directory /absolute/repo/target/release/p5-effect-fixture

target/release/b2ige sideeffect verify /absolute/new-corpus-directory/S3.contract.json \
  --store .b2ige/runs
target/release/b2ige report <result-id> --store .b2ige/runs
target/release/b2ige report <result-id> --store .b2ige/runs --output agent
target/release/b2ige report <result-id> --store .b2ige/runs --open
```

Corpus generator는 빈 source fixture, 9개 contract, SAFE/UNSAFE/negative 실제 결과와
`summary.json`을 새 directory에 저장한다. 기존 directory를 덮어쓰지 않는다.
Exit code 0 PASS / 1 FAIL / 2 INCONCLUSIVE / 3 ERROR. Argument misuse는 기존 64 유지.
SideEffect report는 Behavior authorization 파일이 필요 없다. `--open`은 localhost capability URL의
read-only foreground viewer로, 탐색마다 root 및 child를 다시 검증한다.

Human/UI는 verdict → violated contract → expected/observed → 실제 reproduction → evidence /
timeline / runs / raw 순서다. Agent는 counts, expectations, effect IDs, reproduction ref/status와
최대 8개 committed evidence refs만 노출한다. DB path/env/external identity/full history는 제외한다.
Report source JSON은 authoritative import로 사용할 수 없다.

## Schema 결정

신규 SideEffectContract/History/ExperimentResult/FaultScheduleResult는 독립 v1이다.
P0/P1/P2/P3 artifact와 checker semantics는 그대로다. Runner의 controlled observation은 새 타입으로
분리하며 기존 ProcessObservation wire fields는 바꾸지 않는다. P4 ReportDocument/AgentReport 및
projection은 **v2**로 올린다: SideEffect kind/details와 product-neutral expected/observed 추가.
기존 v1 schema는 `schemas/report-document.v1.schema.json`, `agent-report.v1.schema.json`에 보존한다.
현재 `*.schema.json` 및 generator는 v2 export를 가리킨다. Behavior export도 v2이며 verdict는
기존 verified loader가 그대로 결정한다.
