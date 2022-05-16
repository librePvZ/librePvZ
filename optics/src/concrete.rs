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
use std::marker::PhantomData;
use derivative::Derivative;
use crate::traits::*;

crate::declare_lens! {
    /// Identity optics.
    #[derive(Debug)]
    pub Identity as T => T, for<T>, (x) => x
}

impl<T> Review<T> for Identity {
    fn review(&self, a: T) -> T { a }
}

impl<T> Prism<T> for Identity {}

impl<T> Iso<T> for Identity {}

impl Display for Identity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("self")
    }
}

/// Identity optics, explicit about source and view types.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derivative(Copy(bound = ""), Clone(bound = ""))]
#[derivative(Eq(bound = ""), PartialEq(bound = ""))]
pub struct _Identity<T: ?Sized>(PhantomData<fn() -> T>);

impl<T: ?Sized> Debug for _Identity<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "_Identity::<{}>", std::any::type_name::<T>())
    }
}

impl<T: ?Sized> Display for _Identity<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "self<{}>", std::any::type_name::<T>())
    }
}

impl<T: ?Sized> Optics<T> for _Identity<T> { type View = T; }

impl<T: ?Sized> OpticsFallible for _Identity<T> {
    type Success = _Identity<T>;
    type Error = _Identity<T>;
    fn success_witness(&self) -> _Identity<T> { _Identity::default() }
}

impl<T> Getter<T> for _Identity<T> {
    fn view(&self, s: T) -> T { s }
}

impl<'a, T: ?Sized + 'a> GetterRef<'a, T> for _Identity<T> {
    fn view_ref(&self, s: &'a T) -> &'a T { s }
}

impl<'a, T: ?Sized + 'a> GetterMut<'a, T> for _Identity<T> {
    fn view_mut(&self, s: &'a mut T) -> &'a mut T { s }
}

impl<T> AffineFold<T> for _Identity<T> {
    fn preview(&self, s: T) -> Result<T, Self::Error> { Ok(s) }
}

impl<'a, T: ?Sized + 'a> AffineFoldRef<'a, T> for _Identity<T> {
    fn preview_ref(&self, s: &'a T) -> Result<&'a T, Self::Error> { Ok(s) }
}

impl<'a, T: ?Sized + 'a> AffineFoldMut<'a, T> for _Identity<T> {
    fn preview_mut(&self, s: &'a mut T) -> Result<&'a mut T, Self::Error> { Ok(s) }
}

impl<T> Review<T> for _Identity<T> {
    fn review(&self, a: T) -> T { a }
}

impl<T> AffineTraversal<T> for _Identity<T> {
    fn map(&self, s: &mut T, f: impl FnOnce(&mut T)) { f(s) }
}

impl<T> Traversal<T> for _Identity<T> {
    fn traverse(&self, s: T, f: &mut dyn FnMut(T)) { f(s) }
}

impl<T> Setter<T> for _Identity<T> {
    fn over(&self, s: &mut T, f: &mut dyn FnMut(&mut T)) { f(s) }
}

impl<T> Lens<T> for _Identity<T> {}

impl<T> Prism<T> for _Identity<T> {}

impl<T> Iso<T> for _Identity<T> {}

/// Success type for [`Compose`]d optics.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SuccessCompose<S, R>(S, R);

impl<S: Display, R: Display> Display for SuccessCompose<S, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

/// Error type for [`Compose`]d optics.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorCompose<E, S, R> {
    /// Error happened for the first optics in this composition.
    Head(E),
    /// Operation for the first optics succeeded in this composition.
    Tail(S, R),
}

impl<E: Display, S: Display, R: Display> Display for ErrorCompose<E, S, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCompose::Head(x) => x.fmt(f),
            ErrorCompose::Tail(x, r) => write!(f, "{x}.{r}"),
        }
    }
}

impl<E, S, R> From<E> for ErrorCompose<E, S, R> {
    fn from(err: E) -> Self { ErrorCompose::Head(err) }
}

/// Composed optics of `K` and `L`; `K` is applied first.
#[derive(Copy, Clone, PartialEq)]
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

impl<K: Optics<T>, L: Optics<K::View>, T: ?Sized> Optics<T> for Compose<K, L> {
    type View = L::View;
}

