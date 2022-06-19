//!
//! Defines ValueFormat for formatting related issues
//!
//! ```
//! use spreadsheet_ods::{ValueFormat, ValueType};
//! use spreadsheet_ods::format::{FormatCalendar, FormatMonth, FormatNumberStyle, FormatTextual};
//!
//! let mut v = ValueFormat::new_named("dt0", ValueType::DateTime);
//! v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
//! v.push_text(".");
//! v.push_month(FormatNumberStyle::Long, FormatTextual::Numeric, FormatMonth::Nominativ, FormatCalendar::Default);
//! v.push_text(".");
//! v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
//! v.push_text(" ");
//! v.push_hours(FormatNumberStyle::Long);
//! v.push_text(":");
//! v.push_minutes(FormatNumberStyle::Long);
//! v.push_text(":");
//! v.push_seconds(FormatNumberStyle::Long, 0);
//!
//! let mut v = ValueFormat::new_named("n3", ValueType::Number);
//! v.push_number(3, false);
//! ```
//! The output formatting is a rough approximation with the possibilities
//! offered by format! and chrono::format. Especially there is no trace of
//! i18n. But on the other hand the formatting rules are applied by LibreOffice
//! when opening the spreadsheet so typically nobody notices this.
//!

use crate::attrmap2::AttrMap2;
use crate::style::stylemap::StyleMap;
use crate::style::units::{
    FontStyle, FontWeight, Length, LineMode, LineStyle, LineType, LineWidth, TextPosition,
    TextRelief, TextTransform, TransliterationStyle,
};
use crate::style::{
    color_string, percent_string, shadow_string, StyleOrigin, StyleUse, TextStyleRef,
};
use crate::ValueType;
use chrono::Duration;
use chrono::NaiveDateTime;
use color::Rgb;
use icu_locid::subtags::{Language, Region, Script};
use icu_locid::Locale;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Error type for any formatting errors.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum ValueFormatError {
    Format(String),
    NaN,
}

impl Display for ValueFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ValueFormatError::Format(s) => write!(f, "{}", s)?,
            ValueFormatError::NaN => write!(f, "Digit expected")?,
        }
        Ok(())
    }
}

impl std::error::Error for ValueFormatError {}

style_ref!(ValueFormatRef);

// Styles and attributes
//
// Attributes for all styles:
// ok number:country 19.342
// ok number:language 19.351
// ignore number:rfc-language-tag 19.360
// ok number:script 19.361
// ok number:title 19.364
// ok number:transliteration-country 19.365
// ok number:transliteration-format 19.366
// ok number:transliteration-language 19.367
// ok number:transliteration-style 19.368
// ok style:display-name 19.476
// ok style:name 19.502
// ok style:volatile 19.521
//
// ValueType:Number -> number:number-style
//      no extras
//
// ValueType:Currency -> number:currency-style
// number:automatic-order 19.340
//
// ValueType:Percentage -> number:percentage-style
//      no extras
//
// ValueType:DateTime -> number:date-style
// number:automaticorder 19.340
// number:format-source 19.347,
//
// ValueType:TimeDuration -> number:time-style
// number:format-source 19.347
// number:truncate-on-overflow 19.365
//
// ValueType:Boolean -> number:boolean-style
//      no extras
//
// ValueType:Text -> number:text-style
//      no extras

/// Actual textual formatting of values.
#[derive(Debug, Clone)]
pub struct ValueFormat {
    /// Name
    name: String,
    /// Value type
    v_type: ValueType,
    /// Origin information.
    origin: StyleOrigin,
    /// Usage of this style.
    styleuse: StyleUse,
    /// Properties of the format.
    attr: AttrMap2,
    /// Cell text styles
    textstyle: AttrMap2,
    /// Parts of the format.
    parts: Vec<FormatPart>,
    /// Style map data.
    stylemaps: Option<Vec<StyleMap>>,
}

impl Default for ValueFormat {
    fn default() -> Self {
        ValueFormat::new()
    }
}

impl ValueFormat {
    /// New, empty.
    pub fn new() -> Self {
        ValueFormat {
            name: String::from(""),
            v_type: ValueType::Text,
            origin: StyleOrigin::Styles,
            styleuse: StyleUse::Default,
            attr: Default::default(),
            textstyle: Default::default(),
            parts: Default::default(),
            stylemaps: None,
        }
    }

    /// New, with name.
    pub fn new_named<S: Into<String>>(name: S, value_type: ValueType) -> Self {
        assert_ne!(value_type, ValueType::Empty);
        ValueFormat {
            name: name.into(),
            v_type: value_type,
            origin: StyleOrigin::Styles,
            styleuse: StyleUse::Default,
            attr: Default::default(),
            textstyle: Default::default(),
            parts: Default::default(),
            stylemaps: None,
        }
    }

    /// New, with name.
    pub fn new_localized<S: Into<String>>(name: S, locale: Locale, value_type: ValueType) -> Self {
        assert_ne!(value_type, ValueType::Empty);
        let mut v = ValueFormat {
            name: name.into(),
            v_type: value_type,
            origin: StyleOrigin::Styles,
            styleuse: StyleUse::Default,
            attr: Default::default(),
            textstyle: Default::default(),
            parts: Default::default(),
            stylemaps: None,
        };
        v.set_language(locale.id.language);
        if let Some(region) = locale.id.region {
            v.set_country(region);
        }
        if let Some(script) = locale.id.script {
            v.set_script(script);
        }
        v
    }

    /// Returns a reference name for this value format.
    pub fn format_ref(&self) -> ValueFormatRef {
        ValueFormatRef::from(self.name().as_str())
    }

