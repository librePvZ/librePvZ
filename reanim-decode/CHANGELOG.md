This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- added support for the following formats (⚠️ means supported but not via the CLI):

| Format               | Extension          | R   | W   |
|----------------------|--------------------|-----|-----|
| compressed?, binary  | `.reanim.compiled` | ✅   |     |
| reanim XML           | `.reanim`          |     | ✅   |
| reanim JSON          | `.json`            | ⚠️  | ✅   |
| reanim YAML          | `.yaml`            | ⚠️  | ✅   |
| structural `bincode` | `.anim`            | ⚠️  | ✅   |
| structural JSON      | `.packed.json`     | ⚠️  | ✅   |
| structural YAML      | `.packed.yaml`     | ⚠️  | ✅   |
