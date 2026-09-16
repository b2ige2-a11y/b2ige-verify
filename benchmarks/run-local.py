#!/usr/bin/env python3
"""Run an explicit bounded scope in both orders; never update a baseline."""
import argparse
import json
import pathlib
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_directory")
    parser.add_argument("--product", choices=["behavior", "sideeffect", "blindtest"],
                        help="Measure only this product; never grants the full release gate")
    args = parser.parse_args()
    root = pathlib.Path(args.output_directory).resolve()
    root.mkdir()  # No overwrite, including failed/partial earlier runs.
    repo = pathlib.Path(__file__).resolve().parents[1]
    subprocess.run(["cargo", "build", "--workspace", "--release", "--locked"], cwd=repo, check=True)
    binary = repo / "target/release/b2ige"
    results = []
    scope = [args.product] if args.product else []
    for name, flags in [("fixed", []), ("reverse", ["--reverse"])]:
        with (root / f"{name}.stdout.json").open("x") as output:
            subprocess.run([str(binary), "bench", *scope, "--output", "json", "--save", str(root / name), *flags], cwd=repo, stdout=output, check=True)
        result = json.loads((root / name / "result.json").read_text())
        if args.product:
            if (result["complete_release_corpus"] or result["summary"]["gate_pass"]
                    or not result["products"][args.product]["gate_pass"]):
                raise SystemExit(f"{name}: invalid or blocked product scope")
        elif not result["summary"]["gate_pass"] or not result["complete_release_corpus"]:
            raise SystemExit(f"{name}: full release gate blocked")
        results.append(result)
    comparison = subprocess.run([str(binary), "bench", "compare", str(root / "fixed/result.json"), str(root / "reverse/result.json")], cwd=repo, capture_output=True, text=True, check=True)
    (root / "comparison.json").write_text(comparison.stdout)
    if not json.loads(comparison.stdout)["semantic_equal"]:
        raise SystemExit("Order-dependent semantic difference: inspect comparison.json")
    label = f"{args.product} scoped gates" if args.product else "full release gates"
    print(f"{len(results[0]['cases'])}-case fixed/reverse {label} PASS; semantic results unchanged. {root}")


if __name__ == "__main__":
    main()
