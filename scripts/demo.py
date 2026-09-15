#!/usr/bin/env python3
"""Run actual public CLI examples. All generated evidence stays in a fresh controller root."""
import json, os, pathlib, subprocess, sys, tempfile

def run(args, expected=0, env=None):
    r = subprocess.run([str(x) for x in args], env=env, text=True, capture_output=True, timeout=180)
    if r.returncode != expected:
        raise RuntimeError(f"{args}: expected exit {expected}, got {r.returncode}\n{r.stdout}\n{r.stderr}")
    return r.stdout

def demo(product, bindir, root):
    cli = bindir / "b2ige"
    run([bindir / "b2ige-demo", product, root])
    env = dict(os.environ)
    if product == "blindtest":
        env["B2IGE_BLINDTEST_SEALED_ROOT"] = str(root / "sealed")
        # Agent-visible test checks a valid credential only; both implementations pass.
        for mode in ["correct", "mutant_a"]:
            config = json.loads((root / f"{mode}.json").read_text())
            out = run(["docker", "run", "--rm", "--network=none", "--read-only", "--user", "65532:65532", "--cap-drop=ALL", "--security-opt=no-new-privileges", "-e", "LOGICAL_NOW=1", config["target"]["image"], "node", "/app/target.js", "2"])
            assert out == "session created\n"
        print("Coding-agent-visible valid-credential tests: PASS (correct and buggy)")
        configs = [("correct", root / "correct.json", 0), ("buggy", root / "mutant_a.json", 1), ("probe", root / "probe.json", 0)]
    elif product == "behavior":
        configs = [(mode, root / mode / "experiment.json", code) for mode, code in [("pass", 0), ("fail", 1)]]
    else:
        configs = [(mode, root / f"{mode}.contract.json", code) for mode, code in [("safe", 0), ("unsafe", 1)]]
    for name, config, code in configs:
        extra = ["--authorization", config.parent / "authorization.json"] if product == "behavior" else []
        store = root / ("sealed/runs" if product == "blindtest" else "runs")
        raw = run([cli, product, "verify", config, "--store", store, *extra, "--output", "agent", "--protocol", "1"], code, env)
        response = json.loads(raw)
        assert response["verdict"] == ("PASS" if code == 0 else "FAIL")
        (root / f"{name}.agent.json").write_text(raw)
        print(f"{product} {name}: {response['verdict']}")
        if product == "blindtest":
            suite = json.loads((root / "sealed/suite.json").read_text())
            private = [suite["private_canary"], suite["private_metadata"], str(root / "sealed")]
            for case in suite["cases"]:
                private += [case["case_id"], *case["args"], *case["environment"].values()]
            assert all(value not in raw for value in private)
            print("Agent private-value leakage: 0")
        else:
            artifact = response["source"]["artifact_id"]
            for output in ["human", "json", "agent"]:
                report = run([cli, "report", artifact, "--store", store, *extra, "--output", output], code, env)
                (root / f"{name}.{output}.txt").write_text(report)
    return root

if __name__ == "__main__":
    product = sys.argv[1]
    bindir = pathlib.Path(os.environ.get("B2IGE_BIN_DIR", "target/release")).resolve()
    parent = pathlib.Path(tempfile.mkdtemp(prefix="b2ige-demo-"))
    for item in (["behavior", "sideeffect", "blindtest"] if product == "all" else [product]):
        demo(item, bindir, parent / item)
    print(f"Trusted local evidence: {parent}")
