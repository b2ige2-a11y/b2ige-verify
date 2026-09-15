# Canonicalization and Hashing v1

## Purpose
Receipts must identify exactly what experiment and evidence were evaluated.

## v1 rule
- JSON objects are serialized using RFC 8785 JSON Canonicalization Scheme (JCS).
- Hash algorithm: SHA-256.
- Hash text form: `sha256:<lowercase hex>`.
- Volatile presentation-only fields are excluded before canonicalization only when the schema explicitly marks them non-authoritative.
- Raw evidence blobs are hashed independently; the evidence object stores the blob hash.
- The receipt hash covers authoritative run metadata, plan hash, config hash, target identity, coverage, checker outputs, and evidence hashes.

## Prohibition
Never hash pretty-printed JSON or filesystem ordering and call it canonical.

## P1A repair implementation boundary

`verify_evidence::canonical_bytes` is the single canonical representation used by
hashing and checker JSON equality. It validates a serde-value tree before invoking
serde_json_canonicalizer 0.3.2 so nested NaN/Infinity cannot be lost as JSON null.
JSON property ordering is unescaped UTF-16 order and numeric comparison follows
the same ECMAScript binary64 serialization as hashing (including 1 == 1.0).

Seeds, budgets, sequence orders and other exact integer metadata are restricted to
0..2^53-1, with existing positive-only constraints retained. This prevents distinct
exact experiment inputs sharing a rounded numeric hash. Arbitrary JSON observation
numbers have binary64 semantics; larger exact values must be represented as strings.

Inputs are typed, trusted synthetic objects. Typed coverage/observer-version/budget
maps reject duplicate raw keys. General untrusted JSON ingestion and detection of
keys already lost by an upstream Value parser are not implemented. Hashes remain
integrity identifiers, not proof of provenance or approval. See the explicit repair
decision in docs/P1A-SCHEMA-DECISION.md for approval bindings and compatibility.
