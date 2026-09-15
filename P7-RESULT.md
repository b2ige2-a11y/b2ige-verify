# P7 결과 — Agent / MCP / CI Integration

**P7 COMPLETE / LOCAL GATE PASS. P8 시작 가능, 미시작. Commit/push/deploy 미수행.**

## 구현

- **Agent Protocol v1**: versioned request/response/schema, 제품·operation·identity,
  판정·expected/observed·공개 reproduction·evidence refs·scope/coverage/budget·next action.
  기존 authoritative execute → verified load → 기존 AgentReport sanitation → v1 순서.
  별도 판정 계산·evidence 추론 없음. 동일 artifact/operation의 출력 결정성 검증.
- **Skill v1**: 짧은 제품 선택 지침, 네 verdict 처리, 실패 재현/수정/재검증,
  BlindTest secrecy·baseline 자동 변경 금지. Portable bundle + 작은 reference/schema.
  실제 로컬 `codex mcp add --help`로 확인한 adapter 명령 제공. 사용자 Codex 설정은 미변경.
- **MCP**: Rust `verify-mcp` / `b2ige-mcp`, stdio JSON-RPC, 초기화·ping·tools/list/call.
  `b2ige_doctor`, `b2ige_behavior_verify`, `b2ige_sideeffect_verify`,
  `b2ige_blindtest_verify`, `b2ige_report`. Trusted registry와 startup-pinned typed config만
  사용. Agent가 임의 command/args/env/path/store를 주입할 수 없음. Report는 세션의
  opaque alias를 기존 verified loader로 재검증. stdout은 MCP 메시지만 포함.
- **init/doctor**: Rust/Node/Docker/config bounded detection, dry-run, create_new로
  no-overwrite. 자동 source 수정·baseline 생성/승인 없음. Config/identity/approval/
  fixture/SQLite 초기 snapshot/저장소 쓰기/Docker/image/sealed 경계 readiness 확인.
  Doctor는 실행 검증이나 container attestation이 아님. CI guard는 doctor를 거부.
- **CLI**: `behavior verify` 추가. 기존 SideEffect/BlindTest/report와 human/json/agent 유지.
  기존 AgentReport v3 호환을 위해 Protocol v1은 `--output agent --protocol 1`로 명시.
  Machine 초기화 오류도 고정된 구조의 ERROR. 0/1/2/3, argument misuse 64 유지.
- **CI**: GitHub Actions local-first matrix template, Python 표준 라이브러리 wrapper와
  `b2ige ci-check`. 판정·종료 코드·v1 schema shape·verify operation 일치를 검사하며
  PASS만 green. malformed/missing/child crash/doctor 출력은 ERROR. 기본 artifact는
  sanitized report 단일 파일. Hidden human evidence 자동 업로드 없음.

Schema 결정: Agent request/response v1 및 registry v1을 독립 추가.
ReportDocument/AgentReport v3와 이전 schemas, P1–P6 authoritative artifacts는 유지.
Core 추가는 기존 검사를 재사용하는 readiness API 2개뿐이며 실행 판정 의미는 미변경.

## 실제 smoke

| 실행 경로 | 확인한 결과 |
|---|---|
| 실제 CLI Behavior | human/json/legacy agent/protocol v1, PASS/0·FAIL/1 |
| 실제 MCP → Behavior | PASS·FAIL, reproduction/evidence, 반복 report 결정성, evidence 제거 시 ERROR |
| 실제 MCP → SideEffect SQLite | PASS·FAIL·INCONCLUSIVE; 초기 DB 불가 doctor ERROR 및 실행 observer ERROR |
| 실제 MCP → BlindTest Docker | correct PASS, auth mutant FAIL, partial INCONCLUSIVE, missing image ERROR |
| 실제 CI wrapper → SQLite | 0/1/2/3 보존, PASS만 성공 |
| Release CLI → Behavior | 실제 divergence FAIL/1 |
| Release MCP → Behavior | 같은 fixture의 FAIL, 공개 Agent JSON **1,765 bytes** |

실제 Docker images/attestation과 기존 P6 gate를 사용한다. Mock JSON mapping만으로
완료를 판단하지 않았다. CI transport guard의 synthetic mapping 테스트와 실제 실행
테스트는 구분된다. MCP는 verifier subprocess bridge 대신 직접 public API를 호출하므로
child machine-output 검증은 CLI/CI 경계에서 수행한다.

