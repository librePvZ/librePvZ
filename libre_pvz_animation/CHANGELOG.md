This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

- Curves:
  - added Curve: main interface
  - added ConstCurve: curves with constant value
  - added Track, FrameIndex, TrackContent: concrete implementation
  - added CurveBuilder, TrackContentBuilder: convenience API
- Animations:
  - added AnimationClip: representation
  - added AnimationPlayer, AnimationPlugin: worker
- added optics-based "path" for locating attributes to be animated
- added Reflect-based "path" for locating attributes to be animated
- added Transform2d: full-fledged affine transformation
  - use in place of normal local Transform in Bevy
  - require Bevy fork with GlobalTransform being Affine3A
