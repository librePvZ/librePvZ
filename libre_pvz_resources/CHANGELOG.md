This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- defined `Animation` structure
- defined `Model` structure
- support (de)serialization with [`bincode`](https://github.com/bincode-org/bincode)
- implement `serde` (de)serialization for use in JSON/YAML

Feature `bevy`:
- implement `AssetLoader` for animations
