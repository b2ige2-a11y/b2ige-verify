# Security and support

## Supported versions

| Version | Security maintenance |
|---|---|
| 0.1.x | Latest patch only |
| Other versions | No maintenance commitment |

0.2.0 is the current public release; 0.3.0 is an unpublished candidate. Report older-patch issues against the latest patch
when available. No backport, fixed support lifetime or guaranteed remediation deadline is
promised. Package and evidence schema versions are independent. Support is best-effort
community support with no SLA.

## Private vulnerability reports

**ACTIVE: GitHub Private Vulnerability Reporting is enabled.**

The repository is public and its private-reporting channel is enabled. Use the
**Security → Advisories → Report a vulnerability** action. A SECURITY.md file alone
does not enable that feature; the owner must enable it and confirm a report can be submitted.
See [GitHub private reporting instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

Use private reporting for exploit details, credentials, hidden case inputs, raw Human
evidence, sealed suites or private canaries. Public issues should contain only sanitized
details.

Include product, package version, OS/architecture, expected contract, observed verdict,
missing evidence, and a minimal synthetic reproduction. False PASS, missing-evidence
acceptance and disclosure of sealed runtime artifacts receive highest priority.
Acknowledgment and remediation timelines are best effort, not guaranteed.

## Threat boundary

BlindTest's Docker boundary isolates the target container: no network or host mounts,
read-only root, non-root user, dropped capabilities, resource bounds and recorded
inspect-before/after checks. The controller, image build and Human viewer remain trusted.
The target can see its own current input. Public synthetic fixture source is instructional;
it is not a private production oracle and must not be presented as unknown to an agent
that can read it.

Same-host-user unrestricted access, administrator/root access, Docker daemon or kernel
compromise and side channels are outside this protection. There is no perfect secrecy
claim. Docker Desktop's Linux VM does not establish native hardened Linux isolation on
macOS. Keep runtime sealed suites and full Human evidence outside target workspaces;
never upload entire private runtime stores.

Hashes are integrity checks, not authentication. A trusted host user able to rewrite
all files can also replace hashes. Publisher signing and authenticated provenance are
separate from evidence digests; see [release provenance](docs/RELEASE-PROVENANCE.md).
[Threat model](docs/THREAT-MODEL.md) and [evidence contract](docs/EVIDENCE.md) govern.

## Community support

Best-effort community support is provided through sanitized issues in the public
repository. No SLA or guaranteed response/fix time is offered, unless a future separate
commercial agreement explicitly provides one. No such agreement is part of this product.
