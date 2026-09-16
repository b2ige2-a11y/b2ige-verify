# Agent distribution — B2IGE Verify 0.1.0

Don't trust done. Prove it.

Independent deterministic verification for AI-written software. This distribution
reuses the existing [skill](../skills/b2ige-verify/SKILL.md) and **unchanged** native
MCP binaries from the [public v0.1.0 release](https://github.com/b2ige2-a11y/b2ige-verify/releases/tag/v0.1.0).
No product compilation, npm publication, hosted server, or new verifier semantics.

**Status:** local assets prepared; no push, release upload, registry publication or
directory submission. The MCPB download URL in `server.json` is reserved and will not
work until the owner adds the new asset to the existing release. Preserve that release's
tag, existing binaries, manifests, and original `SHA256SUMS`.

## Shared assets

```text
plugin.json                         Cursor / portable Agent Plugins 1.0.0
.codex-plugin/plugin.json           Codex metadata, Developer Tools
skills/b2ige-verify/                 one shared skill, no duplicated variant
server.json                         Official MCP Registry package declaration
distribution/
  mcpb/manifest.json                MCPB 0.4, binary server over local stdio
  mcpb/launch.sh                    strict OS/architecture dispatch
  mcpb/README.md                    bundled operator setup
  release-inputs.json               pinned v0.1.0 archive/manifest/binary hashes
  build_mcpb.py                     download, verify, pack; never compile
  test_distribution.py              packaging/launcher/native stdio checks only
  bundle.sha256                     checksum of the final normalized MCPB
  metadata.md                       copyable directory listing
  README.md                         setup and submission checklist
release/artifacts/agent-distribution/  generated locally, git-ignored
  b2ige-verify-0.1.0.mcpb
  b2ige-verify-0.1.0.mcpb.sha256
```

## Install and configure locally

The plugins provide a skill, not an automatically provisioned verifier. Neither root
`mcp.json` nor `.mcp.json` is shipped: there is no portable trusted registry location,
and an absent PATH binary must not be assumed. Configure the already verified MCPB
explicitly using the same launcher in Cursor and Codex. No network bootstrap occurs
on plugin load or MCP startup. Use the original release archive if CLI tools are needed;
this MCPB includes only `b2ige-mcp`, not `b2ige` or benchmark/demo helpers.

1. Obtain the MCPB and its checksum through a trusted channel. Before public upload,
   use `release/artifacts/agent-distribution/` from the local packaging run. After upload:

   ```sh
   curl --fail --location --output b2ige-verify-0.1.0.mcpb \
     https://github.com/b2ige2-a11y/b2ige-verify/releases/download/v0.1.0/b2ige-verify-0.1.0.mcpb
   # Run in the directory containing the downloaded MCPB; point at this repo's checksum.
   shasum -a 256 -c /absolute/path/to/b2ige-verify/distribution/bundle.sha256
   ```

2. Import it using an MCPB-capable client's local bundle installer, or extract the
   **verified** ZIP into a new, trusted installation directory with `unzip` (which
   preserves executable permissions). Do not use the coding agent's workspace:

   ```sh
   unzip b2ige-verify-0.1.0.mcpb -d /absolute/trusted/b2ige-verify-0.1.0
   ```

3. A trusted operator prepares the registry as described in [MCP setup](../docs/MCP.md).
   Use absolute paths throughout configs, authorizations, targets and storage. Supply
   `registry` in MCPB configuration. For BlindTest, also supply `sealed_root`, approved
   suites/immutable target images and a local Docker Linux engine. Empty sealed-root
   configuration is allowed for BehaviorSeal/SideEffect and grants no BlindTest readiness.
4. For Cursor's manual MCP settings, merge this entry into the user's chosen
   `mcpServers` object after replacing the example absolute paths:

   ```json
   {
     "mcpServers": {
       "b2ige-verify": {
         "command": "/bin/sh",
         "args": ["/absolute/trusted/b2ige-verify-0.1.0/launch.sh", "--registry", "/absolute/trusted/project.json"]
       }
     }
   }
   ```

   For BlindTest add `env` with `B2IGE_BLINDTEST_SEALED_ROOT` set to the operator's
   protected absolute sealed directory. Do not add this to an agent-editable project
   when the deployment claims hidden-suite secrecy.

   For Codex, register the same launcher using the existing CLI adapter:

   ```sh
   codex mcp add b2ige-verify -- /bin/sh /absolute/trusted/b2ige-verify-0.1.0/launch.sh --registry /absolute/trusted/project.json
   # For BlindTest, instead register with the protected sealed-root environment:
   codex mcp add b2ige-verify --env B2IGE_BLINDTEST_SEALED_ROOT=/absolute/trusted/sealed -- /bin/sh /absolute/trusted/b2ige-verify-0.1.0/launch.sh --registry /absolute/trusted/project.json
   ```

Supported targets: darwin arm64, darwin x86_64, linux x86_64 GNU libc (release-tested
Ubuntu 22.04). No Windows, Linux arm64 or musl/Alpine support. MCPB's OS filter cannot
express architecture; the launcher enforces the three target combinations. macOS
binaries and this bundle remain unsigned/unnotarized; no OS-protection bypass is included.

Installation, plugin selection and `doctor` are not verification. Only a verifier's
evidence-backed PASS permits completion in its declared tested scope. Preserve baseline
approval and [hidden-suite isolation](../docs/THREAT-MODEL.md); an unrestricted same-user
process can read controller files, so a local plugin alone cannot make them secret.

## Rebuild and validate distribution only

Requires Python 3.10+, Node/npm for the **packaging tool only**, and a Unix packaging host.
Do not run npm publish or rebuild Rust. Install tools into a disposable directory:

```sh
npm install --prefix /tmp/b2ige-distribution-tools --no-audit --no-fund @anthropic-ai/mcpb@2.1.2
python3 distribution/build_mcpb.py --mcpb /tmp/b2ige-distribution-tools/node_modules/.bin/mcpb
python3 distribution/test_distribution.py -v
sh -n distribution/mcpb/launch.sh
mcp-publisher validate server.json
git diff --check
```

The builder verifies pinned release SHA256SUMS, archive and manifest hashes, release
commit/target identities, and each extracted binary hash. It reads only exact regular
tar members; it does not extract arbitrary archive paths. It copies release license and
security notices, runs official `mcpb validate` and `mcpb pack`, then canonicalizes ZIP
timestamps/order/permissions. Final ZIP payloads are rechecked and its manifest is
validated again. Its SHA-256 is the hash of the **final** bundle, not the pack tool's
intermediate SHA-1. Packaging twice was byte-identical with the recorded toolchain.

Outputs are never overwritten. For another build choose a fresh `--output` directory;
`--cache` can reuse already verified downloads. An intentional packaging change requires
reviewing the resulting hash and updating both `distribution/bundle.sha256` and
`server.json` together. The builder never silently changes the registry declaration.

JSON/schema validation targets the official Agent Plugins 1.0.0 schema, MCPB 0.4 schema
shipped with CLI 2.1.2, and Registry 2025-12-11 schema. Codex validation uses the installed
`plugin-creator/scripts/validate_plugin.py` and `skill-creator/scripts/quick_validate.py`
with PyYAML; those tooling paths depend on the maintainer's Codex installation.
See [P11 validation record](../P11-RESULT.md) for exact results and limitations.

## Format and version decisions

- Cursor: root `plugin.json` uses the vendor-neutral Agent Plugins 1.0.0 format. Only
  skills and optional separately configured MCP are needed; Cursor-specific rules,
  hooks, variables and a `.cursor-plugin/` manifest add no necessary capability.
  Fixed `skills/` discovery reuses the existing skill. This also avoids Cursor's
  incompatibility with standard `${PLUGIN_ROOT}` / `${PLUGIN_DATA}` interpolation.
- Codex: required `.codex-plugin/plugin.json`, same skill tree, no hosted connector.
  No personal marketplace entries or user client settings were changed.
- MCPB: CLI 2.1.2 identifies 0.4 as its latest supported manifest schema. The upstream
  MANIFEST.md header still says 0.3, while its body documents 0.4. This bundle validates
  against 0.4 and uses binary/stdio features without a Python/Node runtime requirement.
- Distribution/plugin version is 0.1.0, using product v0.1.0. External manifest versions
  are independent. Product schemas, protocol version, verdict rules, and baselines have
  **no changes and no version bump**.

## Submission checklist and owner actions

Cursor preflight: valid Agent Plugin manifest; valid skill frontmatter and in-root
paths; shared discoverability description includes all six requested user intents;
Apache-2.0 and public repository provided. Logo is optional and omitted. Root README
already documents CLI use and links the MCP/skill documentation; plugin setup is in
this guide and linked from the skill. Root README is unchanged. **A live Cursor import
and intent-selection check is still required before calling the complete Marketplace
checklist passed.** Global name availability and reviewer acceptance are not guaranteed.

Codex preflight: installed official skill validator accepts manifest and skill; category
is Developer Tools and product display name is B2IGE Verify. Local manifest-ready,
but no fresh-client installation or public directory acceptance is claimed. The current
OpenAI public submission guide requires contacting OpenAI before submitting workflows
whose core value needs local execution/file access. A skills-only package can use the
skills upload path, but that does not establish approval for this local verifier's
execution model. Do not fabricate a hosted endpoint to satisfy the remote MCP path.

Only the owner should perform the following (none performed by this task):

| Platform | Remaining action |
|---|---|
| GitHub prerequisite | Push reviewed P11 commit. Add the MCPB and its separately named `.mcpb.sha256` to the existing v0.1.0 release; never replace its tag/assets/original SHA256SUMS. Download the uploaded MCPB and recheck its recorded hash. |
| Official MCP Registry | Authenticate as `b2ige2-a11y` via `mcp-publisher login github`, validate the now-reachable package and publish `server.json`. |
| Cursor Marketplace | Import/test in a current Cursor client (including a missing-verifier setup and the six discovery intents), then submit the public repository at <https://cursor.com/marketplace/publish>. |
| cursor.directory | Sign in at <https://cursor.directory/plugins/new?type=mcp_server>; paste `metadata.md`, repository and released bundle information. |
| Smithery | Choose **Local (MCPB Bundle)**, supply the same verified MCPB and metadata; use the authenticated bundle publication flow, not a fabricated HTTP service. |
| OpenAI Codex | Contact OpenAI about local execution support/review first. After the local-only distribution path is confirmed, test a fresh install and complete the approved package submission at <https://platform.openai.com/plugins>. Do not submit a nonexistent remote MCP endpoint. |

## Official references checked 2026-09-16

- [MCPB specification](https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md),
  [CLI](https://github.com/modelcontextprotocol/mcpb/blob/main/CLI.md)
- [Official Registry MCPB package rules](https://github.com/modelcontextprotocol/registry/blob/main/docs/modelcontextprotocol-io/package-types.mdx),
  [schema](https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json),
  [publisher validation](https://github.com/modelcontextprotocol/registry/blob/main/docs/reference/cli/commands.md)
- [Cursor formats and submission checklist](https://cursor.com/docs/reference/plugins),
  [Agent Plugins specification](https://agent-plugins.org/specification),
  [manifest schema](https://agent-plugins.org/schemas/1.0.0/plugin.schema.json)
- [Smithery local bundle publishing](https://smithery.ai/docs/build/publish)
- Codex: installed official `plugin-creator` skill manifest reference and validator.
- [OpenAI public submission paths and local-execution review](https://developers.openai.com/plugins/guides/submit-claude-plugin)
