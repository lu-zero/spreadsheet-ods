//! Styles define a large number of attributes. These are grouped together
//! as table, row, column, cell, paragraph and text attributes.
//!
//! ```
//! use spreadsheet_ods::{CellRef, WorkBook};
//! use spreadsheet_ods::style::{StyleOrigin, StyleUse, CellStyle};
//! use color::Rgb;
//! use icu_locid::locale;
//! use spreadsheet_ods::style::stylemap::StyleMap;
//! use spreadsheet_ods::condition::ValueCondition;
//!
//! let mut wb = WorkBook::new(locale!("en_US"));
//!
//! let mut st = CellStyle::new("ce12", &"num2".into());
//! st.set_color(Rgb::new(192, 128, 0));
//! st.set_font_bold();
//! wb.add_cellstyle(st);
//!
//! let mut st = CellStyle::new("ce11", &"num2".into());
//! st.set_color(Rgb::new(0, 192, 128));
//! st.set_font_bold();
//! wb.add_cellstyle(st);
//!
//! let mut st = CellStyle::new("ce13", &"num4".into());
//! st.push_stylemap(StyleMap::new(ValueCondition::content_eq("BB"), "ce12", CellRef::remote("sheet0", 4, 3)));
//! st.push_stylemap(StyleMap::new(ValueCondition::content_eq("CC"), "ce11", CellRef::remote("sheet0", 4, 3)));
//! wb.add_cellstyle(st);
//! ```
//! Styles can be defined in content.xml or as global styles in styles.xml. This
//! is reflected as the StyleOrigin. The StyleUse differentiates between automatic
//! and user visible, named styles. And third StyleFor defines for which part of
//! the document the style can be used.
//!
//! Cell styles usually reference a value format for text formatting purposes.
//!
//! Styles can also link to a parent style and to a pagelayout.
//!

use color::Rgb;

pub use cellstyle::*;
pub use colstyle::*;
pub use fontface::*;
pub use graphicstyle::*;
pub use masterpage::*;
pub use pagestyle::*;
pub use paragraphstyle::*;
pub use rowstyle::*;
pub use tablestyle::*;
pub use textstyle::*;

use crate::style::units::{Border, Length};

mod cellstyle;
mod colstyle;
mod fontface;
mod graphicstyle;
mod masterpage;
mod pagestyle;
mod paragraphstyle;
mod rowstyle;
pub mod stylemap;
mod tablestyle;
pub mod tabstop;
mod textstyle;
pub mod units;

/// Origin of a style. Content.xml or Styles.xml.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleOrigin {
    /// Style comes from Content.xml
    Content,
    /// Style comes from Styles.xml
    Styles,
}

impl Default for StyleOrigin {
    fn default() -> Self {
        StyleOrigin::Content
    }
}

/// Placement of a style. office:styles or office:automatic-styles
/// Defines the usage pattern for the style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleUse {
    /// The style:default-style element represents default styles. A default style specifies
    /// default formatting properties for a style family. These defaults are used if a formatting property is
    /// neither specified by an automatic nor a common style. Default styles exist for all style families that
    /// are represented by the style:style element specified by the style:family attribute
    /// 19.480.
    /// An OpenDocument document should contain the default styles of the style families for which are
    /// used in common or automatic styles in the document.
    Default,
    /// The office:styles element contains common styles used in a document. A common style
    /// is a style chosen by a user for a document or portion thereof.
    Named,
    /// The office:automatic-styles element contains automatic styles used in a document.
    /// An automatic style is a set of formatting properties treated as properties of the object to which the
    /// style is assigned.
    ///
    /// Note: Common and automatic styles behave differently in OpenDocument editing
    /// consumers. Common styles present to a user as a named set of formatting
    /// properties. The formatting properties of an automatic style present to a user as
    /// properties of the object to which the style is applied.
    Automatic,
}

impl Default for StyleUse {
    fn default() -> Self {
        StyleUse::Automatic
    }
}

pub(crate) fn color_string(color: Rgb<u8>) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

pub(crate) fn shadow_string(
    x_offset: Length,
    y_offset: Length,
    blur: Option<Length>,
    color: Rgb<u8>,
) -> String {
    if let Some(blur) = blur {
        format!("{} {} {} {}", color_string(color), x_offset, y_offset, blur)
    } else {
        format!("{} {} {}", color_string(color), x_offset, y_offset)
    }
}

pub(crate) fn rel_width_string(value: f64) -> String {
    format!("{}*", value)
}

pub(crate) fn border_string(width: Length, border: Border, color: Rgb<u8>) -> String {
    format!(
        "{} {} #{:02x}{:02x}{:02x}",
        width, border, color.r, color.g, color.b
    )
}

pub(crate) fn percent_string(value: f64) -> String {
    format!("{}%", value)
}

pub(crate) fn border_line_width_string(inner: Length, space: Length, outer: Length) -> String {
    format!("{} {} {}", inner, space, outer)
}