    /// The style:name attribute specifies names that reference style mechanisms.
    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        self.name = name.into();
    }

    /// Style name.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// The number:title attribute specifies the title of a data style.
    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.attr.set_attr("number:title", title.into());
    }

    /// Title
    pub fn title(&self) -> Option<&String> {
        self.attr.attr("number:title")
    }

    /// The style:display-name attribute specifies the name of a style as it should appear in the user
    /// interface. If this attribute is not present, the display name should be the same as the style name.
    pub fn set_display_name<S: Into<String>>(&mut self, name: S) {
        self.attr.set_attr("number:country", name.into());
    }

    /// Display name.
    pub fn display_name(&self) -> Option<&String> {
        self.attr.attr("number:country")
    }

    /// The number:country attribute specifies a country code for a data style. The country code is
    /// used for formatting properties whose evaluation is locale-dependent.
    /// If a country is not specified, the system settings are used
    pub fn set_country(&mut self, country: Region) {
        self.attr.set_attr("number:country", country.to_string());
    }

    /// Country
    pub fn country(&self) -> Option<Region> {
        match self.attr.attr("number:country") {
            None => None,
            Some(v) => v.parse().ok(),
        }
    }

    /// The number:language attribute specifies a language code. The country code is used for
    /// formatting properties whose evaluation is locale-dependent.
    /// If a language code is not specified, either the system settings or the setting for the system's
    /// language are used, depending on the property whose value should be evaluated.
    pub fn set_language(&mut self, language: Language) {
        self.attr.set_attr("number:language", language.to_string());
    }

    /// Language
    pub fn language(&self) -> Option<Language> {
        match self.attr.attr("number:language") {
            None => None,
            Some(v) => v.parse().ok(),
        }
    }

    /// The number:script attribute specifies a script code. The script code is used for formatting
    /// properties whose evaluation is locale-dependent. The attribute should be used only if necessary
    /// according to the rules of §2.2.3 of [RFC5646], or its successors.
    pub fn set_script(&mut self, script: Script) {
        self.attr.set_attr("number:script", script.to_string());
    }

    /// Script
    pub fn script(&self) -> Option<Script> {
        match self.attr.attr("number:script") {
            None => None,
            Some(v) => v.parse().ok(),
        }
    }

    /// The number:transliteration-country attribute specifies a country code in conformance
    /// with [RFC5646].
    /// If no language/country (locale) combination is specified, the locale of the data style is used.
    pub fn set_transliteration_country(&mut self, country: Region) {
        self.attr
            .set_attr("number:transliteration-country", country.to_string());
    }

    /// Transliteration country.
    pub fn transliteration_country(&self) -> Option<Region> {
        match self.attr.attr("number:transliteration-country") {
            None => None,
            Some(v) => v.parse().ok(),
        }
    }

    /// The number:transliteration-language attribute specifies a language code in
    /// conformance with [RFC5646].
    /// If no language/country (locale) combination is specified, the locale of the data style is used
    pub fn set_transliteration_language(&mut self, language: Language) {
        self.attr
            .set_attr("number:transliteration-language", language.to_string());
    }

    /// Transliteration language.
    pub fn transliteration_language(&self) -> Option<Language> {
        match self.attr.attr("number:transliteration-language") {
            None => None,
            Some(v) => v.parse().ok(),
        }
    }

    /// The number:transliteration-format attribute specifies which number characters to use.
    /// The value of the number:transliteration-format attribute shall be a decimal "DIGIT ONE"
    /// character with numeric value 1 as listed in the Unicode Character Database file UnicodeData.txt
    /// with value 'Nd' (Numeric decimal digit) in the General_Category/Numeric_Type property field 6
    /// and value '1' in the Numeric_Value fields 7 and 8, respectively as listed in
    /// DerivedNumericValues.txt
    /// If no format is specified the default ASCII representation of Latin-Indic digits is used, other
    /// transliteration attributes present in that case are ignored.
    /// The default value for this attribute is 1
    pub fn set_transliteration_format(&mut self, format: char) {
        self.attr
            .set_attr("number:transliteration-format", format.into());
    }

    /// Transliteration format.
    pub fn transliteration_format(&self) -> Option<char> {
        match self.attr.attr("number:transliteration-format") {
            None => None,
            Some(v) => v.chars().next(),
        }
    }

    /// The number:transliteration-style attribute specifies the transliteration format of a
    /// number system.
    /// The semantics of the values of the number:transliteration-style attribute are locale- and
    /// implementation-dependent.
    /// The default value for this attribute is short.
    pub fn set_transliteration_style(&mut self, style: TransliterationStyle) {
        self.attr
            .set_attr("number:transliteration-style", style.to_string());
    }

    /// Transliteration style.
    pub fn transliteration_style(&self) -> Option<TransliterationStyle> {
        match self.attr.attr("number:transliteration-style") {
            None => None,
            Some(s) => FromStr::from_str(s.as_str()).ok(),
        }
    }

    /// The style:volatile attribute specifies whether unused style in a document are retained or
    /// discarded by consumers.
    /// The defined values for the style:volatile attribute are:
    ///   false: consumers should discard the unused styles.
    ///   true: consumers should keep unused styles.
    pub fn set_volatile(&mut self, volatile: bool) {
        self.attr.set_attr("style:volatile", volatile.to_string());
    }

    /// Transliteration style.
    pub fn volatile(&self) -> Option<bool> {
        match self.attr.attr("style:volatile") {
            None => None,
            Some(s) => FromStr::from_str(s.as_str()).ok(),
        }
    }

    /// The number:automatic-order attribute specifies whether data is ordered to match the default
    /// order for the language and country of a data style.
    /// The defined values for the number:automatic-order attribute are:
    /// - false: data is not ordered to match the default order for the language and country of a data
    /// style.
    /// - true: data is ordered to match the default order for the language and country of a data style.
    /// The default value for this attribute is false.
    ///
    /// This attribute is valid for date and currency formats.
    pub fn set_automatic_order(&mut self, volatile: bool) {
        self.attr
            .set_attr("number:automatic-order", volatile.to_string());
    }

    /// Automatic order.
    pub fn automatic_order(&self) -> Option<bool> {
        match self.attr.attr("number:automatic-order") {
            None => None,
            Some(s) => FromStr::from_str(s.as_str()).ok(),
        }
    }

    /// Sets the value type.
    pub fn set_value_type(&mut self, value_type: ValueType) {
        assert_ne!(value_type, ValueType::Empty);
        self.v_type = value_type;
    }

    /// Returns the value type.
    pub fn value_type(&self) -> ValueType {
        self.v_type
    }

    /// Sets the origin.
    pub fn set_origin(&mut self, origin: StyleOrigin) {
        self.origin = origin;
    }

    /// Returns the origin.
    pub fn origin(&self) -> StyleOrigin {
        self.origin
    }

    /// Style usage.
    pub fn set_styleuse(&mut self, styleuse: StyleUse) {
        self.styleuse = styleuse;
    }

    /// Returns the usage.
    pub fn styleuse(&self) -> StyleUse {
        self.styleuse
    }

    /// All direct attributes of the number:xxx-style tag.
    pub(crate) fn attrmap(&self) -> &AttrMap2 {
        &self.attr
    }

    /// All direct attributes of the number:xxx-style tag.
    pub(crate) fn attrmap_mut(&mut self) -> &mut AttrMap2 {
        &mut self.attr
    }

    /// Text style attributes.
    pub fn textstyle(&self) -> &AttrMap2 {
        &self.textstyle
    }

    /// Text style attributes.
    pub fn textstyle_mut(&mut self) -> &mut AttrMap2 {
        &mut self.textstyle
    }

    text!(textstyle_mut);

    /// The <number:number> element specifies the display formatting properties for a decimal
    /// number.
    ///
    /// Can be used with ValueTypes:
    /// * Currency
    /// * Number
    /// * Percentage
    pub fn push_number_full(
        &mut self,
        decimal_places: u8,
        grouping: bool,
        min_decimal_places: u8,
        mininteger_digits: u8,
        display_factor: Option<f64>,
        decimal_replacement: Option<char>,
    ) {
        // TODO: embedded-text
        self.push_part(FormatPart::new_number(
            decimal_places,
            grouping,
            min_decimal_places,
            mininteger_digits,
            display_factor,
            decimal_replacement,
        ));
    }

    /// The <number:number> element specifies the display formatting properties for a decimal
    /// number.
    pub fn push_number(&mut self, decimal_places: u8, grouping: bool) {
        self.push_part(FormatPart::new_number(
            decimal_places,
            grouping,
            0,
            1,
            None,
            None,
        ));
    }

    /// The <number:number> element specifies the display formatting properties for a decimal
    /// number.
    pub fn push_number_fix(&mut self, decimal_places: u8, grouping: bool) {
        self.push_part(FormatPart::new_number(
            decimal_places,
            grouping,
            decimal_places,
            1,
            None,
            None,
        ));
    }

    /// The <number:fill-character> element specifies a Unicode character that is displayed
    /// repeatedly at the position where the element occurs. The character specified is repeated as many
    /// times as possible, but the total resulting string shall not exceed the given cell content area.
    ///
    /// Can be used with ValueTypes:
    /// * Currency
    /// * DateTime
    /// * Number
    /// * Percentage
    /// * Text
    /// * TimeDuration
    pub fn push_fill_character(&mut self, fill_character: char) {
        self.push_part(FormatPart::new_fill_character(fill_character));
    }

    /// The <number:scientific-number> element specifies the display formatting properties for a
    /// number style that should be displayed in scientific format.
    ///
    /// Can be used with ValueTypes:
    /// * Number
    pub fn push_scientific_number(
        &mut self,
        decimal_places: u8,
        grouping: bool,
        min_exponentdigits: Option<u8>,
        min_integer_digits: Option<u8>,
    ) {
        self.push_part(FormatPart::new_scientific_number(
            decimal_places,
            grouping,
            min_exponentdigits,
            min_integer_digits,
        ));
    }

    /// The <number:fraction> element specifies the display formatting properties for a number style
    /// that should be displayed as a fraction.
    ///
    /// Can be used with ValueTypes:
    /// * Number
    pub fn push_fraction(
        &mut self,
        denominatorvalue: u32,
        min_denominator_digits: u8,
        min_integer_digits: u8,
        min_numerator_digits: u8,
        grouping: bool,
        max_denominator_value: Option<u8>,
    ) {
        self.push_part(FormatPart::new_fraction(
            denominatorvalue,
            min_denominator_digits,
            min_integer_digits,
            min_numerator_digits,
            grouping,
            max_denominator_value,
        ));
    }

    /// The <number:currency-symbol> element specifies whether a currency symbol is displayed in
    /// a currency style.
    ///
    /// Can be used with ValueTypes:
    /// * Currency
    pub fn push_currency_symbol<S>(&mut self, locale: Locale, symbol: S)
    where
        S: Into<String>,
    {
        self.push_part(FormatPart::new_currency_symbol(locale, symbol));
    }

    /// The <number:day> element specifies a day of a month in a date.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_day(&mut self, style: FormatNumberStyle, calendar: FormatCalendar) {
        self.push_part(FormatPart::new_day(style, calendar));
    }

    /// The <number:month> element specifies a month in a date.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_month(
        &mut self,
        style: FormatNumberStyle,
        textual: FormatTextual,
        possessive_form: FormatMonth,
        calendar: FormatCalendar,
    ) {
        self.push_part(FormatPart::new_month(
            style,
            textual,
            possessive_form,
            calendar,
        ));
    }

    /// The <number:year> element specifies a year in a date
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_year(&mut self, style: FormatNumberStyle, calendar: FormatCalendar) {
        self.push_part(FormatPart::new_year(style, calendar));
    }

    /// The <number:era> element specifies an era in which a year is counted
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_era(&mut self, style: FormatNumberStyle, calendar: FormatCalendar) {
        self.push_part(FormatPart::new_era(style, calendar));
    }

    /// The <number:day-of-week> element specifies a day of a week in a date
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_day_of_week(&mut self, style: FormatNumberStyle, calendar: FormatCalendar) {
        self.push_part(FormatPart::new_day_of_week(style, calendar));
    }

    /// The <number:week-of-year> element specifies a week of a year in a date.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_week_of_year(&mut self, calendar: FormatCalendar) {
        self.push_part(FormatPart::new_week_of_year(calendar));
    }

    /// The <number:quarter> element specifies a quarter of the year in a date
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    pub fn push_quarter(&mut self, style: FormatNumberStyle, calendar: FormatCalendar) {
        self.push_part(FormatPart::new_quarter(style, calendar));
    }

    /// The <number:hours> element specifies whether hours are displayed as part of a date or time.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    /// * TimeDuration
    pub fn push_hours(&mut self, style: FormatNumberStyle) {
        self.push_part(FormatPart::new_hours(style));
    }

    /// The <number:minutes> element specifies whether minutes are displayed as part of a date or
    /// time.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    /// * TimeDuration
    pub fn push_minutes(&mut self, style: FormatNumberStyle) {
        self.push_part(FormatPart::new_minutes(style));
    }

    /// The <number:seconds> element specifies whether seconds are displayed as part of a date or
    /// time.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    /// * TimeDuration
    pub fn push_seconds(&mut self, style: FormatNumberStyle, decimal_places: u8) {
        self.push_part(FormatPart::new_seconds(style, decimal_places));
    }

    /// The <number:am-pm> element specifies whether AM/PM is included as part of a date or time.
    /// If a <number:am-pm> element is contained in a date or time style, hours are displayed using
    /// values from 1 to 12 only.
    ///
    /// Can be used with ValueTypes:
    /// * DateTime
    /// * TimeDuration
    pub fn push_am_pm(&mut self) {
        self.push_part(FormatPart::new_am_pm());
    }

    /// The <number:boolean> element marks the position of the Boolean value of a Boolean style.
    ///
    /// Can be used with ValueTypes:
    /// * Boolean
    pub fn push_boolean(&mut self) {
        self.push_part(FormatPart::new_boolean());
    }

    /// The <number:text> element contains any fixed text for a data style.
    ///
    /// Can be used with ValueTypes:
    /// * Boolean
    /// * Currency
    /// * DateTime
    /// * Number
    /// * Percentage
    /// * Text
    /// * TimeDuration
    pub fn push_text<S: Into<String>>(&mut self, text: S) {
        self.push_part(FormatPart::new_text(text));
    }

    /// The <number:text-content> element marks the position of variable text content of a text
    /// style.
    ///
    /// Can be used with ValueTypes:
    /// * Text
    pub fn push_text_content(&mut self) {
        self.push_part(FormatPart::new_text_content());
    }

    // /// The <number:///-text> element specifies text that is displayed at one specific position
    // /// within a number.
    // pub fn push_embedded_text<S: Into<String>>(&mut self, position: u8, text: S) {
    //     self.push_part(FormatPart::new_embedded_text(position, text));
    // }

    /// Adds a format part.
    pub fn push_part(&mut self, part: FormatPart) {
        self.parts.push(part);
    }

    /// Adds all format parts.
    pub fn push_parts(&mut self, partvec: &mut Vec<FormatPart>) {
        self.parts.append(partvec);
    }

    /// Returns the parts.
    pub fn parts(&self) -> &Vec<FormatPart> {
        &self.parts
    }

    /// Returns the mutable parts.
    pub fn parts_mut(&mut self) -> &mut Vec<FormatPart> {
        &mut self.parts
    }

    /// Adds a stylemap.
    pub fn push_stylemap(&mut self, stylemap: StyleMap) {
        self.stylemaps.get_or_insert_with(Vec::new).push(stylemap);
    }

    /// Returns the stylemaps
    pub fn stylemaps(&self) -> Option<&Vec<StyleMap>> {
        self.stylemaps.as_ref()
    }

    /// Returns the mutable stylemap.
    pub fn stylemaps_mut(&mut self) -> &mut Vec<StyleMap> {
        self.stylemaps.get_or_insert_with(Vec::new)
    }

    /// Tries to format.
    /// If there are no matching parts, does nothing.
    pub fn format_boolean(&self, b: bool) -> String {
        let mut buf = String::new();
        for p in &self.parts {
            p.format_boolean(&mut buf, b);
        }
        buf
    }

    /// Tries to format.
    /// If there are no matching parts, does nothing.
    pub fn format_float(&self, f: f64) -> String {
        let mut buf = String::new();
        for p in &self.parts {
            p.format_float(&mut buf, f);
        }
        buf
    }

    /// Tries to format.
    /// If there are no matching parts, does nothing.
    pub fn format_str<'a, S: Into<&'a str>>(&self, s: S) -> String {
        let mut buf = String::new();
        let s = s.into();
        for p in &self.parts {
            p.format_str(&mut buf, s);
        }
        buf
    }

    /// Tries to format.
    /// If there are no matching parts, does nothing.
    /// Should work reasonably. Don't ask me about other calenders.
    pub fn format_datetime(&self, d: &NaiveDateTime) -> String {
        let mut buf = String::new();

        let h12 = self
            .parts
            .iter()
            .any(|v| v.part_type == FormatPartType::AmPm);

        for p in &self.parts {
            p.format_datetime(&mut buf, d, h12);
        }
        buf
    }

    /// Tries to format. Should work reasonably.
    /// If there are no matching parts, does nothing.
    pub fn format_time_duration(&self, d: &Duration) -> String {
        let mut buf = String::new();
        for p in &self.parts {
            p.format_time_duration(&mut buf, d);
        }
        buf
    }
}

