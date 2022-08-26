This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- add `TwoStageAsset` and related facilities
- add `Cached` mechanism for serializing as names/paths and cache the in-memory shortcut access (integer indices into arrays, or handles into bevy assets)
- add `AnyResource` and `DynamicResource` for dynamic (de)serialization
- defined `Animation` structure
- defined `Model` structure
- defined macro `cache_known_states` for robust cached translation between state names and state indices
- support (de)serialization with [`bincode`](https://github.com/bincode-org/bincode)
- implement `serde` (de)serialization for use in JSON/YAML
- implement `TwoStageAsset` for animations
