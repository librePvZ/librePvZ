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

/// Easy composition for optics.
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

/// Declare a lens from field name for a `struct`.
///
/// ```
/// # use optics::declare_lens_from_field;
/// # use optics::traits::{Getter, GetterRef, Setter, AffineTraversal, AffineFold};
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
/// // AffineFold
/// assert_eq!(FooC.preview(foo), Some('A'));
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
            impl $(<$($p),+>)? $crate::traits::Optics<$base> for $name {
                type View = $target;
            }

            impl $(<$($p),+>)? $crate::traits::Getter<$base> for $name {
                fn view(&self, s: $base) -> $target { s.$field }
            }

            impl $(<$($p),+>)? $crate::traits::GetterRef<$base> for $name {
                fn view_ref<'a>(&self, s: &'a $base) -> &'a $target { &s.$field }
                fn view_mut<'a>(&self, s: &'a mut $base) -> &'a mut $target { &mut s.$field }
            }

            impl $(<$($p),+>)? $crate::traits::AffineFold<$base> for $name {
                fn preview(&self, s: $base) -> Option<$target> { Some(s.$field) }
            }

            impl $(<$($p),+>)? $crate::traits::Setter<$base> for $name {
                fn over(&self, s: &mut $base, f: &mut dyn FnMut(&mut $target)) {
                    f(&mut s.$field)
                }
            }

            impl $(<$($p),+>)? $crate::traits::Traversal<$base> for $name {
                fn traverse(&self, s: $base, f: &mut dyn FnMut($target)) { f(s.$field) }
            }

            impl $(<$($p),+>)? $crate::traits::AffineTraversal<$base> for $name {
                fn map(&self, s: &mut $base, f: impl FnOnce(&mut $target)) { f(&mut s.$field) }
            }

            impl $(<$($p),+>)? $crate::traits::Lens<$base> for $name {}
        )+
    )+}
}

/// Declare a prism from variant name for an `enum`.
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

        impl $(<$($p),+>)? $crate::traits::Optics<$base $(<$($p1),+>)?> for $name {
            type View = $target;
        }

        impl $(<$($p),+>)? $crate::traits::AffineFold<$base $(<$($p1),+>)?> for $name {
            fn preview(&self, s: $base $(<$($p1),+>)?) -> Option<$target> {
                if let $base::$variant(x) = s { Some(x) } else { None }
            }
        }

        impl $(<$($p),+>)? $crate::traits::AffineFoldRef<$base $(<$($p1),+>)?> for $name {
            fn preview_ref<'a>(&self, s: &'a $base $(<$($p1),+>)?) -> Option<&'a $target> {
                if let $base::$variant(x) = s { Some(x) } else { None }
            }
            fn preview_mut<'a>(&self, s: &'a mut $base $(<$($p1),+>)?) -> Option<&'a mut $target> {
                if let $base::$variant(x) = s { Some(x) } else { None }
            }
        }

        impl $(<$($p),+>)? $crate::traits::Review<$base $(<$($p1),+>)?> for $name {
            fn review(&self, a: $target) -> $base $(<$($p1),+>)? { $base::$variant(a) }
        }

        impl $(<$($p),+>)? $crate::traits::Setter<$base $(<$($p1),+>)?> for $name {
            fn over(&self, s: &mut $base $(<$($p1),+>)?, f: &mut dyn FnMut(&mut $target)) {
                if let $base::$variant(x) = s { f(x) }
            }
        }

        impl $(<$($p),+>)? $crate::traits::Traversal<$base $(<$($p1),+>)?> for $name {
            fn traverse(&self, s: $base $(<$($p1),+>)?, f: &mut dyn FnMut($target)) {
                if let $base::$variant(x) = s { f(x) }
            }
        }

        impl $(<$($p),+>)? $crate::traits::AffineTraversal<$base $(<$($p1),+>)?> for $name {
            fn map(&self, s: &mut $base $(<$($p1),+>)?, f: impl FnOnce(&mut $target)) {
                if let $base::$variant(x) = s { f(x) }
            }
        }

        impl $(<$($p),+>)? $crate::traits::Prism<$base $(<$($p1),+>)?> for $name {}
    )+}
}
