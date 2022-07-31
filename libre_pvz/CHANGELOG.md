This project adheres to [Cargo's Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## Unreleased

### Almanac Scene

> Disabled temporarily in the update to Bevy 0.8. We should migrate the `BoundingBoxPlugin` (and therefore re-enable this scene) as soon as `bevy_prototype_lyon` catches up and release a new version. 

- support previewing animations (default meta track "anim_idle")
- GUI panels for controlling preview behaviour:
  - show (average) frame rate for diagnostics
  - show all meta tracks for selection
  - allow adjusting scale & speed
  - allow pausing and adjusting the progress
  - show bounding boxes for diagnostics

### Lawn Scene

- show day lawn, set up a grid system on the lawn
- show a hard-coded Repeater on the lawn
