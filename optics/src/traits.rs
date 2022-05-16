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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Traits for polymorphic lens hierarchy.
//!
#![doc = include_str ! ("../optics.svg")]

use std::convert::Infallible;
use std::fmt::{Debug, Display};
use crate::concrete::{MapError, MapFallible, MapFallibleTo, MapSuccess};

/// Any optics: a view type associated.
pub trait Optics<T: ?Sized> {
    /// View type for this optics.
    type View: ?Sized;
}

/// Optics with a success type and an error type associated.
pub trait OpticsFallible {
    /// Success type for this optics.
    type Success;
    /// Error type for this optics.
    type Error;
    /// Get a lightweight witness for success.
    fn success_witness(&self) -> Self::Success;
    /// Map the `Success` and `Error` type for this fallible optics.
    fn map_fallible<S, F, E, G>(self, f: F, g: G) -> MapFallible<Self, F, G>
        where Self: Sized, F: Fn(Self::Success) -> S, G: Fn(Self::Error) -> E {
        MapFallible(self, f, g)
    }
    /// Map the `Success` type for this fallible optics.
    fn map_success<S, F>(self, f: F) -> MapSuccess<Self, F>
        where Self: Sized, F: Fn(Self::Success) -> S {
        MapFallible(self, f, std::convert::identity)
    }
    /// Map the `Error` type for this fallible optics.
    fn map_error<E, G>(self, g: G) -> MapError<Self, G>
        where Self: Sized, G: Fn(Self::Error) -> E {
        MapFallible(self, std::convert::identity, g)
    }
    /// Assert that this optics should never fail (in practice).
    /// The resulting optics panics on error.
    fn assert_infallible(self) -> MapFallibleTo<Self, Self::Success, Infallible>
        where Self: Sized, Self::Error: Debug {
        self.map_error(|err| panic!("unexpected failure: {err:?}"))
    }
    /// Map the `Error` type to `Box<str>`.
    fn to_str_err(self) -> MapFallibleTo<Self, Box<str>, Box<str>>
        where Self: Sized, Self::Error: Display, Self::Success: Display {
        self.map_fallible(|s| s.to_string().into_boxed_str(),
                          |err| err.to_string().into_boxed_str())
    }
}

/// Optics, with source and view types [`Sized`].
pub trait OpticsSized<T>: Optics<T, View=Self::ViewSized> {
    /// [`Optics::View`], but explicitly [`Sized`].
    type ViewSized;
}

impl<T, L: Optics<T>> OpticsSized<T> for L where L::View: Sized {
    type ViewSized = L::View;
}

/// Optics, with view types guaranteed to live long enough.
pub trait OpticsLifeBound<'a, T: ?Sized>: Optics<T, View=Self::ViewLifeBound> {
    /// [`Optics::View`], but explicitly bound by a lifetime.
    type ViewLifeBound: ?Sized + 'a;
}

impl<'a, T: ?Sized, L: Optics<T>> OpticsLifeBound<'a, T> for L where L::View: 'a {
    type ViewLifeBound = L::View;
}

/// AffineFold: getter, but may fail.
pub trait AffineFold<T>: OpticsSized<T> + OpticsFallible {
    /// Retrieve the value targeted by an AffineFold.
    fn preview(&self, s: T) -> Result<Self::View, Self::Error>;
}

/// AffineFold, with shared references.
pub trait AffineFoldRef<'a, T: ?Sized>: OpticsLifeBound<'a, T> + OpticsFallible {
    /// Retrieve a shared reference the value targeted by an AffineFold.
    fn preview_ref(&self, s: &'a T) -> Result<&'a Self::View, Self::Error>;
}

/// AffineFold, with mutable references.
pub trait AffineFoldMut<'a, T: ?Sized>: OpticsLifeBound<'a, T> + OpticsFallible {
    /// Retrieve a mutable reference the value targeted by an AffineFold.
    fn preview_mut(&self, s: &'a mut T) -> Result<&'a mut Self::View, Self::Error>;
}

/// Getter.
pub trait Getter<T>: AffineFold<T> {
    /// View the value pointed to by a getter.
    fn view(&self, s: T) -> Self::View;
}

/// Getter, with shared references.
pub trait GetterRef<'a, T: ?Sized>: AffineFoldRef<'a, T> {
    /// Get a shared reference to the value pointed to by a getter.
    fn view_ref(&self, s: &'a T) -> &'a Self::View;
}

/// Getter, with mutable references.
pub trait GetterMut<'a, T: ?Sized>: AffineFoldMut<'a, T> {
    /// Get a mutable reference to the value pointed to by a getter.
    fn view_mut(&self, s: &'a mut T) -> &'a mut Self::View;
}

/// Review: dual of getter.
pub trait Review<T>: OpticsSized<T> {
    /// Retrieve the value targeted by a review.
    fn review(&self, a: Self::View) -> T;
}

/// Isomorphisms: getter and review.
pub trait Iso<T>: Getter<T> + Review<T> + Lens<T> + Prism<T> {}

/// Setter.
pub trait Setter<T>: OpticsSized<T> {
    /// Apply a setter as a modifier.
    fn over(&self, s: &mut T, f: &mut dyn FnMut(&mut Self::View));
    /// Apply a setter.
    ///
    /// # Note
    /// The value to be set is cloned, because we don't know the exact number of holes to be filled
    /// in. If the optics has a stricter interface (i.e., it also implements [`AffineTraversal`]),
    /// use [`AffineTraversal::set`] instead.
    fn set_cloned(&self, a: &Self::View, s: &mut T) where Self::View: Clone {
        self.over(s, &mut |p| *p = a.clone())
    }
}

/// Traversal (and also Fold).
pub trait Traversal<T>: Setter<T> {
    /// Evaluate the action from left to right on each element targeted by a Traversal.
    fn traverse(&self, s: T, f: &mut dyn FnMut(Self::View));
    /// Fold every element targeted by this Traversal into a single result.
    fn fold<C>(&self, s: T, mut init: C, mut f: impl FnMut(&mut C, Self::View)) -> C {
        self.traverse(s, &mut |x| f(&mut init, x));
        init
    }
    /// Flatten the elements targeted by this Traversal into a [`Vec`].
    fn flatten(&self, s: T) -> Vec<Self::View> {
        self.fold(s, Vec::new(), |res, x| res.push(x))
    }
}

/// AffineTraversal: usually composition of [`Lens`]es and [`Prism`]s.
pub trait AffineTraversal<T>: Traversal<T> + AffineFold<T> {
    /// Restricted version for [`Setter::over`]. Custom implementation recommended.
    fn map(&self, s: &mut T, f: impl FnOnce(&mut Self::View)) {
        let mut f = Some(f);
        self.over(s, &mut move |p| std::mem::take(&mut f)
            .expect("this optics should be affine")(p))
    }
    /// Apply a setter. No [`Clone`] is needed, because this optics is _affine_.
    fn set(&self, s: &mut T, a: Self::View) {
        self.map(s, |p| *p = a)
    }
}

/// Lens: getter and setter.
pub trait Lens<T>: Getter<T> + AffineTraversal<T> {}

/// Prism: review and setter.
pub trait Prism<T>: Review<T> + AffineTraversal<T> {}
