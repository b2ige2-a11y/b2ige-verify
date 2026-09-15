> Historical pre-copy sweep. Current public-candidate state and resolved blockers are in
> [FINAL-PUBLICATION-DECISIONS.md](FINAL-PUBLICATION-DECISIONS.md).

# B2IGE Verify 0.1.0 — Publication Readiness Result

**LOCAL_RELEASE_CANDIDATE_READY=true — 검증한 macOS arm64 범위.**
**PUBLICATION_READY=false.** 기술·패키지 blocker와 owner 공개 승인 부재.

## 닫힌 항목

- 전체 Git 객체/history scan 및 finding 분류, 공개 문서 claim audit 완료.
- Native/source/npm staging·추출 검사, tar 계정 metadata 제거, 라이선스 파일 포함,
  checksum/binary/source hash 검증을 보강했다.
- Cargo.lock registry 137개 라이선스 대안과 고지문 정리. 누락된 upstream MIT 고지문
  3건을 해당 고정 commit에서 확보했다. SQLite 및 Rust library 고지문도 배포에 포함한다.
- npm은 신뢰한 SHA-256을 확인한 뒤 기존 CLI의 정확한 버전을 검사하고 실행한다.
  stdio/exit/signal 전달과 offline 실패 동작을 유지하고 상대경로/PATH 대체 실행도 차단했다. 가짜 download URL은 없다.
- 보안 신고 절차·supported-version·best-effort/no-SLA 정책, 서명/provenance 전략,
  read-only cross-platform CI, release manifest v2와 체크리스트를 정리했다.

## 검사 결과와 범위

| 항목 | 결과 |
|---|---|
| History | 24 commits + 388 blobs + 251 trees = 663 objects, 3,105,855 bytes, 생략 0. 241 matches: BLOCKER 224 / REVIEW 3 / SAFE_SYNTHETIC 8 / FALSE_POSITIVE 6 |
| 실제 secret | 조사한 패턴에서 API credential/private key/env 내용/runtime private canary 발견 0. 미지·인코딩된 secret 부재를 보장하지 않음 |
| History hygiene | 과거 6개 blob에 개인 홈/worktree 또는 실제 runtime 임시 경로 잔존. 현재 파일 삭제만으로 해결되지 않음. Finder metadata 3개 blob도 REVIEW |
| Artifact hygiene | Native/source/npm 각각 새 임시 디렉터리에서 추출·검사 통과. 5개 native binary strings에서 검사한 home/Codex/temp/private-canary 패턴 0. 공개 synthetic schema/fixture와 실제 sealed runtime을 구분 |
| License | 137개 metadata/고지문 정합성 및 원본 crate archive SHA-256 확인. 필수 GPL/AGPL/MPL 없음; r-efi는 MIT 선택. Cargo 6개/npm Apache-2.0 및 publish 차단 유지 |
| SBOM | 설치된 npm 11.16.0으로 wrapper-only CycloneDX 1.5 생성·포함. Rust/native 전체 SBOM은 **NOT YET GENERATED** |
| macOS arm64 | fmt/clippy 통과, release build 통과. 새 비-Docker CI 경로 **308 tests PASS**, failed/ignored 0. 전체 test target compile 완료. 기존 P9 376-test 전체 실행은 재실행하지 않음 |
| Fresh native | CLI/init/doctor, Behavior·SQLite·BlindTest PASS/FAIL 사례, reports, 실제 Docker, MCP, B2IGE Verify Bench v1 **31 explicit cases** 전체 gate 통과. benchmark corpus에서 false PASS/FAIL 및 관찰 leakage 0 |
| npm/metadata | wrapper **19/19**, packaging negative tests **3/3**, 실제 npm tarball/native launch 및 변조 거부, manifest schema·artifact hashes·license consistency·Python/YAML syntax 통과 |
| macOS Intel | x86_64 Mach-O release cross-build 및 Rosetta CLI/MCP version, Behavior·SQLite 예제 및 subset benchmark **22 cases(9+13)** 실행 통과. 전체 release gate로 간주하지 않음. 실제 Intel host의 native 실행 완료로 분류하지 않음 |
| Linux x86_64 | native runner 실행 미수행. 로컬 Docker는 Linux aarch64. Linux x86_64/Intel 모두 **REMOTE PLATFORM EXECUTION REQUIRED** |
| CI | PR/manual에 pinned actions/compiler, locked build, 플랫폼별 tests/fresh archive smoke. Docker/full benchmark는 Linux 전용. YAML 파싱/구조 검증 통과; 원격 GitHub 실행 증거는 없음 |
| Signing | 유효한 Apple signing identity 0. 현재 Mach-O는 linker ad-hoc signature이며 publisher 서명 아님. notarization/cryptographic provenance 미생성 |
| Schema/semantics | release manifest만 v1→v2 결정. P1–P8 Rust verifier 소스, Cargo.lock, baseline result JSON 및 판정 의미 변경 없음 |

## 남은 공개 blockers

1. 기존 history의 로컬 경로 제거 방식 승인·처리·재검사. 이번 sweep에서 rewrite하지 않았다.
2. **NAME CHECK INCOMPLETE**: npm `blindtest`는 AI 도구와 충돌한다. npm scope 조회 403,
   GitHub namespace/registry 소유·예약 여부는 미확인이다. B2IGE/B2IGE Verify/BehaviorSeal/
   SideEffect Proof는 조사 범위에서 CLEAR ENOUGH FOR RC이며 법적 clearance가 아니다.
3. Linux x86_64/macOS Intel native runner 실행. Git remote가 없으므로 CI dispatch하지 않았다.
4. 실제 private security reporting channel 구성·동작 확인.
5. Rust/native 전체 표준 SBOM 생성. npm-only SBOM으로 대체하지 않는다.
6. 실제 npm binary hosting 또는 platform packages와 해당 delivery 경로의 end-to-end 검증.

## 사람이 결정할 항목

- BlindTest 충돌 대응, Behavior 최종명(BehaviorSeal은 후보), package/repository namespace.
- History 정리 또는 별도 clean public repository 전략. filter-repo/BFG/rebase 미실행.
- Apple 서명·공증 적용 또는 unsigned 첫 배포의 명시적 결정, 실제 보안 신고 채널.
- 최종 snapshot 검토·commit/tag 및 **명시적 공개 승인**. 이 sweep이 승인을 부여하지 않는다.

[Checklist](release/checklist.md) · [Hygiene/findings](release/public-hygiene-review.md) ·
[Names](release/name-clearance-review.md) · [Licenses](release/dependency-license-review.md) ·
[Provenance](docs/RELEASE-PROVENANCE.md) · [Manifest](release/release-manifest.json).

**Git history rewrite / commit / push / public 전환 / GitHub Release / registry publish /
deploy / 인증서 생성·구매 / 계정 설정 변경 모두 미수행.** Base revision은
`83db19326e830b733e6a83393996e7961442bb83`이며 수정 사항은 uncommitted 상태다.
기술·legal/package blocker 0과 owner authorization을 모두 충족하지 못했으므로 공개하지 않는다.