impl<K: OpticsFallible, L: OpticsFallible> OpticsFallible for Compose<K, L> {
    type Success = SuccessCompose<K::Success, L::Success>;
    type Error = ErrorCompose<K::Error, K::Success, L::Error>;
    fn success_witness(&self) -> Self::Success {
        SuccessCompose(self.0.success_witness(), self.1.success_witness())
    }
}

impl<K: AffineFold<T>, L: AffineFold<K::ViewSized>, T> AffineFold<T> for Compose<K, L> {
    fn preview(&self, s: T) -> Result<L::ViewSized, Self::Error> {
        self.1.preview(self.0.preview(s)?)
            .map_err(|err| ErrorCompose::Tail(self.0.success_witness(), err))
    }
}

impl<'a, T: ?Sized, K, L> AffineFoldRef<'a, T> for Compose<K, L>
    where K: AffineFoldRef<'a, T>,
          L: AffineFoldRef<'a, K::ViewLifeBound> {
    fn preview_ref(&self, s: &'a T) -> Result<&'a L::ViewLifeBound, Self::Error> {
        self.1.preview_ref(self.0.preview_ref(s)?)
            .map_err(|err| ErrorCompose::Tail(self.0.success_witness(), err))
    }
}

impl<'a, T: ?Sized, K, L> AffineFoldMut<'a, T> for Compose<K, L>
    where K: AffineFoldMut<'a, T>,
          L: AffineFoldMut<'a, K::ViewLifeBound> {
    fn preview_mut(&self, s: &'a mut T) -> Result<&'a mut L::ViewLifeBound, Self::Error> {
        self.1.preview_mut(self.0.preview_mut(s)?)
            .map_err(|err| ErrorCompose::Tail(self.0.success_witness(), err))
    }
}

impl<K: Getter<T>, L: Getter<K::ViewSized>, T> Getter<T> for Compose<K, L> {
    fn view(&self, s: T) -> L::ViewSized {
        self.1.view(self.0.view(s))
    }
}

impl<'a, T: ?Sized, K, L> GetterRef<'a, T> for Compose<K, L>
    where K: GetterRef<'a, T>,
          L: GetterRef<'a, K::ViewLifeBound> {
    fn view_ref(&self, s: &'a T) -> &'a L::ViewLifeBound {
        self.1.view_ref(self.0.view_ref(s))
    }
}

impl<'a, T: ?Sized, K, L> GetterMut<'a, T> for Compose<K, L>
    where K: GetterMut<'a, T>,
          L: GetterMut<'a, K::ViewLifeBound> {
    fn view_mut(&self, s: &'a mut T) -> &'a mut L::ViewLifeBound {
        self.1.view_mut(self.0.view_mut(s))
    }
}

impl<K: Review<T>, L: Review<K::ViewSized>, T> Review<T> for Compose<K, L> {
    fn review(&self, a: L::ViewSized) -> T {
        self.0.review(self.1.review(a))
    }
}

impl<K: Iso<T>, L: Iso<K::ViewSized>, T> Iso<T> for Compose<K, L> {}

impl<K: Setter<T>, L: Setter<K::ViewSized>, T> Setter<T> for Compose<K, L> {
    fn over(&self, s: &mut T, f: &mut dyn FnMut(&mut L::ViewSized)) {
        self.0.over(s, &mut |c| self.1.over(c, f))
    }
}

impl<K: Traversal<T>, L: Traversal<K::ViewSized>, T> Traversal<T> for Compose<K, L> {
    fn traverse(&self, s: T, f: &mut dyn FnMut(Self::ViewSized)) {
        self.0.traverse(s, &mut |v| self.1.traverse(v, f))
    }
}

impl<K: AffineTraversal<T>, L: AffineTraversal<K::ViewSized>, T> AffineTraversal<T> for Compose<K, L> {
    fn map(&self, s: &mut T, f: impl FnOnce(&mut L::ViewSized)) {
        self.0.map(s, |v| self.1.map(v, f))
    }
}

impl<K: Lens<T>, L: Lens<K::ViewSized>, T> Lens<T> for Compose<K, L> {}

impl<K: Prism<T>, L: Prism<K::ViewSized>, T> Prism<T> for Compose<K, L> {}

/// Optics wrapper for mapping the [`Success`] and [`Error`] value.
///
/// [`Success`]: OpticsFallible::Success
/// [`Error`]: OpticsFallible::Error
pub struct MapFallible<L, F, G>(pub(crate) L, pub(crate) F, pub(crate) G);

