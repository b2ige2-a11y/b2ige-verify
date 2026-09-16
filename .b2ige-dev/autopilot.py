#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import time
from typing import Any

DEV_NAME = ".b2ige-dev"

def now() -> str:
    return dt.datetime.now().astimezone().isoformat(timespec="seconds")

def stamp() -> str:
    return dt.datetime.now().strftime("%Y%m%d-%H%M%S")

class AutoPilot:
    def __init__(self, root: Path):
        self.root = root.resolve()
        self.dev = self.root / DEV_NAME
        self.runs = self.dev / "runs"
        self.config_path = self.dev / "AUTOPILOT.json"
        self.plan_path = self.dev / "PLAN.json"
        self.state_path = self.dev / "STATE.json"
        self.roadmap_path = self.dev / "ROADMAP_100.md"
        self.decision_path = self.dev / "DECISIONS.md"
        self.failure_path = self.dev / "FAILURES.md"
        self.schemas = self.dev / "schemas"
        self.runs.mkdir(parents=True, exist_ok=True)
        self.cfg = self._load_json(self.config_path)
        self.state = self._load_json(self.state_path, default={})
        self.goal = self.state.get("goal") or self.cfg.get(
            "default_goal",
            "Complete the B2IGE Verify 100-point roadmap end-to-end without weakening its trust contracts."
        )

    def _load_json(self, p: Path, default=None):
        if not p.exists():
            if default is not None:
                return default
            raise RuntimeError(f"Missing required file: {p}")
        try:
            return json.loads(p.read_text())
        except Exception as e:
            raise RuntimeError(f"Invalid JSON in {p}: {e}")

    def _write_json(self, p: Path, data: Any):
        tmp = p.with_suffix(p.suffix + ".tmp")
        tmp.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
        tmp.replace(p)

    def log(self, msg: str):
        line = f"[{now()}] {msg}"
        print(line, flush=True)
        with (self.runs / "autopilot.log").open("a") as f:
            f.write(line + "\n")

    def run(self, cmd, *, check=True, timeout=None, cwd=None, capture=True):
        self.log("$ " + " ".join(shlex.quote(str(x)) for x in cmd))
        try:
            cp = subprocess.run(
                [str(x) for x in cmd],
                cwd=str(cwd or self.root),
                text=True,
                stdout=subprocess.PIPE if capture else None,
                stderr=subprocess.STDOUT if capture else None,
                timeout=timeout,
                check=False,
                env=os.environ.copy(),
            )
        except subprocess.TimeoutExpired as e:
            out = (e.stdout or "") + "\n[TIMEOUT]"
            self.log(out[-4000:])
            if check:
                raise RuntimeError(f"Command timed out: {cmd}")
            return 124, out
        out = cp.stdout or ""
        if out.strip():
            self.log(out[-4000:])
        if check and cp.returncode != 0:
            raise RuntimeError(f"Command failed ({cp.returncode}): {' '.join(map(str, cmd))}")
        return cp.returncode, out

    def doctor(self):
        required = ["git", "codex", "cargo", "rustc", "python3", "docker"]
        missing = [x for x in required if shutil.which(x) is None]
        if missing:
            raise RuntimeError("Missing executables: " + ", ".join(missing))
        branch = self.git(["branch", "--show-current"]).strip()
        expected = self.cfg.get("branch", "automation/100-point")
        if branch != expected:
            raise RuntimeError(f"Expected branch {expected!r}, got {branch!r}")
        self.run(["docker", "info"], check=True, timeout=30)
        self.run(["codex", "--version"], check=True, timeout=30)
        self.run(["git", "remote", "-v"], check=True, timeout=30)
        if not self.roadmap_path.exists() or not self.roadmap_path.read_text().strip():
            raise RuntimeError("ROADMAP_100.md is missing or empty")
        self.log("Doctor: READY")

    def git(self, args, check=True):
        rc, out = self.run(["git", *args], check=check, timeout=600)
        return out

    def codex_call(self, role: str, model_key: str, prompt: str, schema_name: str, sandbox="workspace-write"):
        spec = self.cfg["models"][model_key]
        model = spec["model"]
        effort = spec["reasoning"]
        ts = stamp()
        safe_role = re.sub(r"[^A-Za-z0-9._-]+", "_", role)[:120]
        out_file = self.runs / f"{ts}-{safe_role}-last.json"
        log_file = self.runs / f"{ts}-{safe_role}.log"
        schema = self.schemas / schema_name
        cmd = [
            "codex", "exec", "--ephemeral",
            "-C", str(self.root),
            "-m", model,
            "-c", f'model_reasoning_effort="{effort}"',
            "-a", "never",
            "-s", sandbox,
            "--output-schema", str(schema),
            "-o", str(out_file),
            prompt,
        ]
        self.log(f"Codex role={role} model={model} reasoning={effort} sandbox={sandbox}")
        with log_file.open("w") as lf:
            cp = subprocess.Popen(
                cmd, cwd=self.root, text=True,
                stdout=lf, stderr=subprocess.STDOUT, env=os.environ.copy()
            )
            rc = cp.wait()
        if rc != 0:
            tail = log_file.read_text(errors="replace")[-8000:]
            self.log(f"Codex failed rc={rc}\n{tail}")
            raise RuntimeError(f"Codex {role} failed with rc={rc}")
        if not out_file.exists():
            raise RuntimeError(f"Codex {role} did not write {out_file}")
        try:
            data = json.loads(out_file.read_text())
        except Exception as e:
            raise RuntimeError(f"Invalid structured output from Codex {role}: {e}")
        return data

    def authoritative_docs_text(self):
        docs = [
            "AGENTS.md",
            "docs/CONTRACTS.md",
            "docs/VERDICTS.md",
            "docs/EVIDENCE.md",
            "docs/THREAT-MODEL.md",
            "docs/ARCHITECTURE.md",
        ]
        existing = [x for x in docs if (self.root / x).exists()]
        return ", ".join(existing)

    def ensure_clean_start(self):
        out = self.git(["status", "--porcelain"])
        allowed_prefixes = (".b2ige-dev/",)
        foreign = []
        for line in out.splitlines():
            path = line[3:] if len(line) >= 4 else line
            if not path.startswith(allowed_prefixes):
                foreign.append(line)
        if foreign:
            raise RuntimeError(
                "Refusing to start with unrelated working-tree changes:\n" + "\n".join(foreign)
            )

    def load_plan(self):
        if not self.plan_path.exists():
            return None
        return self._load_json(self.plan_path)

    def plan(self, reason="initial"):
        prompt = f"""
You are the planning controller for an autonomous B2IGE Verify development run.

GOAL:
{self.goal}

Reason for planning: {reason}

Read .b2ige-dev/ROADMAP_100.md and the repository. Read authoritative contracts first:
{self.authoritative_docs_text()}

Produce a coherent implementation queue that can be executed unattended from P4 through the
remaining roadmap. Prefer substantial work packages, not one-file microtasks. Keep the list
bounded (roughly 12-24 packages for a full initial plan). Each package must have measurable
acceptance criteria and a risk level:
- default: routine implementation, tests, docs, packaging, CI, platform work
- advanced: architecture, Task Seal, Verifier Forge, anti-oracle, complex isolation, protocol design
- critical: anything capable of weakening false-PASS resistance, verdict semantics, evidence
  invariants, security/isolation boundaries, verifier commitments, cryptographic trust, or core
  receipt/attestation semantics

Do not implement code. Do not modify files. Do not mark impossible external account setup as
completed; phrase those as locally implementable deliverables unless credentials already exist.
Preserve the authority order in AGENTS.md.
"""
        result = self.codex_call("planner", "advanced", prompt, "planner.schema.json", sandbox="read-only")
        plan = {
            "schema_version": 1,
            "created_at": now(),
            "goal": self.goal,
            "work_packages": []
        }
        for i, wp in enumerate(result["work_packages"], 1):
            item = dict(wp)
            item["status"] = "pending"
            item["attempts"] = 0
            item["created_order"] = i
            plan["work_packages"].append(item)
        self._write_json(self.plan_path, plan)
        self.log(f"Planner created {len(plan['work_packages'])} work packages")
        return plan

    def route(self, wp):
        text = json.dumps(wp, ensure_ascii=False).lower()
        critical_terms = [
            "false pass", "false-pass", "verdict", "evidence invariant", "isolation boundary",
            "security boundary", "verifier commitment", "cryptographic", "attestation core",
            "pass semantics", "hidden leakage", "tamper"
        ]
        advanced_terms = [
            "architecture", "task seal", "verifier forge", "anti-oracle", "anti oracle",
            "protocol", "isolation", "verifier generation", "receipt"
        ]
        if wp.get("risk") == "critical" or any(t in text for t in critical_terms):
            return "critical"
        if wp.get("risk") == "advanced" or any(t in text for t in advanced_terms):
            return "advanced"
        return "default"

    def next_pending(self, plan):
        for wp in plan["work_packages"]:
            if wp.get("status") == "pending":
                return wp
        return None

    def changed_files(self):
        out = self.git(["status", "--porcelain"])
        files = []
        for line in out.splitlines():
            if not line:
                continue
            p = line[3:]
            if " -> " in p:
                p = p.split(" -> ", 1)[1]
            if p.startswith(".b2ige-dev/runs/"):
                continue
            files.append(p)
        return files

    def deterministic_gates(self, risk: str):
        failures = []
        commands = [["git", "diff", "--check"]]
        changed = self.changed_files()
        rust_changed = any(p.endswith(".rs") or p in ("Cargo.toml", "Cargo.lock") for p in changed)
        if rust_changed:
            commands.append(["cargo", "fmt", "--check"])
            if risk == "critical":
                commands.append(["cargo", "test", "--workspace", "--locked"])
            else:
                commands.append(["cargo", "check", "--workspace", "--locked"])
        for cmd in commands:
            rc, out = self.run(cmd, check=False, timeout=self.cfg.get("gate_timeout_seconds", 3600))
            if rc != 0:
                failures.append({"command": cmd, "returncode": rc, "tail": out[-5000:]})
        return failures

    def worker_prompt(self, wp, attempt, previous_failure=""):
        return f"""
You are an autonomous implementation worker inside B2IGE Verify.

GLOBAL GOAL:
{self.goal}

CURRENT WORK PACKAGE:
{json.dumps(wp, indent=2, ensure_ascii=False)}

Attempt: {attempt}

Read AGENTS.md and authoritative contracts before touching verifier logic:
{self.authoritative_docs_text()}

Rules:
1. Implement this work package completely; do not wander into unrelated roadmap work.
2. You may inspect/edit repository files and run terminal commands/tests yourself.
3. Never weaken a contract or test just to obtain green output.
4. Missing verdict-critical evidence must never become PASS.
5. An LLM must never authoritatively assign PASS/FAIL/INCONCLUSIVE/ERROR.
6. Do not expose hidden verifier/oracle material to the coding-agent workspace where secrecy is claimed.
7. Do NOT git commit, git push, merge, rewrite history, or alter remotes. The outer controller owns Git.
8. Do NOT edit .b2ige-dev/PLAN.json, STATE.json, AUTOPILOT.json, schemas, or runtime control files.
9. Run focused tests appropriate to the changes.
10. If blocked, report the concrete blocker. Otherwise finish the implementation and leave changes
    in the working tree for independent review.

Previous failure/review context:
{previous_failure or "(none)"}
"""

    def review_prompt(self, wp, risk):
        return f"""
Act as an independent B2IGE Verify gate reviewer. Do not edit files.

Review the CURRENT UNCOMMITTED WORKING TREE for this work package:
{json.dumps(wp, indent=2, ensure_ascii=False)}

Risk classification: {risk}

Read AGENTS.md and the authoritative contracts:
{self.authoritative_docs_text()}

Inspect git diff, changed files, relevant tests, and the work package acceptance criteria.
Reject any change that:
- can turn missing verdict-critical evidence into PASS,
- confuses observer/verifier failure with product success,
- lets an LLM assign authoritative verdicts,
- weakens claimed isolation or hidden-data boundaries without a deliberate contract/version change,
- auto-approves poisoned baselines,
- makes failure unreproducible without documenting why,
- claims guarantees stronger than executed evidence,
- satisfies a test by weakening/deleting the test rather than fixing behavior.

Return approval only when the package is actually complete and the evidence is adequate.
Do not implement fixes yourself.
"""

    def implement_package(self, plan, wp):
        risk = self.route(wp)
        base_model = risk
        max_attempts = int(self.cfg.get("max_worker_attempts", 4))
        prev = ""
        for attempt in range(1, max_attempts + 1):
            wp["attempts"] = int(wp.get("attempts", 0)) + 1
            self._write_json(self.plan_path, plan)
            model_key = base_model
            if attempt >= max_attempts and risk != "critical":
                model_key = "critical"
            elif attempt >= 3 and risk == "default":
                model_key = "advanced"
            self.log(f"Work package {wp['id']} attempt={attempt} route={model_key}")
            result = self.codex_call(
                f"worker-{wp['id']}-a{attempt}",
                model_key,
                self.worker_prompt(wp, attempt, prev),
                "worker.schema.json",
                sandbox="workspace-write",
            )
            if result.get("status") == "blocked":
                prev = "Worker blocker:\n" + result.get("summary", "unknown")
                self.log(prev)
                continue

            gate_failures = self.deterministic_gates(risk)
            if gate_failures:
                prev = "Deterministic gate failures:\n" + json.dumps(
                    gate_failures, indent=2, ensure_ascii=False
                )
                self.log(prev[-8000:])
                continue

            review_model = "critical" if risk == "critical" else ("advanced" if risk == "advanced" else "default")
            review = self.codex_call(
                f"review-{wp['id']}-a{attempt}",
                review_model,
                self.review_prompt(wp, risk),
                "review.schema.json",
                sandbox="read-only",
            )
            if review.get("approved"):
                return True, review
            prev = "Independent review rejected the change:\n" + json.dumps(
                review, indent=2, ensure_ascii=False
            )
            self.log(prev[-8000:])

        return False, {"approved": False, "issues": [prev or "attempt limit reached"]}

    def commit_and_push(self, plan, wp):
        wp["status"] = "completed"
        wp["completed_at"] = now()
        self.state["current_work_package"] = None
        self.state["last_completed"] = wp["id"]
        self.state["updated_at"] = now()
        self._write_json(self.plan_path, plan)
        self._write_json(self.state_path, self.state)

        self.git(["add", "-A"])
        status = self.git(["status", "--porcelain"]).strip()
        if not status:
            self.log(f"No changes to commit for {wp['id']}")
            return
        msg = f"autopilot: {wp['id']} {wp['title']}"
        self.git(["commit", "-m", msg])
        if self.cfg.get("push_each_package", True):
            remote = self.cfg.get("remote", "origin")
            branch = self.cfg.get("branch", "automation/100-point")
            self.git(["push", remote, branch])
        self.log(f"Committed and pushed {wp['id']}")

    def record_failure(self, wp, detail):
        with self.failure_path.open("a") as f:
            f.write(f"\n## {now()} — {wp.get('id')} {wp.get('title')}\n\n")
            f.write(detail.rstrip() + "\n")
        self.state["status"] = "BLOCKED"
        self.state["current_work_package"] = wp.get("id")
        self.state["updated_at"] = now()
        self._write_json(self.state_path, self.state)

    def final_gates(self):
        cmds = [
            ["git", "diff", "--check"],
            ["cargo", "fmt", "--check"],
            ["cargo", "test", "--workspace", "--locked"],
            ["cargo", "build", "--workspace", "--release", "--locked"],
        ]
        failures = []
        for cmd in cmds:
            rc, out = self.run(cmd, check=False, timeout=self.cfg.get("final_gate_timeout_seconds", 7200))
            if rc != 0:
                failures.append({"command": cmd, "returncode": rc, "tail": out[-8000:]})
        demo = self.root / "scripts" / "demo.py"
        if not failures and demo.exists():
            rc, out = self.run(
                ["python3", str(demo), "all"],
                check=False,
                timeout=self.cfg.get("final_gate_timeout_seconds", 7200),
            )
            if rc != 0:
                failures.append({"command": ["python3", "scripts/demo.py", "all"], "returncode": rc, "tail": out[-8000:]})
        return failures

    def final_audit(self, plan, remediation_cycle):
        prompt = f"""
You are the FINAL independent audit for the autonomous B2IGE Verify 100-point program.
Do not edit files.

GLOBAL GOAL:
{self.goal}

Roadmap: .b2ige-dev/ROADMAP_100.md
Plan: .b2ige-dev/PLAN.json
Remediation cycle: {remediation_cycle}

Read AGENTS.md and all authoritative contracts. Inspect the full branch relative to origin/main,
including commits, tests, docs, threat boundaries, and roadmap exit gates.

The bar is not "a lot of code was added". Approve only if the implemented local/software scope
materially satisfies the roadmap without weakening false-PASS resistance, evidence semantics,
isolation claims, or verdict determinism. External paid/cloud accounts that are not configured
must not be fabricated; local deployable implementations and explicit operational prerequisites
are acceptable.

Return:
- approved
- summary
- issues
- remediation_work_packages for every concrete remaining gap (empty when approved).
Each remediation package must contain id, phase, title, risk, objective, acceptance_criteria.
"""
        return self.codex_call(
            f"final-audit-r{remediation_cycle}",
            "critical",
            prompt,
            "final_audit.schema.json",
            sandbox="read-only",
        )

    def append_remediation(self, plan, audit, cycle):
        existing = {wp["id"] for wp in plan["work_packages"]}
        count = 0
        for raw in audit.get("remediation_work_packages", []):
            wp = dict(raw)
            base_id = wp.get("id") or f"REM-{cycle}-{count+1}"
            ident = base_id
            n = 2
            while ident in existing:
                ident = f"{base_id}-{n}"
                n += 1
            wp["id"] = ident
            wp["status"] = "pending"
            wp["attempts"] = 0
            wp["created_order"] = len(plan["work_packages"]) + 1
            plan["work_packages"].append(wp)
            existing.add(ident)
            count += 1
        self._write_json(self.plan_path, plan)
        return count

    def run_autopilot(self, goal=None):
        if goal:
            self.goal = goal
            self.state["goal"] = goal
        else:
            self.state.setdefault("goal", self.goal)
        self.state["status"] = "RUNNING"
        self.state["started_or_resumed_at"] = now()
        self._write_json(self.state_path, self.state)

        self.doctor()
        self.ensure_clean_start()

        plan = self.load_plan()
        if not plan or not plan.get("work_packages"):
            plan = self.plan("initial")

        remediation_cycle = int(self.state.get("remediation_cycle", 0))
        max_remediation = int(self.cfg.get("max_remediation_cycles", 3))

        while True:
            wp = self.next_pending(plan)
            if wp is not None:
                self.state["current_phase"] = wp.get("phase")
                self.state["current_work_package"] = wp.get("id")
                self.state["updated_at"] = now()
                self._write_json(self.state_path, self.state)

                ok, detail = self.implement_package(plan, wp)
                if not ok:
                    text = json.dumps(detail, indent=2, ensure_ascii=False)
                    self.record_failure(wp, text)
                    self.git(["add", ".b2ige-dev/STATE.json", ".b2ige-dev/FAILURES.md", ".b2ige-dev/PLAN.json"])
                    self.git(["commit", "-m", f"autopilot: block on {wp['id']}"], check=False)
                    if self.cfg.get("push_each_package", True):
                        self.git(["push", self.cfg.get("remote", "origin"), self.cfg.get("branch", "automation/100-point")], check=False)
                    raise RuntimeError(f"Autopilot blocked on {wp['id']}")
                self.commit_and_push(plan, wp)
                continue

            self.log("No pending work packages; running deterministic final gates.")
            failures = self.final_gates()
            if failures:
                remediation_cycle += 1
                self.state["remediation_cycle"] = remediation_cycle
                self._write_json(self.state_path, self.state)
                if remediation_cycle > max_remediation:
                    raise RuntimeError("Final deterministic gates still fail after remediation budget.")
                audit = {
                    "remediation_work_packages": [{
                        "id": f"REM-GATE-{remediation_cycle}",
                        "phase": "FINAL",
                        "title": "Repair final deterministic gate failures",
                        "risk": "critical",
                        "objective": "Repair the final gate failures without weakening tests or contracts.",
                        "acceptance_criteria": [
                            "All listed deterministic final gate failures are fixed",
                            "No authoritative contract is weakened to obtain green output"
                        ],
                    }]
                }
                self.append_remediation(plan, audit, remediation_cycle)
                with self.failure_path.open("a") as f:
                    f.write(f"\n## {now()} — Final gate remediation {remediation_cycle}\n\n")
                    f.write(json.dumps(failures, indent=2, ensure_ascii=False) + "\n")
                continue

            audit = self.final_audit(plan, remediation_cycle)
            if audit.get("approved"):
                self.state["status"] = "COMPLETE"
                self.state["completed_at"] = now()
                self.state["current_phase"] = "COMPLETE"
                self.state["current_work_package"] = None
                self._write_json(self.state_path, self.state)
                self.git(["add", "-A"])
                if self.git(["status", "--porcelain"]).strip():
                    self.git(["commit", "-m", "autopilot: complete 100-point program"])
                self.git(["push", self.cfg.get("remote", "origin"), self.cfg.get("branch", "automation/100-point")])
                self.log("AUTOPILOT COMPLETE")
                return 0

            remediation_cycle += 1
            self.state["remediation_cycle"] = remediation_cycle
            self._write_json(self.state_path, self.state)
            if remediation_cycle > max_remediation:
                detail = json.dumps(audit, indent=2, ensure_ascii=False)
                self.record_failure({"id": "FINAL-AUDIT", "title": "Final audit"}, detail)
                raise RuntimeError("Final audit did not approve within remediation budget.")
            added = self.append_remediation(plan, audit, remediation_cycle)
            if added == 0:
                raise RuntimeError("Final audit rejected but provided no remediation packages.")
            self.log(f"Final audit added {added} remediation packages.")

def repo_root():
    cp = subprocess.run(["git", "rev-parse", "--show-toplevel"], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if cp.returncode != 0:
        raise RuntimeError("Run this from inside the B2IGE Verify git repository.")
    return Path(cp.stdout.strip())

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--doctor", action="store_true")
    ap.add_argument("--goal")
    args = ap.parse_args()
    pilot = AutoPilot(repo_root())
    if args.doctor:
        pilot.doctor()
        return 0
    return pilot.run_autopilot(args.goal)

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("Interrupted.", file=sys.stderr)
        raise SystemExit(130)
    except Exception as e:
        print(f"AUTOPILOT ERROR: {e}", file=sys.stderr)
        try:
            root = repo_root()
            p = root / DEV_NAME / "runs" / "autopilot.log"
            p.parent.mkdir(parents=True, exist_ok=True)
            with p.open("a") as f:
                f.write(f"[{now()}] FATAL: {e}\n")
        except Exception:
            pass
        raise SystemExit(1)
