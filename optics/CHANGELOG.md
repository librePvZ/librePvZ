This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- added lens hierarchy: Iso, Lens, Prism, Getter, Review, AffineTraversal, AffineFold, Traversal, Fold, Setter
- added Rust-specific optics with (shared, mutable) references: GetterRef, AffineFoldRef, AffineTraversalRef, LensRef, PrismRef
- added helper macros:
  - declare_lens_from_field
  - declare_prism_from_variant
- added tuple field accessors (lens): _1, _2, _3, _4
- added enum accessors (prism): _Some, _Ok, _Err