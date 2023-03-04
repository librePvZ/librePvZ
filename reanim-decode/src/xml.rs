/*
 * reanim-decode: decoder for PvZ reanim files.
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

//! Original XML format for `.reanim` files.

use std::fmt::{Display, Formatter};
use crate::reanim::{Animation, Elements, Track, Transform};

/// Display in XML format.
pub trait DisplayXml {
    /// `Display`-like API.
    fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result;
    /// Convenience function: convert to XML string.
    fn to_xml_string(&self) -> String { Xml(self).to_string() }
}

impl<'a, T: DisplayXml + ?Sized> DisplayXml for &'a T {
    fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result { T::fmt_xml(self, f) }
}

/// Wrapper for formatting using [`DisplayXml`].
///
/// # Note
/// We deliberately do not derive [`Debug`] for `Xml`, to prevent accidental misuse.
/// Print by [`Display`], or use [`DisplayXml::to_xml_string`].
#[allow(missing_debug_implementations)]
pub struct Xml<T>(pub T);

impl<T: DisplayXml> Display for Xml<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { self.0.fmt_xml(f) }
}

trait Field {
    fn write_field(&self, f: &mut Formatter<'_>, label: &str) -> std::fmt::Result;
}

impl<T: DisplayXml + ?Sized> Field for T {
    fn write_field(&self, f: &mut Formatter<'_>, label: &str) -> std::fmt::Result {
        write!(f, "<{label}>{}</{label}>", Xml(self))
    }
}

impl<T: Field> Field for Option<T> {
    fn write_field(&self, f: &mut Formatter<'_>, label: &str) -> std::fmt::Result {
        if let Some(val) = self { val.write_field(f, label) } else { Ok(()) }
    }
}

macro_rules! impl_display_xml {
    ($($type_name:ty),+ $(,)?) => {
        $(
            impl DisplayXml for $type_name {
                fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self)
                }
            }
        )+
    }
}

impl_display_xml! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64, str, String,
}

impl DisplayXml for Animation {
    fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fps.write_field(f, "fps")?;
        writeln!(f)?;
        for track in self.tracks.iter() {
            track.fmt_xml(f)?;
        }
        Ok(())
    }
}

impl DisplayXml for Track {
    fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "<track>")?;
        self.name.write_field(f, "name")?;
        writeln!(f)?;
        for frame in self.frames.iter() {
            write!(f, "<t>")?;
            frame.transform.fmt_xml(f)?;
            frame.elements.fmt_xml(f)?;
            writeln!(f, "</t>")?;
        }
        writeln!(f, "</track>")
    }
}

impl DisplayXml for Transform {
    fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.x.write_field(f, "x")?;
        self.y.write_field(f, "y")?;
        self.kx.write_field(f, "kx")?;
        self.ky.write_field(f, "ky")?;
        self.sx.write_field(f, "sx")?;
        self.sy.write_field(f, "sy")?;
        self.f.write_field(f, "f")?;
        self.a.write_field(f, "a")?;
        Ok(())
    }
}

impl DisplayXml for Elements {
    fn fmt_xml(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.image.write_field(f, "i")?;
        self.font.write_field(f, "font")?;
        self.text.write_field(f, "text")?;
        Ok(())
    }
}
