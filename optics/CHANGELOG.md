This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- added lens hierarchy: Iso, Lens, Prism, Getter, Review, AffineTraversal, AffineFold, Traversal, Fold, Setter
- added Rust-specific optics with (shared, mutable) references: GetterRef, GetterMut, AffineFoldRef, AffineFoldMut
- added full error handling capability to AffineFold, AffineFoldRef, AffineFoldMut, and Compose
- added helper macros:
  - "declare" family:
    - declare_lens_from_field
    - declare_lens
    - declare_prism_from_variant
    - declare_affine_traversal
  - "implement" family:
    - impl_lens
    - impl_affine_traversal
    - impl_up_from
  - "marker" family:
    - mark_fallible
    - mark_infallible
  - "composition" family:
    - optics
- added tuple field accessors (lens): _1, _2, _3, _4
- added enum accessors (prism): _Some, _Ok, _Err
- added generic optics: Identity, _Identity
  - Identity: polymorphic in source and view type
  - _Identity: explicit about source and view type (better type inference)
- added: (shared) references to optics are optics
