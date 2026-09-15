#!/usr/bin/env python3
"""Run the complete bounded corpus in both orders; never update a baseline."""
import json
import pathlib
import subprocess
import sys


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 benchmarks/run-local.py NEW_OUTPUT_DIRECTORY")
    root = pathlib.Path(sys.argv[1]).resolve()
    root.mkdir()  # No overwrite, including failed/partial earlier runs.
    repo = pathlib.Path(__file__).resolve().parents[1]
    subprocess.run(["cargo", "build", "--workspace", "--release", "--locked"], cwd=repo, check=True)
    binary = repo / "target/release/b2ige"
    results = []
    for name, flags in [("fixed", []), ("reverse", ["--reverse"])]:
        with (root / f"{name}.stdout.json").open("x") as output:
            subprocess.run([str(binary), "bench", "--output", "json", "--save", str(root / name), *flags], cwd=repo, stdout=output, check=True)
        result = json.loads((root / name / "result.json").read_text())
        if not result["summary"]["gate_pass"] or not result["complete_release_corpus"]:
            raise SystemExit(f"{name}: full release gate blocked")
        results.append(result)
    comparison = subprocess.run([str(binary), "bench", "compare", str(root / "fixed/result.json"), str(root / "reverse/result.json")], cwd=repo, capture_output=True, text=True, check=True)
    (root / "comparison.json").write_text(comparison.stdout)
    if results[0]["semantic_hash"] != results[1]["semantic_hash"]:
        raise SystemExit("Order-dependent semantic difference: inspect comparison.json")
    print(f"31-case fixed/reverse gates PASS; semantic results unchanged. {root}")


if __name__ == "__main__":
    main()
