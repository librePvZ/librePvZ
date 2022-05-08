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

//! Prelude, aimed for blanket import.
//!
//! Lenses for tuples are only defined for each tuple with arity `n <= 4`. Normally, tuples with
//! more fields are discouraged for practical use. That said, one can always declare his own ones
//! with the macro [`declare_lens_from_field`].

#![allow(clippy::manual_map)]

use crate::{declare_lens_from_field, declare_prism_from_variant};

declare_prism_from_variant! {
    /// Prism for [`Option::Some`].
    pub _Some for Some as Option<T> => T, for<T>;
}

declare_prism_from_variant! {
    /// Prism for [`Result::Ok`].
    pub _Ok for Ok as Result<T, E> => T, for<T, E>;
    /// Prism for [`Result::Err`].
    pub _Err for Err as Result<T, E> => E, for<T, E>;
}

declare_lens_from_field! {
    /// Lens for field `0` in tuples.
    pub _0 for 0
        as (T0, ) => T0, for<T0>
        as (T0, T1) => T0, for<T0, T1>
        as (T0, T1, T2) => T0, for<T0, T1, T2>
        as (T0, T1, T2, T3) => T0, for<T0, T1, T2, T3>;
    /// Lens for field `1` in tuples.
    pub _1 for 1
        as (T0, T1) => T1, for<T0, T1>
        as (T0, T1, T2) => T1, for<T0, T1, T2>
        as (T0, T1, T2, T3) => T1, for<T0, T1, T2, T3>;
    /// Lens for field `2` in tuples.
    pub _2 for 2
        as (T0, T1, T2) => T2, for<T0, T1, T2>
        as (T0, T1, T2, T3) => T2, for<T0, T1, T2, T3>;
    /// Lens for field `3` in tuples.
    pub _3 for 3
        as (T0, T1, T2, T3) => T3, for<T0, T1, T2, T3>;
}
