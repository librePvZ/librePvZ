This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- animation: added support for the following formats (with the exception that structural formats cannot be converted to reanim formats):

| Format               | Extension          | R   | W   |
|----------------------|--------------------|-----|-----|
| compressed?, binary  | `.reanim.compiled` | ✅   |     |
| reanim XML           | `.reanim`          |     | ✅   |
| reanim JSON          | `.json`            | ✅   | ✅   |
| reanim YAML          | `.yaml`            | ✅   | ✅   |
| structural `bincode` | `.anim`            | ✅   | ✅   |
| structural JSON      | `.packed.json`     | ✅   | ✅   |
| structural YAML      | `.packed.yaml`     | ✅   | ✅   |

- model: added animation model description, available in the following formats:

| Format    | Extension |
|-----------|-----------|
| `bincode` | `.model`  |
| JSON      | `.json`   |
| YAML      | `.yaml`   |

