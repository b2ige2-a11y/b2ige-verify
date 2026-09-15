# P4 결과 — Evidence / Result Experience

- **완성 묶음:** verified artifact loading/navigation, Unified Report Model,
  Human terminal, compact Agent JSON, report CLI, local-only UI 완료.
- **CLI:** `target/release/b2ige report <id|store/id/result.json>` +
  `--output json`, `--output agent`, `--open`. `--store` 및 `--authorization` 지원.
  원래 trusted authorization을 사용하며 verdict exit code 0/1/2/3 유지.
- **Report model:** ReportDocument/schema v1, projection v1, AgentReport/schema v1.
  P2 exact/P3A stability/P3B suite/P3C reduction의 verified load/recompute를 사용.
  source hash 기록, export 재입력 금지, source/child 검증 실패 시 정상 report 금지.
- **Human/Agent:** verdict → 핵심 결과 → expected/observed → reproduction → coverage.
  Agent는 명시적 reference allowlist로 raw/path/environment를 제외한다.
  Reducer verdict는 final comparison에서 가져오며 축소 status와 분리한다.
- **Local UI:** Rust에 포함한 HTML/CSS, localhost capability URL. Primary failure,
  collapsed others/evidence/runs/coverage/limitations/raw, verified child navigation.
  MINIMIZED만 locally minimized로 표시하며 BUDGET_EXHAUSTED를 명시한다.
- **검증:** 신규 **22**, 기존 **206** 보존, 전체 **228/228 PASS**, failed/ignored 0.
  fmt/clippy/workspace tests/release build PASS. 최종 UI 수정 후 22개 재검증 PASS.
  별도 JS frontend 없음: Rust compiler/clippy/release build가 typecheck/lint/build.
  agent-browser 실제 --open smoke, desktop 1280px/mobile 390px, 탐색·접기/펼치기,
  console error 0, overflow 0, expanded evidence/raw 접근성 위반 0 확인.
- **남은 제한:** trusted local Unix, hash 비인증, paired replay unavailable,
  expected/observed bytes는 원본 hash/length로 표시. Human inputs는 local에 노출되며
  Agent는 refs 위주. Viewer는 탐색 시 재검증하는 snapshot이며 live monitor가 아니다.
  `--open`은 Ctrl-C까지 foreground 실행(OS interrupt exit); 자동화 exit code는
  --open 없는 명령을 사용한다. Suite에서 reducer 자동 검색은 하지 않으며 알려진
  reduction ID를 열어 탐색한다. BlindTest secrecy는 구현하지 않는다.
- **P5:** **P4 LOCAL GATE PASS**, P5 시작 가능. **P5 미시작**, commit/push/deploy 미수행.
