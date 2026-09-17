# Status CLI

Small existing-project archetype with conventional metadata and a source tree.
The `base` command preserves the reviewed P8 reference output `same`, except
the Node candidate deliberately returns `changed` (P8 behavior.stdout).
The runner compiles Rust with rustc; Node/Python use their installed runtime
through a generated local launcher. No package download is needed.
