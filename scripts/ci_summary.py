"""Render a deliberately small GitHub/agent-facing summary from Agent Protocol v1."""
import json
import re


PRODUCTS = {"behavior", "sideeffect", "blindtest"}
OPERATIONS = {"verify", "report"}
VERDICTS = {"PASS", "FAIL", "INCONCLUSIVE", "ERROR"}
HASH = re.compile(r"^sha256:[0-9a-f]{64}$")
KEY = re.compile(r"^[a-z0-9_]{1,64}$")

MESSAGES = {
    "PASS": "No contract violation was observed within the declared tested scope.",
    "FAIL": "A deterministic verifier found a contract violation.",
    "INCONCLUSIVE": "Required evidence is incomplete; completion is not established.",
    "ERROR": "The verifier or its required configuration/evidence could not complete.",
}
ACTIONS = {
    "PASS": "Assess remaining changed surfaces; this is not exhaustive proof.",
    "FAIL": "Use the sanitized reproduction/evidence references to repair, then verify again.",
    "INCONCLUSIVE": "Obtain the missing required evidence and rerun; do not claim completion.",
    "ERROR": "Repair verifier configuration or infrastructure and rerun; do not infer a product failure.",
}


def _safe_text(value, limit=240):
    if not isinstance(value, str) or not value or len(value) > limit:
        return None
    if any(ord(char) < 32 or ord(char) == 127 for char in value):
        return None
    if "/" in value or "\\" in value or "BLINDTEST_PRIVATE" in value:
        return None
    return value.replace("`", "'").replace("|", "\\|").replace("\n", " ")


def _safe_int(value):
    return isinstance(value, int) and not isinstance(value, bool) and 0 <= value <= 2**53


def _public_detail(value):
    """Keep only fixed-shape public observables; arbitrary strings are omitted."""
    if value is None:
        return None
    if isinstance(value, dict):
        kind = value.get("kind")
        if kind == "status" and set(value) <= {"kind", "value"}:
            status = value.get("value")
            if status is None or (isinstance(status, int) and not isinstance(status, bool) and -1 <= status <= 255):
                return f"status={status if status is not None else 'unavailable'}"
        if kind == "bytes" and set(value) == {"kind", "sha256", "length"}:
            if (isinstance(value.get("sha256"), str)
                    and HASH.fullmatch(value["sha256"])
                    and _safe_int(value.get("length"))):
                return f"bytes sha256={value['sha256']} length={value['length']}"
        violation = value.get("violation")
        if isinstance(violation, str) and violation in {
            "duplicate_committed_effect",
            "missing_committed_effect",
            "forbidden_committed_effect",
            "commit_order",
            "half_commit",
        }:
            return f"violation={violation}"
    if isinstance(value, list):
        details = []
        for item in value[:16]:
            detail = _public_detail(item)
            if detail:
                details.append(detail)
        return "; ".join(details) if details else None
    # Free-form verifier/model/target text is intentionally not projected into
    # a public CI summary. Fixed-shape observations above remain actionable.
    return None


def _scope_counts(payload):
    scope = payload.get("scope")
    if not isinstance(scope, dict):
        return []
    counts = []
    for group in ("coverage", "budget"):
        values = scope.get(group)
        if not isinstance(values, dict):
            continue
        for key, value in sorted(values.items()):
            if KEY.fullmatch(key) and _safe_int(value):
                counts.append(f"{key}={value}")
    return counts[:32]


def render_error():
    return (
        "## B2IGE Verify · ERROR\n\n"
        "The verifier did not produce a usable sanitized Agent Protocol response.\n\n"
        "Next action: repair verifier configuration or infrastructure and rerun."
    )


def render_payload(payload):
    if (not isinstance(payload, dict) or payload.get("product") not in PRODUCTS
            or payload.get("protocol_version") != "1"
            or payload.get("operation") not in OPERATIONS
            or not isinstance(payload.get("kind"), str)
            or not payload["kind"]
            or payload.get("kind") == "readiness"
            or payload.get("verdict") not in VERDICTS
            or (payload["verdict"] != "ERROR"
                and (not isinstance(payload.get("source"), dict)
                     or not isinstance(payload.get("scope"), dict)))):
        return render_error()
    verdict = payload["verdict"]
    product = payload["product"]
    operation = payload["operation"]
    lines = [
        f"## B2IGE Verify · {product} · {verdict}",
        "",
        MESSAGES[verdict],
        "",
        f"- Operation: `{operation}`",
        f"- Verified evidence references: {len(payload.get('evidence_refs', [])) if isinstance(payload.get('evidence_refs'), list) else 0}",
        f"- Other reported findings: {payload.get('other_failure_count', 0) if _safe_int(payload.get('other_failure_count')) else 0}",
    ]
    counts = _scope_counts(payload)
    if counts:
        lines.append(f"- Bounded scope counters: `{', '.join(counts)}`")
    if verdict == "FAIL":
        expected = _public_detail(payload.get("expected"))
        observed = _public_detail(payload.get("observed"))
        if expected:
            lines.append(f"- Expected (sanitized): `{expected}`")
        if observed:
            lines.append(f"- Observed (sanitized): `{observed}`")
    lines.extend(["", f"Next action: {ACTIONS[verdict]}"])
    return "\n".join(lines)


def render_many(items):
    sections = []
    for label, payload in items:
        heading = _safe_text(label, 128) or "configured contract"
        section = render_payload(payload)
        sections.append(f"### Contract `{heading}`\n\n{section}")
    return "\n\n---\n\n".join(sections) if sections else render_error()


def load_payload(path):
    data = path.read_bytes()
    if len(data) > 4 * 1024 * 1024:
        raise ValueError("summary input too large")
    return json.loads(data)