/// Identifies the structural parts of a value format.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum FormatPartType {
    Number,
    FillCharacter,
    ScientificNumber,
    Fraction,
    CurrencySymbol,
    Day,
    Month,
    Year,
    Era,
    DayOfWeek,
    WeekOfYear,
    Quarter,
    Hours,
    Minutes,
    Seconds,
    AmPm,
    Boolean,
    //EmbeddedText,
    Text,
    TextContent,
}

/// One structural part of a value format.
#[derive(Debug, Clone)]
pub struct FormatPart {
    /// What kind of format part is this?
    part_type: FormatPartType,
    /// Properties of this part.
    attr: AttrMap2,
    /// Some content.
    content: Option<String>,
}

/// Flag for several PartTypes.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum FormatNumberStyle {
    Short,
    Long,
}

impl Display for FormatNumberStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FormatNumberStyle::Short => write!(f, "short"),
            FormatNumberStyle::Long => write!(f, "long"),
        }
    }
}

/// Flag for several PartTypes.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum FormatTextual {
    Numeric,
    Textual,
}

impl Display for FormatTextual {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FormatTextual::Numeric => write!(f, "false"),
            FormatTextual::Textual => write!(f, "true"),
        }
    }
}

/// Flag for several PartTypes.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum FormatMonth {
    Nominativ,
    Possessiv,
}

