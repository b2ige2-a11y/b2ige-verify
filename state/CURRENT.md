# Current State

Date: 2026-09-15
Phase: P9 RELEASE CANDIDATE PREP (gate preserved; no next phase started)
Status: PUBLICATION FINALIZATION COMPLETE — OWNER / EXTERNAL GATES REMAIN

CLEAN_PUBLIC_REPO_READY = true
LOCAL_RELEASE_CANDIDATE_READY = true (macOS arm64 measured scope)
TECHNICAL_PUBLICATION_READY = false
PUBLICATION_READY = false

- 대상은 B2IGE-Verify-Public만이다. Private repository는 수정하지 않았다.
- P1–P8 verifier 소스, evidence schemas, Cargo.lock, benchmark baseline 보존 확인.
- fmt/clippy/release build, Rust 376 tests, npm 28 tests 통과.
- 공식 cargo-cyclonedx 0.5.9로 CycloneDX 1.5 SBOM 6개 생성: 143개 locked packages
  전체 포함. 공식 schema, dependency references, registry checksum 대조 통과.
- README/quickstart, fresh installed Docker/제품/report/MCP/npm smoke, archive 및
  source inventory/checksum/hygiene 검증 통과. 별도 전체 benchmark 재실행 없음.
- Public HEAD는 unborn, commits 0. Git capture tree/blob은 history commit이 아니다.
  기존 private history blocker는 clean public strategy로 이 candidate에서 해결됐다.
- Release manifest v3: git_commit=null, 플랫폼 status/scope 및 clean/technical 상태.
  npm native-manifest v2: 향후 실제 GitHub repository + archive/binary pins. Verifier schema 변경 없음.
- npm downloader 및 fail-closed integrity/version/안전 extraction/cache 구현·검증.
  실제 release host/pins는 미설정이며 private/prepublish guard 유지.
- macOS arm64 VERIFIED_NATIVE. Linux x86_64 / macOS x86_64 NOT_RUN:
  REMOTE EXECUTION REQUIRED. Docker arm64 VM이나 Rosetta를 native x86 검증으로 표기하지 않았다.
- 남은 항목: BehaviorSeal 승인, GitHub owner/repo, @b2ige scope, signing/notarization
  선택, Private Vulnerability Reporting, remote native CI 및 실제 hosted delivery, 최종 공개 승인.
- commit/push/publish/release/deploy/account 설정/인증서 생성/history rewrite 미수행.

[Final decisions](../FINAL-PUBLICATION-DECISIONS.md) · [Checklist](../release/checklist.md) ·
[Readiness](../PUBLIC-REPO-READINESS.md) · [Manifest](../release/release-manifest.json).
