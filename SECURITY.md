# Security and support

## Supported versions

| Version | Security maintenance |
|---|---|
| 0.1.x | Latest patch only, after the first authorized public release |
| Other versions | No maintenance commitment |

0.1.0 is currently an unpublished candidate. No public release exists yet. Report
older-patch issues against the latest patch when available. No backport, fixed support
lifetime or guaranteed remediation deadline is promised. Package and evidence schema
versions are independent. Support is best-effort community support with no SLA.

## Private vulnerability reports

**NOT ACTIVE YET: Enable GitHub Private Vulnerability Reporting after public conversion**

The repository is still private, so the owner has not configured or verified the public
private-reporting channel. **This is a publication blocker, not a technical blocker.**
No security email or repository URL is invented here. Immediately after the repository
becomes public, enable GitHub private vulnerability reporting and use its
**Security → Advisories → Report a vulnerability** action. A SECURITY.md file alone
does not enable that feature; the owner must enable it and confirm a report can be submitted.
See [GitHub private reporting instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

Until that channel is confirmed, use an existing private maintainer channel, if you
already have one. Otherwise request a private contact using only a sanitized public
issue after the repository exists. Never include exploit details, credentials, hidden
case inputs, raw Human evidence, sealed suites or private canaries in public issues.

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

Best-effort community support is planned through sanitized issues in the eventual
public repository. No SLA or guaranteed response/fix time is offered, unless a future
separate commercial agreement explicitly provides one. No such agreement is part of 0.1.0.