impl Display for FormatMonth {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FormatMonth::Nominativ => write!(f, "false"),
            FormatMonth::Possessiv => write!(f, "true"),
        }
    }
}

/// Calendar types.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum FormatCalendar {
    Default,
    Gregorian,
    Gengou,
    Roc,
    Hanja,
    Hijri,
    Jewish,
    Buddhist,
}

impl Display for FormatCalendar {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FormatCalendar::Gregorian => write!(f, "gregorian"),
            FormatCalendar::Gengou => write!(f, "gengou"),
            FormatCalendar::Roc => write!(f, "ROC"),
            FormatCalendar::Hanja => write!(f, "hanja"),
            FormatCalendar::Hijri => write!(f, "hijri"),
            FormatCalendar::Jewish => write!(f, "jewish"),
            FormatCalendar::Buddhist => write!(f, "buddhist"),
            FormatCalendar::Default => Ok(()),
        }
    }
}

impl FormatPart {
    /// New, empty
    pub fn new(ftype: FormatPartType) -> Self {
        FormatPart {
            part_type: ftype,
            attr: Default::default(),
            content: None,
        }
    }

    /// New, with string content.
    pub fn new_with_content<S: Into<String>>(ftype: FormatPartType, content: S) -> Self {
        FormatPart {
            part_type: ftype,
            attr: Default::default(),
            content: Some(content.into()),
        }
    }

    /// The <number:number> element specifies the display formatting properties for a decimal
    /// number.
    /// The <number:number> element is usable within the following elements:
    /// * <number:currencystyle> 16.29.8,
    /// * <number:number-style> 16.29.2 and
    /// * <number:percentage-style> 16.29.10.
    ///
    /// The <number:number> element has the following attributes:
    /// * number:decimal-places 19.343,
    /// * number:decimal-replacement 19.344
    /// * number:display-factor 19.346
    /// * number:grouping 19.350
    /// * number:min-decimal-places 19.356 and
    /// * number:mininteger-digits 19.355.
    ///
    /// The <number:number> element has the following child element: <number:embedded-text>
    /// 16.29.4.
    ///
    pub fn new_number(
        decimal_places: u8,
        grouping: bool,
        min_decimal_places: u8,
        mininteger_digits: u8,
        display_factor: Option<f64>,
        decimal_replacement: Option<char>,
    ) -> Self {
        let mut p = FormatPart::new(FormatPartType::Number);
        p.set_attr("number:min-integer-digits", 1.to_string());
        p.set_attr("number:decimal-places", decimal_places.to_string());
        if let Some(decimal_replacement) = decimal_replacement {
            p.set_attr(
                "number:decimal-replacement",
                decimal_replacement.to_string(),
            );
        }
        if let Some(display_factor) = display_factor {
            p.set_attr("number:display-factor", display_factor.to_string());
        }
        p.set_attr("number:mininteger-digits", mininteger_digits.to_string());
        p.set_attr("number:min-decimal-places", min_decimal_places.to_string());
        if grouping {
            p.set_attr("number:grouping", String::from("true"));
        }

        // TODO: number:embedded-text
        p
    }

