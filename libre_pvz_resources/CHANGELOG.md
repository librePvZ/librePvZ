This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- add `TwoStageAssetLoader` and related facilities
- defined `Animation` structure
- defined `Model` structure
- support (de)serialization with [`bincode`](https://github.com/bincode-org/bincode)
- implement `serde` (de)serialization for use in JSON/YAML
- implement `TwoStageAssetLoader` for animations
- add `Cached` mechanism for serializing as names/paths and cache the in-memory shortcut access (integer indices into arrays, or handles into bevy assets)
