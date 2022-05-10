/*
 * optics: yet another Haskell optics in Rust.
 * Copyright (c) 2022  Ruifeng Xie
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR $target PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Concrete implementations for optics.
use std::fmt::{Debug, Display, Formatter};
use crate::traits::*;

/// Composed optics of `K` and `L`; `K` is applied first.
#[derive(Copy, Clone)]
pub struct Compose<K, L>(pub K, pub L);

impl<K: Debug, L: Debug> Debug for Compose<K, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}.{:?}", self.0, self.1)
    }
}

impl<K: Display, L: Display> Display for Compose<K, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

impl<K: Optics<T>, L: Optics<K::View>, T> Optics<T> for Compose<K, L> {
    type View = L::View;
}

impl<K: AffineFold<T>, L: AffineFold<K::View>, T> AffineFold<T> for Compose<K, L> {
    fn preview(&self, s: T) -> Option<L::View> {
        self.1.preview(self.0.preview(s)?)
    }
}

impl<K: Getter<T>, L: Getter<K::View>, T> Getter<T> for Compose<K, L> {
    fn view(&self, s: T) -> L::View {
        self.1.view(self.0.view(s))
    }
}

impl<K: Review<T>, L: Review<K::View>, T> Review<T> for Compose<K, L> {
    fn review(&self, a: L::View) -> T {
        self.0.review(self.1.review(a))
    }
}

impl<K: Iso<T>, L: Iso<K::View>, T> Iso<T> for Compose<K, L> {}

impl<K: Setter<T>, L: Setter<K::View>, T> Setter<T> for Compose<K, L> {
    fn over(&self, s: &mut T, f: &mut dyn FnMut(&mut L::View)) {
        self.0.over(s, &mut |c| self.1.over(c, f))
    }
}

impl<K: Traversal<T>, L: Traversal<K::View>, T> Traversal<T> for Compose<K, L> {
    fn traverse(&self, s: T, f: &mut dyn FnMut(Self::View)) {
        self.0.traverse(s, &mut |v| self.1.traverse(v, f))
    }
}

impl<K: AffineTraversal<T>, L: AffineTraversal<K::View>, T> AffineTraversal<T> for Compose<K, L> {
    fn map(&self, s: &mut T, f: impl FnOnce(&mut L::View)) {
        self.0.map(s, |v| self.1.map(v, f))
    }
}

impl<K: Lens<T>, L: Lens<K::View>, T> Lens<T> for Compose<K, L> {}

impl<K: Prism<T>, L: Prism<K::View>, T> Prism<T> for Compose<K, L> {}

/// Easy composition for optics. See also [`Compose`].
///
/// ```
/// # use optics::optics;
/// # use optics::prelude::*;
/// # use optics::traits::AffineFold;
/// let val: Result<(bool, Option<u32>), &str> = Ok((true, Some(42)));
/// assert_eq!(optics!(_Ok._1._Some).preview(val), Some(42));
/// assert_eq!(format!("{}", optics!(_Ok._1._Some)), "Ok.1.Some".to_string());
/// assert_eq!(
///     format!("{:?}", optics!(_Ok._1._Some)),
///     "Result::Ok.{(T0, T1),(T0, T1, T2),(T0, T1, T2, T3)}::1.Option::Some".to_string()
/// );
/// ```
#[macro_export]
macro_rules! optics {
    ($single:ident) => { $single };
    ($head:ident . $($tail:ident).+) => {
        $crate::concrete::Compose($head, $crate::optics!($($tail).+))
    }
}

/// Declare a [`Lens`] from an accessor expression.
///
/// Since we have mutability built into the language, we can make use of it, and [`Getter`] plus
/// [`GetterRef`] gives us full capabilities of the [`Lens`] interface.
///
/// Here is an example usage. The `match` expression below is reused 3 times in [`Getter::view`],
/// [`GetterRef::view_ref`], and [`GetterRef::view_mut`]; this relies on the fact that (shared,
/// mutable) borrow of fields is implicit in `match` expressions.
///
/// ```
/// # use optics::declare_lens;
/// # use optics::traits::{Getter, GetterRef, Setter, AffineTraversal, AffineFold, AffineFoldRef};
/// #[derive(Debug, Copy, Clone, PartialEq)]
/// enum Color {
///     Rgba { red: f32, green: f32, blue: f32, alpha: f32 },
///     Hsla { hue: f32, saturation: f32, lightness: f32, alpha: f32 },
/// }
///
/// declare_lens! {
///     /// Alpha component for colors.
///     _Alpha as Color => f32,
///     (c) => match c {
///         Color::Rgba { alpha, .. } |
///         Color::Hsla { alpha, .. } => alpha,
///     }
/// }
///
/// let mut color = Color::Rgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };
/// // Getter(Ref)
/// assert_eq!(_Alpha.view(color), 1.0);
/// assert_eq!(_Alpha.view_ref(&color), &1.0);
/// assert_eq!(_Alpha.view_mut(&mut color), &mut 1.0);
/// // AffineFold(Ref)
/// assert_eq!(_Alpha.preview(color), Some(1.0));
/// assert_eq!(_Alpha.preview_ref(&color), Some(&1.0));
/// assert_eq!(_Alpha.preview_mut(&mut color), Some(&mut 1.0));
/// // Setter
/// _Alpha.over(&mut color, &mut |alpha| *alpha *= 0.5);
/// assert_eq!(color, Color::Rgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 0.5 });
/// // AffineTraversal
/// _Alpha.set(&mut color, 0.0);
/// assert_eq!(color, Color::Rgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 0.0 });
/// ```
///
/// It is also possible to define the normal "field accessor" lenses with this macro, but whenever
/// possible, one should prefer [`declare_lens_from_field`] for simplicity.
///
/// ```
/// # use optics::declare_lens;
/// # use optics::traits::{Getter, GetterRef, Setter, AffineTraversal, AffineFold, AffineFoldRef};
/// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// pub struct Foo { a: u32, b: bool, c: char }
///
/// declare_lens! {
///     /// Lens for `Foo::c`.
///     FooC as Foo => char, (foo) =>
///     by_val: foo.c,
///     by_ref: &foo.c,
///     by_mut: &mut foo.c,
/// }
///
/// let mut foo = Foo { a: 42, b: true, c: 'A' };
/// // Getter(Ref)
/// assert_eq!(FooC.view(foo), 'A');
/// assert_eq!(FooC.view_ref(&foo), &'A');
/// assert_eq!(FooC.view_mut(&mut foo), &mut 'A');
/// // AffineFold(Ref)
/// assert_eq!(FooC.preview(foo), Some('A'));
/// assert_eq!(FooC.preview_ref(&foo), Some(&'A'));
/// assert_eq!(FooC.preview_mut(&mut foo), Some(&mut 'A'));
/// // Setter
/// FooC.over(&mut foo, &mut |c| c.make_ascii_lowercase());
/// assert_eq!(foo, Foo { a: 42, b: true, c: 'a' });
/// // AffineTraversal
/// FooC.set(&mut foo, 'X');
/// assert_eq!(foo, Foo { a: 42, b: true, c: 'X' });
/// ```
///
/// To reuse the accessor expression to some extent, we can also use the `reused(wrap)` syntax:
/// ```
/// # use optics::declare_lens;
/// # pub struct Foo { a: u32, b: bool, c: char }
/// declare_lens! {
///     /// Lens for `Foo::c`.
///     FooC as Foo => char,
///     (foo) reused(wrap) => wrap!(foo.c)
/// }
/// ```
/// The expression `wrap!(x)` will be replaced by `x`, `&x`, and finally `&mut x`. One can also use
/// different identifiers other than `wrap`.
#[macro_export]
macro_rules! declare_lens {
    (
        $(#[$m:meta])* $vis:vis
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) => by_val: $by_val:expr, by_ref: $by_ref:expr, by_mut: $by_mut:expr $(,)?
    ) => {
        $(#[$m])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        $crate::impl_lens! {
            $name as $base => $target $(, for<$($p),+>)?,
            ($s) => by_val: $by_val, by_ref: $by_ref, by_mut: $by_mut
        }
    };
    (
        $(#[$m:meta])* $vis:vis
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) $(reused($wrap:ident))? => $reused:expr $(,)?
    ) => {
        $(#[$m])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        $crate::impl_lens! {
            $name as $base => $target $(, for<$($p),+>)?,
            ($s) $(reused($wrap))? => $reused
        }
    };
}

/// Similar to [`declare_lens`], but does not define the lens type for you.
#[macro_export]
macro_rules! impl_lens {
    (
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) => by_val: $by_val:expr, by_ref: $by_ref:expr, by_mut: $by_mut:expr $(,)?
    ) => {
        impl $(<$($p),+>)? $crate::traits::Optics<$base> for $name {
            type View = $target;
        }

        impl $(<$($p),+>)? $crate::traits::Getter<$base> for $name {
            fn view(&self, $s: $base) -> $target { $by_val }
        }

        impl $(<$($p),+>)? $crate::traits::GetterRef<$base> for $name {
            fn view_ref<'a>(&self, $s: &'a $base) -> &'a $target { $by_ref }
            fn view_mut<'a>(&self, $s: &'a mut $base) -> &'a mut $target { $by_mut }
        }

        $crate::impl_up_from!([GetterRef] $name as $base => $target $(, for<$($p),+>)?);

        impl $(<$($p),+>)? $crate::traits::Lens<$base> for $name {}
    };
    (
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) $(reused($wrap:ident))? => $reused:expr $(,)?
    ) => {
        $crate::impl_lens! { $name as $base => $target $(, for<$($p),+>)?, ($s) =>
            by_val: { $(macro_rules! $wrap { ($res:expr) => { $res } })? $reused },
            by_ref: { $(macro_rules! $wrap { ($res:expr) => { &$res } })? $reused },
            by_mut: { $(macro_rules! $wrap { ($res:expr) => { &mut $res } })? $reused },
        }
    };
}

/// Declare a [`Lens`] from a field name for a `struct`.
///
/// ```
/// # use optics::declare_lens_from_field;
/// # use optics::traits::{Getter, GetterRef, Setter, AffineTraversal, AffineFold, AffineFoldRef};
/// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// pub struct Foo { a: u32, b: bool, c: char }
/// declare_lens_from_field! {
///     /// Lens for `Foo::c`.
///     pub FooC for c as Foo => char
/// }
/// assert_eq!(format!("{:?}", FooC), "Foo::c".to_string());
/// assert_eq!(format!("{}", FooC), "c".to_string());
///
/// let mut foo = Foo { a: 42, b: true, c: 'A' };
/// // Getter(Ref)
/// assert_eq!(FooC.view(foo), 'A');
/// assert_eq!(FooC.view_ref(&foo), &'A');
/// assert_eq!(FooC.view_mut(&mut foo), &mut 'A');
/// // AffineFold(Ref)
/// assert_eq!(FooC.preview(foo), Some('A'));
/// assert_eq!(FooC.preview_ref(&foo), Some(&'A'));
/// assert_eq!(FooC.preview_mut(&mut foo), Some(&mut 'A'));
/// // Setter
/// FooC.over(&mut foo, &mut |c| c.make_ascii_lowercase());
/// assert_eq!(foo, Foo { a: 42, b: true, c: 'a' });
/// // AffineTraversal
/// FooC.set(&mut foo, 'X');
/// assert_eq!(foo, Foo { a: 42, b: true, c: 'X' });
/// ```
#[macro_export]
macro_rules! declare_lens_from_field {
    ($(
        $(#[$m:meta])* $vis:vis $name:ident for $field:tt
        $(as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?)+
    );+ $(;)?) => {$(
        $(#[$m])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                const BASES: &[&str] = &[$(stringify!($base)),+];
                if BASES.len() == 1 {
                    f.write_str(BASES[0])?;
                } else {
                    write!(f, "{{{}", BASES[0])?;
                    for base in BASES.iter().skip(1).take(2) {
                        write!(f, ",{}", base)?;
                    }
                    if BASES.len() > 3 { write!(f, ",..")? }
                    write!(f, "}}")?;
                }
                write!(f, "::{}", stringify!($field))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(stringify!($field))
            }
        }

        $(
            $crate::impl_lens! {
                $name as $base => $target $(, for<$($p),+>)?,
                (s) reused(wrap) => wrap!(s.$field)
            }
        )+
    )+}
}

/// Declare a [`Prism`] from a variant name for an `enum`.
///
/// **Note:** Only tuple-like variant with exactly one field is supported. For unsupported `enum`s,
/// use [`declare_affine_traversal`], and implement [`Review`] and [`Prism`] manually: the former
/// specifies how to construct an instance of that `enum` from this variant, and the latter is only
/// a marker trait.
///
/// ```
/// # use optics::declare_prism_from_variant;
/// # use optics::traits::{Setter, AffineTraversal, AffineFold, AffineFoldRef, Review};
/// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// pub enum Foo {
///     Int(u32),
///     Char(char),
/// }
/// declare_prism_from_variant! {
///     /// Prism for `Foo::Char`.
///     pub FooChar for Char as Foo => char
/// }
/// assert_eq!(format!("{:?}", FooChar), "Foo::Char".to_string());
/// assert_eq!(format!("{}", FooChar), "Char".to_string());
///
/// let mut foo = Foo::Char('A');
/// let mut bar = Foo::Int(42);
/// // AffineFold(Ref)
/// assert_eq!(FooChar.preview(foo), Some('A'));
/// assert_eq!(FooChar.preview_ref(&foo), Some(&'A'));
/// assert_eq!(FooChar.preview_mut(&mut foo), Some(&mut 'A'));
/// assert_eq!(FooChar.preview(bar), None);
/// assert_eq!(FooChar.preview_ref(&bar), None);
/// assert_eq!(FooChar.preview_mut(&mut bar), None);
/// // Setter
/// FooChar.over(&mut foo, &mut |c| c.make_ascii_lowercase());
/// assert_eq!(foo, Foo::Char('a'));
/// FooChar.over(&mut bar, &mut |c| c.make_ascii_lowercase());
/// assert_eq!(bar, Foo::Int(42));
/// // AffineTraversal
/// FooChar.set(&mut foo, 'X');
/// assert_eq!(foo, Foo::Char('X'));
/// FooChar.set(&mut bar, 'X');
/// assert_eq!(bar, Foo::Int(42));
/// ```
#[macro_export]
macro_rules! declare_prism_from_variant {
    ($(
        $(#[$m:meta])* $vis:vis $name:ident for $variant:tt
        as $base:ident $(<$($p1:ident),+ $(,)?>)? => $target:ty
        $(, for <$($p:ident),+ $(,)?>)?
    );+ $(;)?) => {$(
        $(#[$m])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}::{}", stringify!($base), stringify!($variant))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(stringify!($variant))
            }
        }

        $crate::impl_affine_traversal! {
            $name as $base $(<$($p1),+>)? => $target $(, for<$($p),+>)?,
            (s) => if let $base::$variant(x) = s { Some(x) } else { None }
        }

        impl $(<$($p),+>)? $crate::traits::Review<$base $(<$($p1),+>)?> for $name {
            fn review(&self, a: $target) -> $base $(<$($p1),+>)? { $base::$variant(a) }
        }

        impl $(<$($p),+>)? $crate::traits::Prism<$base $(<$($p1),+>)?> for $name {}
    )+}
}

/// Declare an [`AffineTraversal`] from an accessor expression.
///
/// Normally we obtain [`AffineTraversal`]s by composing [`Lens`]es and [`Prism`]s. However, due to
/// the ownership system in Rust, and the lack of enum variants as standalone types, it is generally
/// difficult to define prisms for enum variants with more than one field.
///
/// Use this macro (as a workaround for the said problem) to directly define [`AffineTraversal`]s.
///
/// ```
/// # use optics::declare_affine_traversal;
/// # use optics::traits::{Getter, GetterRef, Setter, AffineTraversal, AffineFold, AffineFoldRef};
/// #[derive(Debug, Copy, Clone, PartialEq)]
/// enum Color {
///     Rgba { red: f32, green: f32, blue: f32, alpha: f32 },
///     Hsla { hue: f32, saturation: f32, lightness: f32, alpha: f32 },
/// }
///
/// declare_affine_traversal! {
///     /// Alpha component for colors.
///     _Green as Color => f32,
///     (c) => match c {
///         Color::Rgba { green, .. } => Some(green),
///         _ => None,
///     }
/// }
///
/// let mut rgba = Color::Rgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };
/// let mut hsla = Color::Hsla { hue: 1.0, saturation: 1.0, lightness: 1.0, alpha: 1.0 };
/// // AffineFold(Ref)
/// assert_eq!(_Green.preview(rgba), Some(1.0));
/// assert_eq!(_Green.preview_ref(&rgba), Some(&1.0));
/// assert_eq!(_Green.preview_mut(&mut rgba), Some(&mut 1.0));
/// assert_eq!(_Green.preview(hsla), None);
/// assert_eq!(_Green.preview_ref(&hsla), None);
/// assert_eq!(_Green.preview_mut(&mut hsla), None);
/// // Setter
/// _Green.over(&mut rgba, &mut |green| *green *= 0.5);
/// assert_eq!(rgba, Color::Rgba { red: 1.0, green: 0.5, blue: 1.0, alpha: 1.0 });
/// _Green.over(&mut hsla, &mut |green| *green *= 0.5);
/// assert_eq!(hsla, Color::Hsla { hue: 1.0, saturation: 1.0, lightness: 1.0, alpha: 1.0 });
/// // AffineTraversal
/// _Green.set(&mut rgba, 0.0);
/// assert_eq!(rgba, Color::Rgba { red: 1.0, green: 0.0, blue: 1.0, alpha: 1.0 });
/// _Green.set(&mut hsla, 0.0);
/// assert_eq!(hsla, Color::Hsla { hue: 1.0, saturation: 1.0, lightness: 1.0, alpha: 1.0 });
/// ```
#[macro_export]
macro_rules! declare_affine_traversal {
    (
        $(#[$m:meta])* $vis:vis
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) => by_val: $by_val:expr, by_ref: $by_ref:expr, by_mut: $by_mut:expr $(,)?
    ) => {
        $(#[$m])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        $crate::impl_affine_traversal! {
            $name as $base => $target $(, for<$($p),+>)?,
            ($s) => by_val: $by_val, by_ref: $by_ref, by_mut: $by_mut
        }
    };
    (
        $(#[$m:meta])* $vis:vis
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) $(reused($wrap:ident))? => $reused:expr $(,)?
    ) => {
        $(#[$m])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        $crate::impl_affine_traversal! {
            $name as $base => $target $(, for<$($p),+>)?,
            ($s) $(reused($wrap))? => $reused
        }
    };
}

/// Similar to [`declare_affine_traversal`], but does not define the lens type for you.
#[macro_export]
macro_rules! impl_affine_traversal {
    (
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) => by_val: $by_val:expr, by_ref: $by_ref:expr, by_mut: $by_mut:expr $(,)?
    ) => {
        impl $(<$($p),+>)? $crate::traits::Optics<$base> for $name {
            type View = $target;
        }

        impl $(<$($p),+>)? $crate::traits::AffineFold<$base> for $name {
            fn preview(&self, $s: $base) -> Option<$target> { $by_val }
        }

        impl $(<$($p),+>)? $crate::traits::AffineFoldRef<$base> for $name {
            fn preview_ref<'a>(&self, $s: &'a $base) -> Option<&'a $target> { $by_ref }
            fn preview_mut<'a>(&self, $s: &'a mut $base) -> Option<&'a mut $target> { $by_mut }
        }

        $crate::impl_up_from!([AffineFoldRef] $name as $base => $target $(, for<$($p),+>)?);
    };
    (
        $name:ident as $base:ty => $target:ty $(, for<$($p:ident),+ $(,)?>)?,
        ($s:ident) $(reused($wrap:ident))? => $reused:expr $(,)?
    ) => {
        $crate::impl_affine_traversal! {
            $name as $base => $target $(, for<$($p),+>)?, ($s) =>
            by_val: { $(macro_rules! $wrap { ($res:expr) => { $res } })? $reused },
            by_ref: { $(macro_rules! $wrap { ($res:expr) => { &$res } })? $reused },
            by_mut: { $(macro_rules! $wrap { ($res:expr) => { &mut $res } })? $reused },
        }
    };
}

/// Implement the lens hierarchy from some specific level.
///
/// | Base Trait                                       | Free Implementation                |
/// |--------------------------------------------------|------------------------------------|
/// | [`AffineTraversal`]                              | [`Setter`]                         |
/// | [`AffineFoldRef`] (and therefore [`AffineFold`]) | [`Traversal`], [`AffineTraversal`] |
/// | [`GetterRef`] (and therefore [`Getter`])         | [`AffineFold`], [`AffineFoldRef`]  |
///
/// Call this in the following form:
/// ```ignore
/// impl_up_from! {
///     [BaseTrait]
///     OpticsName as BaseType => TargetType,
///     for<... generic parameters ...>;
///     // ... and perhaps more lines like this
/// }
/// ```
#[macro_export]
macro_rules! impl_up_from {
    // AffineTraversal => Setter
    (
        [AffineTraversal]
        $(
            $name:ident as $base:ty => $target:ty
            $(, for <$($p:ident),+ $(,)?>)?
        );+ $(;)?
    ) => {$(
        impl $(<$($p),+>)? $crate::traits::Setter<$base> for $name {
            fn over(&self, s: &mut $base, f: &mut dyn FnMut(&mut $target)) {
                $crate::traits::AffineTraversal::map(self, s, f)
            }
        }
    )+};
    // AffineFold(Ref) => Traversal, AffineTraversal
    (
        [AffineFoldRef]
        $(
            $name:ident as $base:ty => $target:ty
            $(, for <$($p:ident),+ $(,)?>)?
        );+ $(;)?
    ) => {$(
        impl $(<$($p),+>)? $crate::traits::Traversal<$base> for $name {
            fn traverse(&self, s: $base, f: &mut dyn FnMut($target)) {
                $crate::traits::AffineFold::preview(self, s).map(f);
            }
        }

        impl $(<$($p),+>)? $crate::traits::AffineTraversal<$base> for $name {
            fn map(&self, s: &mut $base, f: impl FnOnce(&mut $target)) {
                $crate::traits::AffineFoldRef::preview_mut(self, s).map(f);
            }
        }

        $crate::impl_up_from!([AffineTraversal] $name as $base => $target $(, for<$($p),+>)?);
    )+};
    // GetterRef => AffineFold, AffineFoldRef
    (
        [GetterRef]
        $(
            $name:ident as $base:ty => $target:ty
            $(, for <$($p:ident),+ $(,)?>)?
        );+ $(;)?
    ) => {$(
        impl $(<$($p),+>)? $crate::traits::AffineFold<$base> for $name {
            fn preview(&self, s: $base) -> Option<$target> {
                Some($crate::traits::Getter::view(self, s))
            }
        }

        impl $(<$($p),+>)? $crate::traits::AffineFoldRef<$base> for $name {
            fn preview_ref<'a>(&self, s: &'a $base) -> Option<&'a $target> {
                Some($crate::traits::GetterRef::view_ref(self, s))
            }
            fn preview_mut<'a>(&self, s: &'a mut $base) -> Option<&'a mut $target> {
                Some($crate::traits::GetterRef::view_mut(self, s))
            }
        }

        $crate::impl_up_from!([AffineFoldRef] $name as $base => $target $(, for<$($p),+>)?);
    )+};
}