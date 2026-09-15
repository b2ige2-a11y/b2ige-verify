# P5 ORDERED_AFTER Repair Result

## Root cause

`ordered_after(A, B)`의 checker가 `order(A) < order(B)`만 위반으로 처리해
`order(A) == order(B)`를 성공으로 통과시켰다. 따라서 하나의 authoritative SQLite
row가 서로 다른 logical effect alias로 관측되면 동일 commit이 ordering을 만족한 것처럼
판정될 수 있었다.

## 수정

`crates/verify-core/src/sideeffect.rs`의 relational checker를 최소 수정해
`order(A) <= order(B)`를 `CommitOrder` 위반으로 처리한다. ordering은 계속
authoritative SQLite commit sequence만 사용하며 observer를 광범위하게 제한하지 않는다.
독립 failing regression test는 그대로 유지했다.

## 검증

- 수정 전 독립 regression: 동일 physical commit/order 1 alias가 `PASS`하여 실패.
- 수정 후 독립 regression: **1/1 PASS**.
- P5 전용 tests: core **39/39**, CLI/report **6/6**.
- `RUST_TEST_THREADS=4 cargo test --workspace`: **273/273 PASS**.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `cargo build --workspace --release`: PASS.
- release corpus: S1–S3 PASS, U1–U3 FAIL, N1–N2 INCONCLUSIVE, N3 ERROR.
- corpus guards: known unsafe false PASS **0**, SAFE false FAIL **0**, missing committed
  evidence PASS **0**, unexecuted fault PASS **0**.

## 판정

**P5 REPAIR PASS. P6 시작 가능.** P6는 시작하지 않았으며 commit/push/deploy도 수행하지 않았다.