    /// The <number:fill-character> element specifies a Unicode character that is displayed
    /// repeatedly at the position where the element occurs. The character specified is repeated as many
    /// times as possible, but the total resulting string shall not exceed the given cell content area.
    ///
    /// Fill characters may not fill all the available space in a cell. The distribution of the
    /// remaining space is implementation-dependent.
    ///
    /// The <number:fill-character> element is usable within the following elements:
    /// * <number:currency-style> 16.29.8,
    /// * <number:date-style> 16.29.11,
    /// * <number:number-style> 16.29.2,
    /// * <number:percentage-style> 16.29.10,
    /// * <number:text-style> 16.29.26 and
    /// * <number:time-style> 16.29.19.
    ///
    /// The <number:fill-character> element has no attributes.
    /// The <number:fill-character> element has no child elements.
    /// The <number:fill-character> element has character data content.
    pub fn new_fill_character(fill_character: char) -> Self {
        let mut p = FormatPart::new(FormatPartType::FillCharacter);
        p.set_content(fill_character.to_string());
        p
    }

    /// The <number:fraction> element specifies the display formatting properties for a number style
    /// that should be displayed as a fraction.
    ///
    /// The <number:fraction> element is usable within the following element:
    /// * <number:numberstyle> 16.29.2.
    ///
    /// The <number:fraction> element has the following attributes:
    /// * number:denominatorvalue 19.345,
    /// * number:grouping 19.350,
    /// * number:max-denominator-value 19.352,
    /// * number:min-denominator-digits 19.353,
    /// * number:min-integer-digits 19.355 and
    /// * number:min-numerator-digits 19.357.
    ///
    /// The <number:fraction> element has no child elements.
    pub fn new_fraction(
        denominatorvalue: u32,
        min_denominator_digits: u8,
        min_integer_digits: u8,
        min_numerator_digits: u8,
        grouping: bool,
        max_denominator_value: Option<u8>,
    ) -> Self {
        let mut p = Self::new(FormatPartType::Fraction);
        p.set_attr("number:denominator-value", denominatorvalue.to_string());
        if let Some(max_denominator_value) = max_denominator_value {
            p.set_attr(
                "number:max-denominator-value",
                max_denominator_value.to_string(),
            );
        }
        p.set_attr(
            "number:min-denominator-digits",
            min_denominator_digits.to_string(),
        );
        p.set_attr("number:min-integer-digits", min_integer_digits.to_string());
        p.set_attr(
            "number:min-numerator-digits",
            min_numerator_digits.to_string(),
        );
        if grouping {
            p.set_attr("number:grouping", String::from("true"));
        }
        p
    }

    /// The <number:scientific-number> element specifies the display formatting properties for a
    /// number style that should be displayed in scientific format.
    ///
    /// The <number:scientific-number> element is usable within the following element:
    /// * <number:number-style> 16.27.2.
    ///
    /// The <number:scientific-number> element has the following attributes:
    /// * number:decimal-places 19.343.4,
    /// * number:grouping 19.348,
    /// * number:min-exponentdigits 19.351 and
    /// * number:min-integer-digits 19.352.
    ///
    /// The <number:scientific-number> element has no child elements.
    pub fn new_scientific_number(
        decimal_places: u8,
        grouping: bool,
        min_exponentdigits: Option<u8>,
        min_integer_digits: Option<u8>,
    ) -> Self {
        let mut p = Self::new(FormatPartType::ScientificNumber);
        p.set_attr("number:decimal-places", decimal_places.to_string());
        if grouping {
            p.set_attr("number:grouping", String::from("true"));
        }
        if let Some(min_exponentdigits) = min_exponentdigits {
            p.set_attr("number:min-exponentdigits", min_exponentdigits.to_string());
        }
        if let Some(min_integer_digits) = min_integer_digits {
            p.set_attr("number:min-integer-digits", min_integer_digits.to_string());
        }
        p
    }

    /// The <number:currency-symbol> element specifies whether a currency symbol is displayed in
    /// a currency style.
    /// The content of this element is the text that is displayed as the currency symbol.
    /// If the element is empty or contains white space characters only, the default currency
    /// symbol for the currency style or the language and country of the currency style is displayed.
    ///
    /// The <number:currency-symbol> element is usable within the following element:
    /// * <number:currency-style> 16.27.7.
    ///
    /// The <number:currency-symbol> element has the following attributes:
    /// * number:country 19.342,
    /// * number:language 19.349,
    /// * number:rfc-language-tag 19.356 and
    /// * number:script 19.357.
    ///
    /// The <number:currency-symbol> element has no child elements.
    /// The <number:currency-symbol> element has character data content.
    pub fn new_currency_symbol<S>(locale: Locale, symbol: S) -> Self
    where
        S: Into<String>,
    {
        let mut p = Self::new_with_content(FormatPartType::CurrencySymbol, symbol);
        p.set_attr("number:language", locale.id.language.to_string());
        if let Some(region) = locale.id.region {
            p.set_attr("number:country", region.to_string());
        }
        p
    }