/// Optics wrapper for mapping the [`Success`] value.
///
/// [`Success`]: OpticsFallible::Success
pub type MapSuccess<L, F> = MapFallible<L, F, fn(<L as OpticsFallible>::Error) -> <L as OpticsFallible>::Error>;

/// Optics wrapper for mapping the [`Error`] value.
///
/// [`Error`]: OpticsFallible::Error
pub type MapError<L, G> = MapFallible<L, fn(<L as OpticsFallible>::Success) -> <L as OpticsFallible>::Success, G>;

/// Optics wrapper for mapping the [`Success`] and [`Error`] value to some specific type.
///
/// [`Success`]: OpticsFallible::Success
/// [`Error`]: OpticsFallible::Error
pub type MapFallibleTo<L, S, E> = MapFallible<L,
    fn(<L as OpticsFallible>::Success) -> S,
    fn(<L as OpticsFallible>::Error) -> E,
>;

impl<T: ?Sized, L: Optics<T>, F, G> Optics<T> for MapFallible<L, F, G> {
    type View = L::View;
}

impl<L: OpticsFallible, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> OpticsFallible for MapFallible<L, F, G> {
    type Success = S;
    type Error = E;
    fn success_witness(&self) -> S {
        (self.1)(self.0.success_witness())
    }
}

impl<T, L: AffineFold<T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> AffineFold<T> for MapFallible<L, F, G> {
    fn preview(&self, s: T) -> Result<Self::View, E> {
        self.0.preview(s).map_err(&self.2)
    }
}

impl<'a, T: ?Sized, L: AffineFoldRef<'a, T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> AffineFoldRef<'a, T> for MapFallible<L, F, G> {
    fn preview_ref(&self, s: &'a T) -> Result<&'a Self::View, E> {
        self.0.preview_ref(s).map_err(&self.2)
    }
}

impl<'a, T: ?Sized, L: AffineFoldMut<'a, T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> AffineFoldMut<'a, T> for MapFallible<L, F, G> {
    fn preview_mut(&self, s: &'a mut T) -> Result<&'a mut Self::View, E> {
        self.0.preview_mut(s).map_err(&self.2)
    }
}

impl<T, L: Getter<T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> Getter<T> for MapFallible<L, F, G> {
    fn view(&self, s: T) -> Self::View { self.0.view(s) }
}

impl<'a, T: ?Sized, L: GetterRef<'a, T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> GetterRef<'a, T> for MapFallible<L, F, G> {
    fn view_ref(&self, s: &'a T) -> &'a Self::View { self.0.view_ref(s) }
}

impl<'a, T: ?Sized, L: GetterMut<'a, T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> GetterMut<'a, T> for MapFallible<L, F, G> {
    fn view_mut(&self, s: &'a mut T) -> &'a mut Self::View { self.0.view_mut(s) }
}

impl<T, L: Review<T>, F, G> Review<T> for MapFallible<L, F, G> {
    fn review(&self, a: Self::View) -> T { self.0.review(a) }
}

impl<T, L: Iso<T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> Iso<T> for MapFallible<L, F, G> {}

impl<T, L: Setter<T>, F, G> Setter<T> for MapFallible<L, F, G> {
    fn over(&self, s: &mut T, f: &mut dyn FnMut(&mut Self::View)) { self.0.over(s, f) }
    fn set_cloned(&self, a: &Self::View, s: &mut T) where Self::View: Clone { self.0.set_cloned(a, s) }
}

impl<T, L: Traversal<T>, F, G> Traversal<T> for MapFallible<L, F, G> {
    fn traverse(&self, s: T, f: &mut dyn FnMut(Self::View)) { self.0.traverse(s, f) }
    fn fold<C>(&self, s: T, init: C, f: impl FnMut(&mut C, Self::View)) -> C { self.0.fold(s, init, f) }
    fn flatten(&self, s: T) -> Vec<Self::View> { self.0.flatten(s) }
}

impl<T, L: AffineTraversal<T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> AffineTraversal<T> for MapFallible<L, F, G> {
    fn map(&self, s: &mut T, f: impl FnOnce(&mut Self::View)) { self.0.map(s, f) }
    fn set(&self, s: &mut T, a: Self::View) { self.0.set(s, a) }
}

