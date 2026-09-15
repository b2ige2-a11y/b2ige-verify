# P5 — SideEffect Proof MVP

P4 LOCAL GATE PASS를 선행 gate로 사용한다. P5 전체를 하나의 gate로 완료하며 P6는 시작하지 않는다.

## 범위

- [x] SideEffectContract / History / ExperimentResult / FaultScheduleResult v1
- [x] AttemptIdentity와 CommittedEffectIdentity 분리, external ID 중복 관측 합치기
- [x] SQLite durable ledger 온라인 백업 관측, 로드 시 SQLite 재조회
- [x] P1 process runner 재사용, NONE / RETRY / DUPLICATE_DELIVERY / KILL_AFTER_COMMIT
- [x] exactly_once / at_most_once / at_least_once / never / ordered_after / atomic_with
- [x] 실제 child 실행·fault·history 검증, evidence 부족 시 PASS 차단
- [x] 실제 재실행 기반 schedule 삭제 reducer, budget/minimality 구분
- [x] P4 Human / Agent / CLI / local UI 통합, report export v2
- [x] 실제 executable + SQLite SAFE / UNSAFE / negative corpus
- [x] 최종 fmt / clippy / workspace tests / release build
- [x] 실제 CLI corpus 및 desktop / 390px browser gate
- [x] P5-RESULT 및 CURRENT에 완료 gate 기록

설계 및 사용: [SIDEEFFECT.md](../docs/SIDEEFFECT.md)

**P5 LOCAL GATE PASS — 272/272 tests, corpus 9/9, browser desktop/mobile PASS. P6 미시작.**