    /// The <number:day> element specifies a day of a month in a date.
    ///
    /// The <number:day> element is usable within the following element:
    /// * <number:date-style> 16.27.10.
    ///
    /// The <number:day> element has the following attributes:
    /// * number:calendar 19.341 and
    /// * number:style 19.358.2.
    ///
    /// The <number:day> element has no child elements.
    pub fn new_day(style: FormatNumberStyle, calendar: FormatCalendar) -> Self {
        let mut p = Self::new(FormatPartType::Day);
        p.set_attr("number:style", style.to_string());
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:month> element specifies a month in a date.
    /// The <number:month> element is usable within the following element:
    /// * <number:date-style> 16.27.10.
    /// The <number:month> element has the following attributes:
    /// number:calendar 19.341,
    /// number:possessive-form 19.355,
    /// number:style 19.358.7 and
    /// number:textual 19.359.
    ///
    /// The <number:month> element has no child elements
    pub fn new_month(
        style: FormatNumberStyle,
        textual: FormatTextual,
        possessive_form: FormatMonth,
        calendar: FormatCalendar,
    ) -> Self {
        let mut p = Self::new(FormatPartType::Month);
        p.set_attr("number:style", style.to_string());
        p.set_attr("number:textual", textual.to_string());
        if possessive_form != FormatMonth::Possessiv {
            p.set_attr("number:possessive-form", true.to_string());
        }
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:year> element specifies a year in a date.
    /// The <number:year> element is usable within the following element:
    /// * <number:date-style> 16.27.10.
    ///
    /// The <number:year> element has the following attributes:
    /// * number:calendar 19.341 and
    /// * number:style 19.358.10.
    ///
    /// The <number:year> element has no child elements.
    pub fn new_year(style: FormatNumberStyle, calendar: FormatCalendar) -> Self {
        let mut p = Self::new(FormatPartType::Year);
        p.set_attr("number:style", style.to_string());
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:era> element specifies an era in which a year is counted.
    ///
    /// The <number:era> element is usable within the following element:
    /// * <number:date-style> 16.27.10.
    ///
    /// The <number:era> element has the following attributes:
    /// * number:calendar 19.341 and
    /// * number:style 19.358.4.
    ///
    /// The <number:era> element has no child elements
    pub fn new_era(number: FormatNumberStyle, calendar: FormatCalendar) -> Self {
        let mut p = Self::new(FormatPartType::Era);
        p.set_attr("number:style", number.to_string());
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:day-of-week> element specifies a day of a week in a date.
    ///
    /// The <number:day-of-week> element is usable within the following element:
    /// * <number:datestyle> 16.27.10.
    ///
    /// The <number:day-of-week> element has the following attributes:
    /// * number:calendar 19.341 and
    /// * number:style 19.358.3.
    ///
    /// The <number:day-of-week> element has no child elements.
    pub fn new_day_of_week(style: FormatNumberStyle, calendar: FormatCalendar) -> Self {
        let mut p = Self::new(FormatPartType::DayOfWeek);
        p.set_attr("number:style", style.to_string());
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:week-of-year> element specifies a week of a year in a date.
    ///
    /// The <number:week-of-year> element is usable within the following element:
    /// * <number:date-style> 16.27.10.
    ///
    /// The <number:week-of-year> element has the following attribute:
    /// * number:calendar 19.341.
    ///
    /// The <number:week-of-year> element has no child elements.
    pub fn new_week_of_year(calendar: FormatCalendar) -> Self {
        let mut p = Self::new(FormatPartType::WeekOfYear);
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:quarter> element specifies a quarter of the year in a date.
    ///
    /// The <number:quarter> element is usable within the following element:
    /// * <number:datestyle> 16.27.10.
    ///
    /// The <number:quarter> element has the following attributes:
    /// * number:calendar 19.341 and
    /// * number:style 19.358.8.
    ///
    /// The <number:quarter> element has no child elements
    pub fn new_quarter(style: FormatNumberStyle, calendar: FormatCalendar) -> Self {
        let mut p = Self::new(FormatPartType::Quarter);
        p.set_attr("number:style", style.to_string());
        if calendar != FormatCalendar::Default {
            p.set_attr("number:calendar", calendar.to_string());
        }
        p
    }

    /// The <number:hours> element specifies whether hours are displayed as part of a date or time.
    ///
    /// The <number:hours> element is usable within the following elements:
    /// * <number:datestyle> 16.27.10 and
    /// * <number:time-style> 16.27.18.
    ///
    /// The <number:hours> element has the following attribute:
    /// * number:style 19.358.5.
    ///
    /// The <number:hours> element has no child elements.
    pub fn new_hours(style: FormatNumberStyle) -> Self {
        let mut p = Self::new(FormatPartType::Hours);
        p.set_attr("number:style", style.to_string());
        p
    }

    /// The <number:minutes> element specifies whether minutes are displayed as part of a date or
    /// time.
    /// The <number:minutes> element is usable within the following elements:
    /// * <number:datestyle> 16.27.10 and
    /// * <number:time-style> 16.27.18.
    ///
    /// The <number:minutes> element has the following attribute:
    /// * number:style 19.358.6.
    ///
    /// The <number:minutes> element has no child elements.
    pub fn new_minutes(style: FormatNumberStyle) -> Self {
        let mut p = Self::new(FormatPartType::Minutes);
        p.set_attr("number:style", style.to_string());
        p
    }

    /// The <number:seconds> element specifies whether seconds are displayed as part of a date or
    /// time.
    ///
    /// The <number:seconds> element is usable within the following elements:
    /// * <number:datestyle> 16.27.10 and
    /// * <number:time-style> 16.27.18.
    ///
    /// The <number:seconds> element has the following attributes:
    /// * number:decimal-places 19.343.3 and
    /// * number:style 19.358.9.
    ///
    /// The <number:seconds> element has no child elements.
    pub fn new_seconds(style: FormatNumberStyle, decimal_places: u8) -> Self {
        let mut p = Self::new(FormatPartType::Seconds);
        p.set_attr("number:style", style.to_string());
        p.set_attr("number:decimal-places", decimal_places.to_string());
        p
    }

    /// The <number:am-pm> element specifies whether AM/PM is included as part of a date or time.
    /// If a <number:am-pm> element is contained in a date or time style, hours are displayed using
    /// values from 1 to 12 only.
    ///
    /// The <number:am-pm> element is usable within the following elements:
    /// * <number:datestyle> 16.27.10 and
    /// * <number:time-style> 16.27.18.
    ///
    /// The <number:am-pm> element has no attributes.
    /// The <number:am-pm> element has no child elements.
    pub fn new_am_pm() -> Self {
        Self::new(FormatPartType::AmPm)
    }

    /// The <number:boolean> element marks the position of the Boolean value of a Boolean style.
    ///
    /// The <number:boolean> element is usable within the following element:
    /// * <number:booleanstyle> 16.29.24.
    ///
    /// The <number:boolean> element has no attributes.
    /// The <number:boolean> element has no child elements.
    pub fn new_boolean() -> Self {
        FormatPart::new(FormatPartType::Boolean)
    }

    /// The <number:text> element contains any fixed text for a data style.
    ///
    /// The <number:text> element is usable within the following elements:
    /// * <number:booleanstyle> 16.27.23,
    /// * <number:currency-style> 16.27.7,
    /// * <number:date-style> 16.27.10,
    /// * <number:number-style> 16.27.2,
    /// * <number:percentage-style> 16.27.9,
    /// * <number:text-style> 16.27.25 and
    /// * <number:time-style> 16.27.18.
    ///
    /// The <number:text> element has no attributes.
    /// The <number:text> element has no child elements.
    /// The <number:text> element has character data content
    pub fn new_text<S: Into<String>>(text: S) -> Self {
        Self::new_with_content(FormatPartType::Text, text)
    }

    /// The <number:text-content> element marks the position of variable text content of a text
    /// style.
    ///
    /// The <number:text-content> element is usable within the following element:
    /// * <number:text-style> 16.27.25.
    ///
    /// The <number:text-content> element has no attributes.
    /// The <number:text-content> element has no child elements.
    pub fn new_text_content() -> Self {
        Self::new(FormatPartType::TextContent)
    }

    // The <number:embedded-text> element specifies text that is displayed at one specific position
    // within a number.
    //
    // The <number:embedded-text> element is usable within the following element:
    // * <number:number> 16.27.3.
    //
    // The <number:embedded-text> element has the following attribute:
    // * number:position 19.354.
    //
    // The <number:embedded-text> element has no child elements.
    // The <number:embedded-text> element has character data content.
    // pub fn new_embedded_text<S: Into<String>>(position: u8, text: S) -> Self {
    //     let mut p = Self::new(FormatPartType::EmbeddedText);
    //     p.set_attr("number:position", position.to_string());
    //     p.set_content(text);
    //     p
    // }

    /// Sets the kind of the part.
    pub fn set_part_type(&mut self, p_type: FormatPartType) {
        self.part_type = p_type;
    }

    /// What kind of part?
    pub fn part_type(&self) -> FormatPartType {
        self.part_type
    }

    /// General attributes.
    pub fn attrmap(&self) -> &AttrMap2 {
        &self.attr
    }

    /// General attributes.
    pub fn attrmap_mut(&mut self) -> &mut AttrMap2 {
        &mut self.attr
    }

    /// Adds an attribute.
    pub fn set_attr(&mut self, name: &str, value: String) {
        self.attr.set_attr(name, value);
    }

    /// Returns a property or a default.
    pub fn attr_def<'a0, 'a1, S0, S1>(&'a1 self, name: S0, default: S1) -> &'a1 str
    where
        S0: Into<&'a0 str>,
        S1: Into<&'a1 str>,
    {
        if let Some(v) = self.attr.attr(name.into()) {
            v
        } else {
            default.into()
        }
    }

    /// Sets a textual content for this part. This is only used
    /// for text and currency-symbol.
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        self.content = Some(content.into());
    }

    /// Returns the text content.
    pub fn content(&self) -> Option<&String> {
        self.content.as_ref()
    }

    /// Tries to format the given boolean, and appends the result to buf.
    /// If this part does'nt match does nothing
    fn format_boolean(&self, buf: &mut String, b: bool) {
        match self.part_type {
            FormatPartType::Boolean => {
                buf.push_str(if b { "true" } else { "false" });
            }
            FormatPartType::Text => {
                if let Some(content) = &self.content {
                    buf.push_str(content)
                }
            }
            _ => {}
        }
    }

    /// Tries to format the given float, and appends the result to buf.
    /// If this part does'nt match does nothing
    fn format_float(&self, buf: &mut String, f: f64) {
        match self.part_type {
            FormatPartType::Number => {
                let dec = self.attr_def("number:decimal-places", "0").parse::<usize>();
                if let Ok(dec) = dec {
                    buf.push_str(&format!("{:.*}", dec, f));
                }
            }
            FormatPartType::ScientificNumber => {
                buf.push_str(&format!("{:e}", f));
            }
            FormatPartType::CurrencySymbol => {
                if let Some(content) = &self.content {
                    buf.push_str(content)
                }
            }
            FormatPartType::Text => {
                if let Some(content) = &self.content {
                    buf.push_str(content)
                }
            }
            _ => {}
        }
    }

    /// Tries to format the given string, and appends the result to buf.
    /// If this part does'nt match does nothing
    fn format_str(&self, buf: &mut String, s: &str) {
        match self.part_type {
            FormatPartType::TextContent => {
                buf.push_str(s);
            }
            FormatPartType::Text => {
                if let Some(content) = &self.content {
                    buf.push_str(content)
                }
            }
            _ => {}
        }
    }

    /// Tries to format the given DateTime, and appends the result to buf.
    /// Uses chrono::strftime for the implementation.
    /// If this part does'nt match does nothing
    #[allow(clippy::collapsible_else_if)]
    fn format_datetime(&self, buf: &mut String, d: &NaiveDateTime, h12: bool) {
        match self.part_type {
            FormatPartType::Day => {
                let is_long = self.attr_def("number:style", "") == "long";
                if is_long {
                    buf.push_str(&d.format("%d").to_string());
                } else {
                    buf.push_str(&d.format("%-d").to_string());
                }
            }
            FormatPartType::Month => {
                let is_long = self.attr_def("number:style", "") == "long";
                let is_text = self.attr_def("number:textual", "") == "true";
                if is_text {
                    if is_long {
                        buf.push_str(&d.format("%b").to_string());
                    } else {
                        buf.push_str(&d.format("%B").to_string());
                    }
                } else {
                    if is_long {
                        buf.push_str(&d.format("%m").to_string());
                    } else {
                        buf.push_str(&d.format("%-m").to_string());
                    }
                }
            }
            FormatPartType::Year => {
                let is_long = self.attr_def("number:style", "") == "long";
                if is_long {
                    buf.push_str(&d.format("%Y").to_string());
                } else {
                    buf.push_str(&d.format("%y").to_string());
                }
            }
            FormatPartType::DayOfWeek => {
                let is_long = self.attr_def("number:style", "") == "long";
                if is_long {
                    buf.push_str(&d.format("%A").to_string());
                } else {
                    buf.push_str(&d.format("%a").to_string());
                }
            }
            FormatPartType::WeekOfYear => {
                let is_long = self.attr_def("number:style", "") == "long";
                if is_long {
                    buf.push_str(&d.format("%W").to_string());
                } else {
                    buf.push_str(&d.format("%-W").to_string());
                }
            }
            FormatPartType::Hours => {
                let is_long = self.attr_def("number:style", "") == "long";
                if !h12 {
                    if is_long {
                        buf.push_str(&d.format("%H").to_string());
                    } else {
                        buf.push_str(&d.format("%-H").to_string());
                    }
                } else {
                    if is_long {
                        buf.push_str(&d.format("%I").to_string());
                    } else {
                        buf.push_str(&d.format("%-I").to_string());
                    }
                }
            }
            FormatPartType::Minutes => {
                let is_long = self.attr_def("number:style", "") == "long";
                if is_long {
                    buf.push_str(&d.format("%M").to_string());
                } else {
                    buf.push_str(&d.format("%-M").to_string());
                }
            }
            FormatPartType::Seconds => {
                let is_long = self.attr_def("number:style", "") == "long";
                if is_long {
                    buf.push_str(&d.format("%S").to_string());
                } else {
                    buf.push_str(&d.format("%-S").to_string());
                }
            }
            FormatPartType::AmPm => {
                buf.push_str(&d.format("%p").to_string());
            }
            FormatPartType::Text => {
                if let Some(content) = &self.content {
                    buf.push_str(content)
                }
            }
            _ => {}
        }
    }

    /// Tries to format the given Duration, and appends the result to buf.
    /// If this part does'nt match does nothing
    fn format_time_duration(&self, buf: &mut String, d: &Duration) {
        match self.part_type {
            FormatPartType::Hours => {
                buf.push_str(&d.num_hours().to_string());
            }
            FormatPartType::Minutes => {
                buf.push_str(&(d.num_minutes() % 60).to_string());
            }
            FormatPartType::Seconds => {
                buf.push_str(&(d.num_seconds() % 60).to_string());
            }
            FormatPartType::Text => {
                if let Some(content) = &self.content {
                    buf.push_str(content)
                }
            }
            _ => {}
        }
    }
}

/// Creates a new number format.
pub fn create_loc_boolean_format<S: Into<String>>(name: S, locale: Locale) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::Boolean);
    v.push_boolean();
    v
}

/// Creates a new number format.
pub fn create_loc_number_format<S: Into<String>>(
    name: S,
    locale: Locale,
    decimal: u8,
    grouping: bool,
) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::Number);
    v.push_number(decimal, grouping);
    v
}

/// Creates a new percentage format.
pub fn create_loc_percentage_format<S: Into<String>>(
    name: S,
    locale: Locale,
    decimal: u8,
) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::Percentage);
    v.push_number_fix(decimal, false);
    v.push_text("%");
    v
}