impl<T, L: Lens<T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> Lens<T> for MapFallible<L, F, G> {}

impl<T, L: Prism<T>, S, F: Fn(L::Success) -> S, E, G: Fn(L::Error) -> E> Prism<T> for MapFallible<L, F, G> {}

/// Easy composition for optics. See also [`Compose`].
///
/// ```
/// # use optics::optics;
/// # use optics::prelude::*;
/// # use optics::traits::AffineFold;
/// let val: Result<(bool, Option<u32>), &str> = Ok((true, Some(42)));
/// assert_eq!(optics!(_Ok._1._Some).preview(val), Ok(42));
/// ```
///
/// [`Compose`] implement [`Debug`] and [`Display`] in human readable format:
/// ```
/// # use optics::optics;
/// # use optics::prelude::*;
/// assert_eq!(format!("{}", optics!(_Ok._1._Some)), "Ok.1.Some".to_string());
/// assert_eq!(
///     format!("{:?}", optics!(_Ok._1._Some)),
///     "Result::Ok\
///     .{(T0, T1),(T0, T1, T2),(T0, T1, T2, T3)}::1\
///     .Option::Some".to_string()
/// );
/// ```
///
/// [`Compose`] implements provides error types through the [`OpticsFallible`] interface. The error
/// type implements [`Debug`] and [`Display`], indicating the shortest prefix of this [`Compose`]d
/// optics responsible for this error.
/// ```
/// # use optics::optics;
/// # use optics::prelude::*;
/// # use optics::traits::AffineFold;
/// let val: Result<(bool, Option<u32>), &str> = Err("top-level mismatch");
/// assert_eq!(optics!(_Ok._1._Some).preview(val).unwrap_err().to_string(), "Ok");
/// let val: Result<(bool, Option<u32>), &str> = Ok((true, None));
/// assert_eq!(optics!(_Ok._1._Some).preview(val).unwrap_err().to_string(), "Ok.1.Some");
/// ```
#[macro_export]
macro_rules! optics {
    () => { $crate::concrete::Identity };
    ($single:tt) => { #[allow(unused_parens)]{ $single } };
    ($head:tt $(. $tail:tt)*) => {
        $crate::concrete::Compose(
            #[allow(unused_parens)]{ $head },
            $crate::optics!($($tail).*),
        )
    }
}

/// Declare a [`Lens`] from an accessor expression.
///
/// Since we have mutability built into the language, we can make use of it, and [`Getter`] plus
/// [`GetterRef`] gives us full capabilities of the [`Lens`] interface.
///
/// Here is an example usage. The `match` expression below is reused 3 times in [`Getter::view`],
/// [`GetterRef::view_ref`], and [`GetterMut::view_mut`]; this relies on the fact that (shared,
/// mutable) borrow of fields is implicit in `match` expressions.
///
/// ```
/// # use optics::declare_lens;
/// # use optics::traits::*;
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
/// // Getter(Ref,Mut)
/// assert_eq!(_Alpha.view(color), 1.0);
/// assert_eq!(_Alpha.view_ref(&color), &1.0);
/// assert_eq!(_Alpha.view_mut(&mut color), &mut 1.0);
/// // AffineFold(Ref,Mut)
/// assert_eq!(_Alpha.preview(color), Ok(1.0));
/// assert_eq!(_Alpha.preview_ref(&color), Ok(&1.0));
/// assert_eq!(_Alpha.preview_mut(&mut color), Ok(&mut 1.0));
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
/// # use optics::traits::*;
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
/// // Getter(Ref,Mut)
/// assert_eq!(FooC.view(foo), 'A');
/// assert_eq!(FooC.view_ref(&foo), &'A');
/// assert_eq!(FooC.view_mut(&mut foo), &mut 'A');
/// // AffineFold(Ref,Mut)
/// assert_eq!(FooC.preview(foo), Ok('A'));
/// assert_eq!(FooC.preview_ref(&foo), Ok(&'A'));
/// assert_eq!(FooC.preview_mut(&mut foo), Ok(&mut 'A'));
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
        #[derive(Copy, Clone, PartialEq)]
        $vis struct $name;

        $crate::mark_infallible!($name);
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
        #[derive(Copy, Clone, PartialEq)]
        $vis struct $name;

        $crate::mark_infallible!($name);
        $crate::impl_lens! {
            $name as $base => $target $(, for<$($p),+>)?,
            ($s) $(reused($wrap))? => $reused
        }
    };
}

