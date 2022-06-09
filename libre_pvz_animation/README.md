# librePvZ Animation

> **Note in advance:** below whenever we point out any drawbacks on the "current approach" or the "design in the RFC", we actually mean specifically our own understanding and implementation of the RFC. It is perfectly possible (and even likely) that the RFC actually covers the topics below.

Previously, we mostly copied the design in the RFC [Bevy animation primitives](https://github.com/james7132/rfcs/blob/animation-primitives/rfcs/49-animation-primitives.md) (see also the reference implementation [`bevy_prototype_animation`](https://github.com/HouraiTeahouse/bevy_prototype_animation)). It turns out the design there does not fit exactly in our use case here:

- _Switching animations means rebinding curves._ An animation player is attached to some entity, and then every curve in the clip gets bound to its corresponding child entity, so that we can avoid repeatedly performing the search. However, the binding is invalidated by switching the animation clip, which happens very often in PvZ (because plants/zombies need to transition among different states).
- _Animation blending is a non-trivial task._ Blending means smoothly transition from one animation to another, playing a mixture of both animations during the transition. However, in current design, before we can play an animation, it needs to be compiled into a clip in advance. Therefore, to blend two animations (each from a specific timestamp), we must generate transient clips again and again (and there are infinitely many of them, since we might want to perform blending at any arbitrary time).
- _We need finer-grained control over the animation playing process._ Animation primitives we want here resembles those of the good old ActionScript 2.0 in Adobe Flash, e.g. to play a segment and then stop, or to loop a segment. We share some invariants with Flash animations:
  - Tracks (layers) will always exist, even if we switch from one segment to another;
  - Animations are based on keyframes, and we have a fixed framerate for animations (this does not imply the animation will be played under the same fixed framerate);
  - There are only finitely many possible types of components that we want to animate;

So we decide to adopt the Flash animation model here. Animations are stored as a series of frames at some pre-determined framerate. Segments are stored as a pair of frame indices (left inclusive, right exclusive). Looping etc. becomes a property of the animation player, without requiring special adjustment of the animation (previously, a looping animation looks correct only if it has the same starting and ending frame).
