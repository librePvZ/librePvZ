/*
 * librePvZ-animation: animation playing for librePvZ.
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

//! Helpers for [`bevy::reflect`], for use in keyframe animations.

use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use derivative::Derivative;
use bevy::reflect::{Reflect, ReflectMut, ReflectRef};
use optics::traits::{AffineFoldMut, AffineFoldRef, Optics, OpticsFallible};

/// Access into [`Reflect`] data types.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Access {
    /// Fields in structs, see [`Struct`](bevy::reflect::Struct).
    Field(&'static str),
    /// Fields in tuples or tuple structs, see [`Tuple`](bevy::reflect::Tuple) and
    /// [`TupleStruct`](bevy::reflect::TupleStruct).
    TupleIndex(usize),
    /// Elements in lists, see [`List`](bevy::reflect::List).
    ListIndex(usize),
}

/// Helper for creating [`Access`]es.
///
/// ```
/// # use libre_pvz_animation::access;
/// # use libre_pvz_animation::reflect::Access;
/// assert_eq!(access!(field_name), Access::Field("field_name"));
/// assert_eq!(access!(2),          Access::TupleIndex(2));
/// assert_eq!(access!([42]),       Access::ListIndex(42));
/// ```
///
/// Extra parentheses does not change the meaning (good for disambiguation):
/// ```
/// # use libre_pvz_animation::access;
/// # use libre_pvz_animation::reflect::Access;
/// assert_eq!(access!((2)),        Access::TupleIndex(2));
/// assert_eq!(access!((((2)))),    Access::TupleIndex(2));
/// ```
#[macro_export]
macro_rules! access {
    (($any:tt))    => { $crate::access!($any) };
    ($field:ident) => { $crate::reflect::Access::Field(stringify!($field)) };
    ($k:literal)   => { $crate::reflect::Access::TupleIndex($k) };
    ([$k:literal]) => { $crate::reflect::Access::ListIndex($k) };
}

impl Display for Access {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Access::Field(field) => write!(f, "{field}"),
            Access::TupleIndex(index) => write!(f, "{index}"),
            Access::ListIndex(index) => write!(f, "[{index}]"),
        }
    }
}

impl Optics<dyn Reflect> for Access {
    type View = dyn Reflect;
}

impl OpticsFallible for Access {
    type Success = ();
    type Error = ();
    fn success_witness(&self) {}
}

impl<'a> AffineFoldRef<'a, dyn Reflect> for Access {
    fn preview_ref(&self, data: &'a dyn Reflect) -> Result<&'a dyn Reflect, ()> {
        match (*self, data.reflect_ref()) {
            (Access::Field(f), ReflectRef::Struct(s)) => s.field(f),
            (Access::TupleIndex(k), ReflectRef::TupleStruct(t)) => t.field(k),
            (Access::TupleIndex(k), ReflectRef::Tuple(t)) => t.field(k),
            (Access::ListIndex(k), ReflectRef::List(lst)) => lst.get(k),
            _ => None,
        }.ok_or(())
    }
}

impl<'a> AffineFoldMut<'a, dyn Reflect> for Access {
    fn preview_mut(&self, data: &'a mut dyn Reflect) -> Result<&'a mut dyn Reflect, ()> {
        match (*self, data.reflect_mut()) {
            (Access::Field(f), ReflectMut::Struct(s)) => s.field_mut(f),
            (Access::TupleIndex(k), ReflectMut::TupleStruct(t)) => t.field_mut(k),
            (Access::TupleIndex(k), ReflectMut::Tuple(t)) => t.field_mut(k),
            (Access::ListIndex(k), ReflectMut::List(lst)) => lst.get_mut(k),
            _ => None,
        }.ok_or(())
    }
}

/// Whole path for access into [`Reflect`] data types.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct FieldPath(pub &'static [Access]);

/// Helper for creating [`FieldPath`]s.
/// - Empty field path refers to the root data.
/// - The `2` is parenthesized for disambiguation, because otherwise `2.` is parsed as a whole
///   floating-point literal.
///
/// ```
/// # use libre_pvz_animation::field_path;
/// # use libre_pvz_animation::reflect::{Access, FieldPath};
/// assert_eq!(field_path!(), FieldPath(&[]));
/// assert_eq!(field_path!(field_name.(2).[42]), FieldPath(&[
///     Access::Field("field_name"),
///     Access::TupleIndex(2),
///     Access::ListIndex(42),
/// ]));
/// ```
#[macro_export]
macro_rules! field_path {
    ($($head:tt $(. $tail:tt)*)?) => {
        $crate::reflect::FieldPath(&[
            $( $crate::access!($head) $(, $crate::access!($tail))* )?
        ])
    }
}

impl Display for FieldPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut parts = self.0.iter();
        if let Some(head) = parts.next() {
            write!(f, "{head}")?;
            for part in parts {
                write!(f, ".{part}")?;
            }
        }
        Ok(())
    }
}

impl FieldPath {
    /// Truncate this field path to only preserve the first `n` [`Access`] segments.
    pub fn truncate(self, n: usize) -> FieldPath { FieldPath(&self.0[..n]) }
}

impl<'a> Optics<dyn Reflect> for FieldPath {
    type View = dyn Reflect;
}

impl OpticsFallible for FieldPath {
    type Success = FieldPath;
    type Error = FieldPath;
    fn success_witness(&self) -> FieldPath { *self }
}

impl<'a> AffineFoldRef<'a, dyn Reflect> for FieldPath {
    fn preview_ref(&self, mut data: &'a dyn Reflect) -> Result<&'a dyn Reflect, FieldPath> {
        for (k, access) in std::iter::zip(1.., self.0) {
            data = access.preview_ref(data).map_err(|()| self.truncate(k + 1))?;
        }
        Ok(data)
    }
}

impl<'a> AffineFoldMut<'a, dyn Reflect> for FieldPath {
    fn preview_mut(&self, mut data: &'a mut dyn Reflect) -> Result<&'a mut dyn Reflect, FieldPath> {
        for (k, access) in std::iter::zip(1.., self.0) {
            data = access.preview_mut(data).map_err(|()| self.truncate(k + 1))?;
        }
        Ok(data)
    }
}

/// [`AffineFoldRef`] and [`AffineFoldMut`] from [`Reflect`] to a concrete type.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derivative(Copy(bound = ""), Clone(bound = ""))]
#[derivative(Eq(bound = ""), PartialEq(bound = ""))]
pub struct _Reflect<T: ?Sized>(PhantomData<fn() -> T>);

impl<T: ?Sized> Debug for _Reflect<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "_Reflect::<{}>", std::any::type_name::<T>())
    }
}

impl<T: ?Sized> Display for _Reflect<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "reflect<{}>", std::any::type_name::<T>())
    }
}

impl<T: ?Sized> Optics<dyn Reflect> for _Reflect<T> { type View = T; }

impl<T: ?Sized> OpticsFallible for _Reflect<T> {
    type Success = _Reflect<T>;
    type Error = _Reflect<T>;
    fn success_witness(&self) -> _Reflect<T> { *self }
}

impl<'a, T: Reflect + 'a> AffineFoldRef<'a, dyn Reflect> for _Reflect<T> {
    fn preview_ref(&self, s: &'a dyn Reflect) -> Result<&'a T, _Reflect<T>> {
        s.downcast_ref().ok_or(*self)
    }
}

impl<'a, T: Reflect + 'a> AffineFoldMut<'a, dyn Reflect> for _Reflect<T> {
    fn preview_mut(&self, s: &'a mut dyn Reflect) -> Result<&'a mut T, _Reflect<T>> {
        s.downcast_mut().ok_or(*self)
    }
}