/// Creates a new currency format.
pub fn create_loc_currency_prefix<S1, S2>(
    name: S1,
    locale: Locale,
    symbol_locale: Locale,
    symbol: S2,
) -> ValueFormat
where
    S1: Into<String>,
    S2: Into<String>,
{
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::Currency);
    v.push_currency_symbol(symbol_locale, symbol.into());
    v.push_text(" ");
    v.push_number_fix(2, true);
    v
}

/// Creates a new currency format.
pub fn create_loc_currency_suffix<S1, S2>(
    name: S1,
    locale: Locale,
    symbol_locale: Locale,
    symbol: S2,
) -> ValueFormat
where
    S1: Into<String>,
    S2: Into<String>,
{
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::Currency);
    v.push_number_fix(2, true);
    v.push_text(" ");
    v.push_currency_symbol(symbol_locale, symbol.into());
    v
}

/// Creates a new date format D.M.Y
pub fn create_loc_date_dmy_format<S: Into<String>>(name: S, locale: Locale) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::DateTime);
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text(".");
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text(".");
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v
}

/// Creates a new date format M/D/Y
pub fn create_loc_date_mdy_format<S: Into<String>>(name: S, locale: Locale) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::DateTime);
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text("/");
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text("/");
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v
}

/// Creates a datetime format Y-M-D H:M:S
pub fn create_loc_datetime_format<S: Into<String>>(name: S, locale: Locale) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::DateTime);
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text(".");
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text(".");
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text(" ");
    v.push_hours(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_minutes(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_seconds(FormatNumberStyle::Long, 0);
    v
}

/// Creates a new time-Duration format H:M:S
pub fn create_loc_time_format<S: Into<String>>(name: S, locale: Locale) -> ValueFormat {
    let mut v = ValueFormat::new_localized(name.into(), locale, ValueType::TimeDuration);
    v.push_hours(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_minutes(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_seconds(FormatNumberStyle::Long, 0);
    v
}

/// Creates a new number format.
pub fn create_boolean_format<S: Into<String>>(name: S) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::Boolean);
    v.push_boolean();
    v
}

/// Creates a new number format.
pub fn create_number_format<S: Into<String>>(name: S, decimal: u8, grouping: bool) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::Number);
    v.push_number(decimal, grouping);
    v
}

