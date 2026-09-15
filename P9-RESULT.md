# P9 결과 — PUBLIC RELEASE PREPARATION

> Historical result. Current publication decisions supersede open items below: [final readiness sweep](PUBLICATION-READINESS-RESULT.md).

**P9 완료. LOCAL RELEASE CANDIDATE READY = true. PUBLICATION READY = false.**

## 공개 표면과 패키징

- Root README를 세 제품의 질문, 실제 실행 예, 범위가 명시된 benchmark로 구성했다.
- BlindTest / Behavior(가칭) / SideEffect Proof별 README, config 생성기, quickstart,
  실제 PASS/FAIL 예제와 demo scripts를 제공한다. Core는 공유 내부 기술로 유지한다.
- Version 0.1.0, changelog의 candidate 표기는 0.1.0-rc. 기존 product/schema 숫자는 유지;
  release manifest만 독립 v1. `--version`을 두 공개 실행 파일에 추가했다.
- macOS arm64 native archive, full-workspace source archive, SHA256SUMS, manifest,
  locked dependency inventory 및 npm thin-wrapper tarball을 로컬 생성했다.
- macOS Intel / Linux x86_64 native CI 설정. 해당 플랫폼 실행은 아직 검증하지 않았다.
  Linux arm64는 보류, Windows는 미지원. SBOM/서명/공증은 완료했다고 주장하지 않는다.
- Cargo path install은 전체 workspace 기준. registry `.crate` publish는 차단 상태다.
  npm launcher는 platform/args/stdio/exit/signal을 전달하며 검증 로직·자동 다운로드가 없다.

## 좁은 P8 hardening 및 계약 보존

- compare가 benchmark/schema/corpus 불일치를 거부한다.
- semantic_equal은 저장 hash를 신뢰하지 않고 canonical semantic content에서 재계산한다.
- 회귀 테스트 1개 추가. 기존 테스트 삭제/약화와 P1–P8 verification semantics 변경 없음.
- P8 public snapshot에서 비의미적 로컬 evidence/reproduction 경로만 제거하고 canonical
  content checksum을 다시 계산했다. Labels/metrics/corpus 및 semantic hash는 동일하다.
- 필수 review: 누락 evidence→PASS 경로 추가 없음; observer 실패를 제품 FAIL로 변환하지 않음;
  nondeterminism 처리/승인 baseline/hidden isolation 변경 없음; 기존 FAIL 재현 계약 유지.

## 실행 검증

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `RUST_TEST_THREADS=4 cargo test --workspace`, `cargo build --workspace --release` 통과.
  전체 **376/376 PASS**, failed/ignored **0**. 기존 375개 유지 + 신규 1개.

- 실제 examples: Behavior PASS/FAIL, SQLite safe PASS/unsafe retry FAIL.
- BlindTest: 두 구현의 coding-agent-visible valid credential test PASS 후 correct PASS,
  buggy FAIL, probe PASS. Fresh sealed store 사용; 검사한 Agent private-value leakage 0.
- Native archive를 새 임시 디렉터리에 설치해 help/version, init dry-run/init, empty 및
  registered doctor, 세 제품, report human/json/agent, BlindTest doctor, MCP initialize/
  tools/list/basic verify, full 31-case benchmark 통과. 개발 source 대신 설치된 binaries 사용.
- 별도 full source archive에서 CLI/MCP/benchmark helper `cargo install --path` 성공.
- P8 release gate: 정방향 **31/31 PASS (56.608초)**, 역방향 **31/31 PASS (45.199초)**.
  P8 baseline 및 실행 순서 비교 **31/31 unchanged**; known bugs/reproduction **16/16**,
  false PASS/FAIL **0/0**, hidden/Agent leakage **0/0**, known benchmark mutants **2/2**.
  작은 bounded corpus 결과이며 exhaustive correctness proof가 아니다.
- npm wrapper **12/12** 테스트 통과; npm pack 후 실제 native binary 실행 확인.
- README benchmark snippet 자동 검증, Python syntax, GitHub YAML syntax, 공개 파일/압축 해제
  hygiene, archive checksum/binary hash 및 release manifest JSON schema 검증 통과.
- 초기 무거운 빌드/benchmark 동시 실행 중 기존 Behavior 테스트 4개가 불완전 관측으로 실패했다.
  테스트/timeout/판정은 수정하지 않았으며 단독 전체 실행에서 **376/376** 통과했다.
  실행 부하에 민감한 bounded timing 테스트가 있으므로 무거운 gate는 순차 실행한다.

## CI와 위생

PR checks 및 수동 candidate workflows 준비. 실제 Docker 테스트, native archive 및 fresh
smoke를 포함한다. 원격 CI는 실행하지 않았다. publish/release step과 write 권한은 없다.
런타임 store/숨겨진 evidence는 업로드하지 않는다.

기존 문서의 사용자 절대 경로를 상대 링크/비공개 표기로 교체했다. `.DS_Store`, `.b2ige`,
`target`, Python cache, local npm binary, 환경 파일을 배포에서 제외한다. 바이너리에는
build path remapping을 적용했다. 의도적인 synthetic fixture source만 남긴다.
현재 파일/배포물 검사는 Git history 전체의 비밀정보 검사를 대체하지 않는다.

## 남은 공개 blockers

라이선스 사업 결정(근거 없는 MIT manifest 표기 제거), 전체 이름 collision check와 Behavior
최종 이름, 실제 공개 승인, 타 플랫폼 CI, 보안 신고 채널/지원 정책, 의존성 라이선스 검토,
서명/공증/인증된 배포 경로, 실제 npm hosting 전략. [Checklist](release/checklist.md).

**Public push / GitHub Release / crates.io / npm / PyPI publish / deploy 모두 미수행.**
Commit도 수행하지 않았다. P9 이후 Phase는 시작하지 않는다.

## 산출물과 최종 상태

- [Native macOS arm64 archive](release/artifacts/b2ige-0.1.0-aarch64-apple-darwin.tar.gz)
- [전체 source archive](release/artifacts/b2ige-0.1.0-source.tar.gz)
- [npm skeleton archive](release/artifacts/b2ige-0.1.0.tgz)
- [Checksums](release/artifacts/SHA256SUMS) · [Manifest](release/release-manifest.json)
- [원본 전체 test log](.b2ige/p9-tests-isolated.log) · [Fresh smoke](.b2ige/p9-fresh.log)

로컬 로그와 runtime evidence는 배포하지 않는다. Manifest는 base commit와 dirty 상태 및
source inventory hash를 함께 기록한다. 깨끗한 새 commit에서 빌드했다고 주장하지 않는다.

**LOCAL RELEASE CANDIDATE READY = true (검증한 macOS arm64 범위).**
**PUBLICATION READY = false.** P9 이후 작업 및 실제 publication은 수행하지 않았다.
