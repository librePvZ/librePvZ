This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- Curves:
  - added Curve: main interface
  - added Track, FrameIndex, TrackContent: concrete implementation
  - added CurveBuilder, TrackContentBuilder: convenience API
- Animations:
  - added AnimationClip: representation
  - added AnimationPlayer, AnimationPlugin: worker
- added optics-based "path" for locating attributes to be animated
- added Reflect-based "path" for locating attributes to be animated