/// Mark an optics as infallible by implementing [`OpticsFallible`].
#[macro_export]
macro_rules! mark_infallible {
    ($name:ident) => {
        impl $crate::traits::OpticsFallible for $name {
            type Success = $name;
            type Error = std::convert::Infallible;
            fn success_witness(&self) -> $name { *self }
        }
    }
}

/// Mark an optics as fallible by implementing [`OpticsFallible`].
#[macro_export]
macro_rules! mark_fallible {
    ($name:ident) => {
        impl $crate::traits::OpticsFallible for $name {
            type Success = $name;
            type Error = $name;
            fn success_witness(&self) -> $name { *self }
        }
    }
}

/// Similar to [`declare_lens`], but does not define the lens type for you.
/// Normally [`mark_infallible`] should be used together with this macro.
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

        impl <'a $($(, $p)+)?> $crate::traits::GetterRef<'a, $base> for $name where $target: 'a {
            fn view_ref(&self, $s: &'a $base) -> &'a $target { $by_ref }
        }

        impl <'a $($(, $p)+)?> $crate::traits::GetterMut<'a, $base> for $name where $target: 'a {
            fn view_mut(&self, $s: &'a mut $base) -> &'a mut $target { $by_mut }
        }

        $crate::impl_up_from!([Getter(Ref,Mut)] $name as $base => $target $(, for<$($p),+>)?);

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
/// # use optics::traits::*;
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
/// // Getter(Ref,Mut)
/// assert_eq!(FooC.view(foo), 'A');
/// assert_eq!(FooC.view_ref(&foo), &'A');
/// assert_eq!(FooC.view_mut(&mut foo), &mut 'A');
/// // AffineFold(Ref,Mut)
/// assert_eq!(FooC.preview(foo), Ok('A'));
/// assert_eq!(FooC.preview_ref(&foo), Ok(&'A'));
/// assert_eq!(FooC.preview_mut(&mut foo), Ok(&mut 'A'));
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
        #[derive(Copy, Clone, PartialEq)]
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

        $crate::mark_infallible!($name);

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
/// # use optics::traits::*;
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
/// // AffineFold(Ref,Mut)
/// assert_eq!(FooChar.preview(foo), Ok('A'));
/// assert_eq!(FooChar.preview_ref(&foo), Ok(&'A'));
/// assert_eq!(FooChar.preview_mut(&mut foo), Ok(&mut 'A'));
/// assert_eq!(FooChar.preview(bar), Err(FooChar));
/// assert_eq!(FooChar.preview_ref(&bar), Err(FooChar));
/// assert_eq!(FooChar.preview_mut(&mut bar), Err(FooChar));
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
        $crate::declare_affine_traversal! {
            $(#[$m])* $vis $name as $base $(<$($p1),+>)? => $target $(, for<$($p),+>)?,
            (s) => if let $base::$variant(x) = s { Ok(x) } else { Err($name) }
        }

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
/// Similar to the example for [`declare_lens`], the `match` expression below is reused 3 times in
/// [`AffineFold::preview`], [`AffineFoldRef::preview_ref`], and [`AffineFoldMut::preview_mut`];
/// this relies on the fact that (shared, mutable) borrow of fields is implicit in `match`
/// expressions. Again, the `reused(wrap)` syntax is available for complicated use cases.
///
/// ```
/// # use optics::declare_affine_traversal;
/// # use optics::traits::*;
/// #[derive(Debug, Copy, Clone, PartialEq)]
/// enum Color {
///     Rgba { red: f32, green: f32, blue: f32, alpha: f32 },
///     Hsla { hue: f32, saturation: f32, lightness: f32, alpha: f32 },
/// }
///
/// declare_affine_traversal! {
///     /// Alpha component for colors.
///     #[derive(Debug)]
///     _Green as Color => f32,
///     (c) => match c {
///         Color::Rgba { green, .. } => Ok(green),
///         _ => Err(_Green),
///     }
/// }
///
/// let mut rgba = Color::Rgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };
/// let mut hsla = Color::Hsla { hue: 1.0, saturation: 1.0, lightness: 1.0, alpha: 1.0 };
/// // AffineFold(Ref,Mut)
/// assert_eq!(_Green.preview(rgba), Ok(1.0));
/// assert_eq!(_Green.preview_ref(&rgba), Ok(&1.0));
/// assert_eq!(_Green.preview_mut(&mut rgba), Ok(&mut 1.0));
/// assert_eq!(_Green.preview(hsla), Err(_Green));
/// assert_eq!(_Green.preview_ref(&hsla), Err(_Green));
/// assert_eq!(_Green.preview_mut(&mut hsla), Err(_Green));
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
        #[derive(Copy, Clone, PartialEq)]
        $vis struct $name;

        $crate::mark_fallible!($name);
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
        #[derive(Copy, Clone, PartialEq)]
        $vis struct $name;

        $crate::mark_fallible!($name);
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
            fn preview(&self, $s: $base) -> Result<$target, Self::Error> { $by_val }
        }

        impl <'a $($(, $p)+)?> $crate::traits::AffineFoldRef<'a, $base> for $name where $target: 'a {
            fn preview_ref(&self, $s: &'a $base) -> Result<&'a $target, Self::Error> { $by_ref }
        }

        impl <'a $($(, $p)+)?> $crate::traits::AffineFoldMut<'a, $base> for $name where $target: 'a {
            fn preview_mut(&self, $s: &'a mut $base) -> Result<&'a mut $target, Self::Error> { $by_mut }
        }

        $crate::impl_up_from!([AffineFold(Mut)] $name as $base => $target $(, for<$($p),+>)?);
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
/// | Base Trait                               | Free Implementation                                  |
/// |------------------------------------------|------------------------------------------------------|
/// | [`AffineTraversal`]                      | [`Setter`]                                           |
/// | [`AffineFold`], [`AffineFoldMut`]        | [`Traversal`], [`AffineTraversal`]                   |
/// | [`Getter`], [`GetterRef`], [`GetterMut`] | [`AffineFold`], [`AffineFoldRef`], [`AffineFoldMut`] |
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
    // AffineFold(Mut) => Traversal, AffineTraversal
    (
        [AffineFold(Mut)]
        $(
            $name:ident as $base:ty => $target:ty
            $(, for <$($p:ident),+ $(,)?>)?
        );+ $(;)?
    ) => {$(
        impl $(<$($p),+>)? $crate::traits::Traversal<$base> for $name {
            fn traverse(&self, s: $base, f: &mut dyn FnMut($target)) {
                let _ = $crate::traits::AffineFold::preview(self, s).map(f);
            }
        }

        impl $(<$($p),+>)? $crate::traits::AffineTraversal<$base> for $name {
            fn map(&self, s: &mut $base, f: impl FnOnce(&mut $target)) {
                let _ = $crate::traits::AffineFoldMut::preview_mut(self, s).map(f);
            }
        }

        $crate::impl_up_from!([AffineTraversal] $name as $base => $target $(, for<$($p),+>)?);
    )+};
    // Getter(Ref,Mut) => AffineFold, AffineFoldRef, AffineFoldMut
    (
        [Getter(Ref,Mut)]
        $(
            $name:ident as $base:ty => $target:ty
            $(, for <$($p:ident),+ $(,)?>)?
        );+ $(;)?
    ) => {$(
        impl $(<$($p),+>)? $crate::traits::AffineFold<$base> for $name {
            fn preview(&self, s: $base) -> Result<$target, Self::Error> {
                Ok($crate::traits::Getter::view(self, s))
            }
        }

        impl <'a $($(, $p)+)?> $crate::traits::AffineFoldRef<'a, $base> for $name where $target: 'a {
            fn preview_ref(&self, s: &'a $base) -> Result<&'a $target, Self::Error> {
                Ok($crate::traits::GetterRef::view_ref(self, s))
            }
        }

        impl <'a $($(, $p)+)?> $crate::traits::AffineFoldMut<'a, $base> for $name where $target: 'a {
            fn preview_mut(&self, s: &'a mut $base) -> Result<&'a mut $target, Self::Error> {
                Ok($crate::traits::GetterMut::view_mut(self, s))
            }
        }

        $crate::impl_up_from!([AffineFold(Mut)] $name as $base => $target $(, for<$($p),+>)?);
    )+};
}