/// Creates a new number format with a fixed number of decimal places.
pub fn create_number_format_fixed<S: Into<String>>(
    name: S,
    decimal: u8,
    grouping: bool,
) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::Number);
    v.push_number_fix(decimal, grouping);
    v
}

/// Creates a new percentage format.
pub fn create_percentage_format<S: Into<String>>(name: S, decimal: u8) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::Percentage);
    v.push_number_fix(decimal, false);
    v.push_text("%");
    v
}

/// Creates a new currency format.
pub fn create_currency_prefix<S1, S2>(name: S1, symbol_locale: Locale, symbol: S2) -> ValueFormat
where
    S1: Into<String>,
    S2: Into<String>,
{
    let mut v = ValueFormat::new_named(name.into(), ValueType::Currency);
    v.push_currency_symbol(symbol_locale, symbol.into());
    v.push_text(" ");
    v.push_number_fix(2, true);
    v
}

/// Creates a new currency format.
pub fn create_currency_suffix<S1, S2>(name: S1, symbol_locale: Locale, symbol: S2) -> ValueFormat
where
    S1: Into<String>,
    S2: Into<String>,
{
    let mut v = ValueFormat::new_named(name.into(), ValueType::Currency);
    v.push_number_fix(2, true);
    v.push_text(" ");
    v.push_currency_symbol(symbol_locale, symbol.into());
    v
}

/// Creates a new date format YYYY-MM-DD
pub fn create_date_iso_format<S: Into<String>>(name: S) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::DateTime);
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text("-");
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text("-");
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v
}

/// Creates a new date format D.M.Y
pub fn create_date_dmy_format<S: Into<String>>(name: S) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::DateTime);
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text(".");
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text(".");
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v
}

/// Creates a new date format M/D/Y
pub fn create_date_mdy_format<S: Into<String>>(name: S) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::DateTime);
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text("/");
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text("/");
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v
}

/// Creates a datetime format Y-M-D H:M:S
pub fn create_datetime_format<S: Into<String>>(name: S) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::DateTime);
    v.push_year(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text("-");
    v.push_month(
        FormatNumberStyle::Long,
        FormatTextual::Numeric,
        FormatMonth::Nominativ,
        FormatCalendar::Default,
    );
    v.push_text("-");
    v.push_day(FormatNumberStyle::Long, FormatCalendar::Default);
    v.push_text(" ");
    v.push_hours(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_minutes(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_seconds(FormatNumberStyle::Long, 0);
    v
}

/// Creates a new time-Duration format H:M:S
pub fn create_time_format<S: Into<String>>(name: S) -> ValueFormat {
    let mut v = ValueFormat::new_named(name.into(), ValueType::TimeDuration);
    v.push_hours(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_minutes(FormatNumberStyle::Long);
    v.push_text(":");
    v.push_seconds(FormatNumberStyle::Long, 0);
    v
}