Release [CLI Agent](.b2ige/p7-release-behavior-agent.json),
[MCP Agent](.b2ige/p7-release-mcp-agent.json). 해당 smoke의 temporary raw store는 정리했다.
공개 report는 실행 관측 기록이며 현재 reload 가능한 source store를 보장하지 않는다.

## Gate

- 신규 **19개**: CLI/Agent/CI **9**, MCP **10**. 기존 **339개** 삭제·수정·약화 없음.
- 전체 workspace test: **기존 339 + 신규 19 = 358/358 PASS**, failed/ignored **0**.
- `cargo fmt --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `cargo build --workspace --release`: PASS.
- P7 MCP integration: **10/10 PASS**, 실제 Docker 포함.
- P7 CLI/Agent/CI: **9/9 PASS**.
- GitHub Actions template: **actionlint v1.7.7 PASS**.
- Skill: skill-creator quick_validate **PASS** (검증 전용 temporary PyYAML 환경).
- 웹 UI 변경 없음. 기존 viewer 테스트를 유지; 새로운 브라우저 검증을 했다고 주장하지 않음.

[전체 로그](.b2ige/p7-tests.log) · [MCP 로그](.b2ige/p7-mcp-tests.log) ·
[clippy](.b2ige/p7-clippy.log) · [release build](.b2ige/p7-build.log).

| Bounded gate metric | 결과 |
|---|---:|
| Agent hidden field/value leakage | 0 |
| Agent private canary leakage | 0 |
| INCONCLUSIVE → CI success | 0 |
| ERROR → CI success | 0 |
| MCP arbitrary shell/file/raw-query/export surfaces | 0 |
| 기존 verifier false-PASS regression | 0 |

각 0은 실행한 corpus/negative controls 범위의 관측이다. Exhaustive proof나 P8 benchmark가 아니다.

## 필수 review

1. **Missing evidence → PASS?** 기존 loader를 재사용하고 source 제거 재로드 ERROR,
   malformed output/exit mismatch/unknown alias를 거부한다.
2. **Observer 실패 → product FAIL?** 기존 판정 그대로 유지. SQLite schema/launch/Docker
   오류는 infrastructure ERROR, partial coverage는 기존 INCONCLUSIVE.
3. **Nondeterminism?** 새 timestamp/LLM 해석 없이 같은 verified source의 결정적 projection.
   별도 실행은 기존 timestamp/source hash가 달라질 수 있으며 byte identity를 약속하지 않음.
4. **Baseline poisoning?** init/Skill/MCP가 baseline이나 승인을 생성·수정하지 않음.
   MCP는 운영자가 등록한 configuration/authorization을 시작 시 고정한다.
5. **Agent secret access?** BlindTest는 기존 AgentReport 필터, 공개 숫자 scope metadata만
   사용. raw reader 없음. 동일 host-user 파일 접근을 차단했다고 주장하지 않음.
6. **재현?** 실제 repro/evidence refs와 명시적 replay unavailability 유지. Hidden exact
   inputs는 private evidence에 보존하고 agent에는 공개 steps만 반환한다.

## 제한 / 후속 단계

- MCP v1은 local stdio·serial 실행. HTTP/remote, cancellation/progress, arbitrary
  commands/files, 자동 repair, IDE extension, LLM/paid API/telemetry 없음.
- 실행 budget은 기존 reviewed product config 소유. non-null protocol override는 명시적 ERROR.
- MCP report alias는 서버 세션 수명까지. 재시작 후 historical report는 trusted CLI 사용.
- Registry/approval/server executable은 trusted controller가 관리해야 한다. Behavior/SideEffect
  등록은 로컬 프로그램 실행 권한이며 sandbox가 아니다. 기존 same-user/Docker/kernel 한계 유지.
- init은 최소 빈 registry만 만든다. 실제 제품 fixture/approved baseline/sealed suite는
  운영자가 제공해야 한다. CI template도 해당 입력을 연결한 뒤 활성화한다.
- JSON/human BlindTest 출력은 trusted private view. Agent/CI는 sanitized output만 사용.
- Target-specific repository-local Codex installer를 추측해 추가하지 않았다. Portable skill과
  확인된 MCP adapter를 제공하며, 지원하지 않는 client 자동 설치는 제공하지 않는다.
- **P8 시작 가능. 이번 작업에서 P8을 시작하지 않았으며 commit·push·deploy도 하지 않았다.**
