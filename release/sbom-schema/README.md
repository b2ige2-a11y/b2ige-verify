# CycloneDX schema validation

These unmodified CycloneDX 1.5 JSON schemas are distributed under Apache-2.0,
as stated in their upstream `$comment` fields. See the project [LICENSE](../../LICENSE).
[Exact retrieval URLs and SHA-256](sources.json) bind the local validation inputs.
Source: [CycloneDX specification 1.5](https://github.com/CycloneDX/specification/tree/1.5/schema).

Release tooling uses `jsonschema==4.25.1` in an isolated Python environment.
The validator loads schema references locally; it does not resolve remote schemas.
This tooling is separate from the Rust verifier and the dependency-free npm launcher.
