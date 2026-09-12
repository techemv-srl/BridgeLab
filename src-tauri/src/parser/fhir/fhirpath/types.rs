//! Type inspection for FHIRPath over FHIR's JSON encoding.
//!
//! FHIR's JSON representation is lossy about types: a `date`, a `code` and a
//! free-text `string` are all JSON strings, and a `Quantity` is just an
//! object. A conforming implementation recovers the declared type from the
//! StructureDefinitions; without those loaded, types are inferred from the
//! JSON shape. That covers the cases people actually write — `ofType(Quantity)`
//! on a choice element, `is Boolean`, date comparisons — and the inference
//! rules are documented here so the limits are visible.

use std::cmp::Ordering;

use serde_json::{Map, Value};

/// Name reported by `type()` and used in error messages.
pub fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "empty",
        Value::Bool(_) => "Boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "Integer"
            } else {
                "Decimal"
            }
        }
        Value::String(s) => match as_temporal(s) {
            Some(t) => match t.kind {
                TemporalKind::Date => "Date",
                TemporalKind::DateTime => "DateTime",
                TemporalKind::Time => "Time",
            },
            None => "String",
        },
        Value::Array(_) => "Collection",
        Value::Object(o) => {
            if o.contains_key("resourceType") {
                "Resource"
            } else if is_quantity_map(o) {
                "Quantity"
            } else {
                "Element"
            }
        }
    }
}

/// Namespace and name as reported by `type()`. System primitives live in
/// `System`; everything recovered from the FHIR encoding lives in `FHIR`.
///
/// A resource reports its own `resourceType` rather than the generic
/// "Resource", since that is the one FHIR type the JSON does carry.
pub fn namespaced_type(v: &Value) -> (&'static str, String) {
    if let Some(rt) = v.get("resourceType").and_then(|t| t.as_str()) {
        return ("FHIR", rt.to_string());
    }
    let name = type_name(v);
    let namespace = match name {
        "Boolean" | "Integer" | "Decimal" | "String" | "Date" | "DateTime" | "Time" => "System",
        _ => "FHIR",
    };
    (namespace, name.to_string())
}

/// Does `value` match the (optionally namespace-qualified) `type_name`?
///
/// Resource types are matched against `resourceType`, so `Bundle.entry
/// .resource.ofType(Patient)` works exactly. Element and system types fall
/// back to shape inference.
pub fn is_type(value: &Value, type_name: &str) -> bool {
    // `System.` restricts the match to the primitive types; no FHIR resource
    // or element lives in that namespace, so `Patient is System.Patient` is
    // false however the JSON looks.
    let system_only = type_name.starts_with("System.");
    let name = type_name
        .strip_prefix("System.")
        .or_else(|| type_name.strip_prefix("FHIR."))
        .unwrap_or(type_name);

    // A resource is its own declared type, and also matches the abstract
    // supertypes every resource has.
    if let Some(rt) = value.get("resourceType").and_then(|v| v.as_str()) {
        if system_only {
            return false;
        }
        if rt == name {
            return true;
        }
        if matches!(name, "Resource" | "DomainResource" | "Any") {
            return true;
        }
        return false;
    }

    match name {
        "Any" => true,
        "Boolean" | "boolean" => value.is_boolean(),
        "Integer" | "integer" | "positiveInt" | "unsignedInt" => value.is_i64() || value.is_u64(),
        // Integer and Decimal are distinct types even though an Integer
        // converts implicitly to one, so `1 is Decimal` is false.
        "Decimal" | "decimal" => value.is_f64(),
        "Quantity" | "Age" | "Duration" | "Count" | "Distance" | "Money" | "SimpleQuantity" => {
            is_quantity(value)
        }
        "Date" | "date" => matches_temporal(value, TemporalKind::Date),
        "DateTime" | "dateTime" | "instant" => matches_temporal(value, TemporalKind::DateTime),
        "Time" | "time" => matches_temporal(value, TemporalKind::Time),
        // Every FHIR primitive that serialises as a JSON string.
        "String" | "string" | "code" | "uri" | "url" | "canonical" | "id" | "oid" | "uuid"
        | "markdown" | "base64Binary" | "xhtml" => value.is_string(),
        // Complex types are recognised by their characteristic fields; this
        // is inference, not a StructureDefinition lookup.
        "Coding" => is_coding(value),
        "CodeableConcept" => value.get("coding").is_some() || value.get("text").is_some(),
        "Period" => value.get("start").is_some() || value.get("end").is_some(),
        "Reference" => value.get("reference").is_some(),
        "Identifier" => value.get("system").is_some() && value.get("value").is_some(),
        "HumanName" => value.get("family").is_some() || value.get("given").is_some(),
        "Range" => value.get("low").is_some() || value.get("high").is_some(),
        "Ratio" => value.get("numerator").is_some() || value.get("denominator").is_some(),
        // Unknown or unmodelled type: only an object can be one, and we
        // cannot tell which, so say no rather than guess yes.
        _ => false,
    }
}

fn is_coding(v: &Value) -> bool {
    let Some(o) = v.as_object() else { return false };
    o.contains_key("code") && !o.contains_key("value") && !o.contains_key("coding")
}

pub fn is_quantity(v: &Value) -> bool {
    v.as_object().is_some_and(is_quantity_map)
}

fn is_quantity_map(o: &Map<String, Value>) -> bool {
    o.get("value").is_some_and(|v| v.is_number())
        && (o.contains_key("unit") || o.contains_key("code") || o.contains_key("system"))
}

/// Numeric value and unit of a quantity, for comparison and arithmetic.
///
/// The unit prefers the UCUM `code` over the human-readable `unit`, and
/// calendar words are mapped to their UCUM equivalents so `1 week` and
/// `1 'wk'` are the same quantity.
pub fn quantity_parts(v: &Value) -> Option<(f64, String)> {
    let o = v.as_object()?;
    let value = o.get("value")?.as_f64()?;
    let raw = quantity_raw_unit(v)?;
    let unit = ucum_for_time_unit(&raw).map(str::to_string).unwrap_or(raw);
    Some((value, unit))
}

/// The unit exactly as stored, which is what `toString()` renders.
pub fn quantity_raw_unit(v: &Value) -> Option<String> {
    let o = v.as_object()?;
    Some(
        o.get("code")
            .and_then(|u| u.as_str())
            .or_else(|| o.get("unit").and_then(|u| u.as_str()))
            .unwrap_or("")
            .to_string(),
    )
}

pub fn make_quantity(value: f64, unit: &str) -> Value {
    let mut m = Map::new();
    // Keep an integral magnitude an integer so `4 'mg'` does not render as
    // `4.0 'mg'` on the way back out.
    m.insert(
        "value".into(),
        if value.fract() == 0.0 && value.abs() < 9e15 {
            Value::from(value as i64)
        } else {
            Value::from(value)
        },
    );
    m.insert("unit".into(), Value::from(unit.to_string()));
    Value::Object(m)
}

/// `4 'mg'` — the rendering `toString()` produces for a quantity. Calendar
/// units keep their word and their lack of quotes: `1 week`.
pub fn quantity_to_string(v: &Value) -> Option<String> {
    if !is_quantity(v) {
        return None;
    }
    let value = v.get("value")?.as_f64()?;
    let unit = quantity_raw_unit(v)?;
    let magnitude = if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    Some(if ucum_for_time_unit(&unit).is_some() {
        format!("{} {}", magnitude, unit)
    } else {
        format!("{} '{}'", magnitude, unit)
    })
}

/// Factor and base unit for the UCUM units BridgeLab converts between.
///
/// This is a deliberately small table — metric mass, length and volume plus
/// the time units — not a UCUM engine. Anything outside it keeps its own
/// unit, so comparing it against a different one stays indeterminate rather
/// than silently wrong.
pub fn ucum_scale(unit: &str) -> Option<(f64, &'static str)> {
    // Time first: the calendar words normalise into it.
    let unit = ucum_for_time_unit(unit).unwrap_or(unit);
    Some(match unit {
        "ms" => (0.001, "s"),
        "s" => (1.0, "s"),
        "min" => (60.0, "s"),
        "h" => (3600.0, "s"),
        "d" => (86_400.0, "s"),
        "wk" => (604_800.0, "s"),
        // Calendar months and years are not fixed multiples of a second, so
        // they only compare against themselves.
        "mo" | "a" => return None,

        "ng" => (1e-9, "g"),
        "ug" => (1e-6, "g"),
        "mg" => (0.001, "g"),
        "cg" => (0.01, "g"),
        "g" => (1.0, "g"),
        "kg" => (1000.0, "g"),

        "nm" => (1e-9, "m"),
        "um" => (1e-6, "m"),
        "mm" => (0.001, "m"),
        "cm" => (0.01, "m"),
        "dm" => (0.1, "m"),
        "m" => (1.0, "m"),
        "km" => (1000.0, "m"),
        "[in_i]" => (0.0254, "m"),
        "[ft_i]" => (0.3048, "m"),

        "[lb_av]" => (453.592_37, "g"),
        "[oz_av]" => (28.349_523_125, "g"),

        "uL" | "ul" => (1e-6, "L"),
        "mL" | "ml" => (0.001, "L"),
        "dL" | "dl" => (0.1, "L"),
        "L" | "l" => (1.0, "L"),

        _ => return None,
    })
}

/// The magnitude of `value` expressed in `to`, when both units share a base.
pub fn convert_quantity(value: f64, from: &str, to: &str) -> Option<f64> {
    if from == to {
        return Some(value);
    }
    let (from_factor, from_base) = ucum_scale(from)?;
    let (to_factor, to_base) = ucum_scale(to)?;
    (from_base == to_base).then(|| value * from_factor / to_factor)
}

/// Calendar durations have UCUM equivalents; FHIRPath treats `1 day` and
/// `1 'd'` as the same quantity.
pub fn ucum_for_time_unit(word: &str) -> Option<&'static str> {
    Some(match word.trim_end_matches('s') {
        "year" => "a",
        "month" => "mo",
        "week" => "wk",
        "day" => "d",
        "hour" => "h",
        "minute" => "min",
        "second" => "s",
        "millisecond" => "ms",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Date / time
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalKind {
    Date,
    DateTime,
    Time,
}

/// A partial-precision date, dateTime or time.
///
/// FHIR allows any prefix of a timestamp (`2015`, `2015-02`, `2015-02-04`,
/// …), and FHIRPath says comparing two values whose precisions differ is
/// *indeterminate* — it returns the empty collection rather than a guess —
/// unless they already differ at a precision both carry.
#[derive(Debug, Clone)]
pub struct Temporal {
    pub kind: TemporalKind,
    /// Components in descending significance, normalised to UTC-naive text:
    /// year, month, day, hour, minute, second (with fraction).
    parts: Vec<String>,
    /// Timezone offset in minutes, when the literal carried one.
    offset: Option<i32>,
}

fn matches_temporal(v: &Value, kind: TemporalKind) -> bool {
    v.as_str()
        .and_then(as_temporal)
        .is_some_and(|t| t.kind == kind)
}

/// Parse a time for the explicit `toTime()` / `convertsToTime()`
/// conversions, which accept a bare hour (`'14'`) that [`as_temporal`]
/// deliberately does not — otherwise every two-digit string in a resource
/// would infer as a Time.
pub fn as_time_lenient(s: &str) -> Option<Temporal> {
    if let Some(t) = as_temporal(s) {
        return (t.kind == TemporalKind::Time).then_some(t);
    }
    let body = s.strip_prefix('T').unwrap_or(s);
    if body.len() == 2 && body.bytes().all(|b| b.is_ascii_digit()) {
        return Some(Temporal {
            kind: TemporalKind::Time,
            parts: vec![body.to_string()],
            offset: None,
        });
    }
    None
}

/// Parse a FHIR/FHIRPath date, dateTime or time literal. Returns `None` for
/// anything that is not shaped like one, so ordinary strings stay strings.
pub fn as_temporal(s: &str) -> Option<Temporal> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(time) = s.strip_prefix('T') {
        let (parts, offset) = split_time(time)?;
        return Some(Temporal {
            kind: TemporalKind::Time,
            parts,
            offset,
        });
    }

    // A bare time, as written in FHIR `time` elements.
    if s.as_bytes()[0].is_ascii_digit() && s.contains(':') && !s.contains('-') {
        let (parts, offset) = split_time(s)?;
        return Some(Temporal {
            kind: TemporalKind::Time,
            parts,
            offset,
        });
    }

    let (date_part, time_part) = match s.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };

    let mut parts = split_date(date_part)?;
    let mut offset = None;
    if let Some(t) = time_part {
        // A trailing 'T' with nothing after it is a DateTime at date
        // precision: `@2015T`, `@2015-02-04T`.
        if !t.is_empty() {
            let (tp, off) = split_time(t)?;
            parts.extend(tp);
            offset = off;
        }
        return Some(Temporal {
            kind: TemporalKind::DateTime,
            parts,
            offset,
        });
    }
    Some(Temporal {
        kind: TemporalKind::Date,
        parts,
        offset,
    })
}

fn split_date(s: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for (i, piece) in s.split('-').enumerate() {
        if i > 2 || piece.is_empty() || !piece.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let expected = if i == 0 { 4 } else { 2 };
        if piece.len() != expected {
            return None;
        }
        parts.push(piece.to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

fn split_time(s: &str) -> Option<(Vec<String>, Option<i32>)> {
    let (body, offset) = match s.find(['Z', '+']) {
        Some(i) => (&s[..i], parse_offset(&s[i..])?),
        None => {
            // A '-' can only be an offset after at least hh:mm.
            match s.rfind('-') {
                Some(i) if i > 2 => (&s[..i], parse_offset(&s[i..])?),
                _ => (s, None),
            }
        }
    };

    let mut parts = Vec::new();
    for (i, piece) in body.split(':').enumerate() {
        if i > 2 || piece.is_empty() {
            return None;
        }
        let valid = if i == 2 {
            piece.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        } else {
            piece.len() == 2 && piece.bytes().all(|b| b.is_ascii_digit())
        };
        if !valid {
            return None;
        }
        parts.push(piece.to_string());
    }
    if parts.is_empty() {
        return None;
    }
    Some((parts, offset))
}

fn parse_offset(s: &str) -> Option<Option<i32>> {
    if s == "Z" {
        return Some(Some(0));
    }
    let sign = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (h, m) = s[1..].split_once(':')?;
    let h: i32 = h.parse().ok()?;
    let m: i32 = m.parse().ok()?;
    Some(Some(sign * (h * 60 + m)))
}

impl Temporal {
    /// The value with its timezone offset folded into the components, so two
    /// zoned timestamps compare as instants rather than as wall-clock text.
    ///
    /// Only possible at minute precision or finer; a zoned value that stops
    /// at the date keeps its components and stays subject to the precision
    /// rules below.
    fn normalized(&self) -> Vec<String> {
        let Some(offset) = self.offset else {
            return self.parts.clone();
        };
        if offset == 0 || self.parts.len() < 5 || self.kind == TemporalKind::Time {
            return self.parts.clone();
        }
        let (Ok(y), Ok(mo), Ok(d), Ok(h), Ok(mi)) = (
            self.parts[0].parse::<i32>(),
            self.parts[1].parse::<u32>(),
            self.parts[2].parse::<u32>(),
            self.parts[3].parse::<u32>(),
            self.parts[4].parse::<u32>(),
        ) else {
            return self.parts.clone();
        };
        let Some(naive) = chrono::NaiveDate::from_ymd_opt(y, mo, d)
            .and_then(|date| date.and_hms_opt(h, mi, 0))
        else {
            return self.parts.clone();
        };
        let utc = naive - chrono::Duration::minutes(offset as i64);
        let mut out = vec![
            utc.format("%Y").to_string(),
            utc.format("%m").to_string(),
            utc.format("%d").to_string(),
            utc.format("%H").to_string(),
            utc.format("%M").to_string(),
        ];
        // Seconds are unaffected by a whole-minute offset, so carry them
        // across verbatim to keep any fractional part.
        out.extend(self.parts.iter().skip(5).cloned());
        out
    }
}

impl Temporal {
    /// Rebuild the literal text, preserving the original precision and zone.
    pub fn to_literal(&self) -> String {
        let mut s = String::new();
        match self.kind {
            TemporalKind::Time => {
                s.push_str(&self.parts.join(":"));
            }
            _ => {
                let date_len = self.parts.len().min(3);
                s.push_str(&self.parts[..date_len].join("-"));
                if self.parts.len() > 3 {
                    s.push('T');
                    s.push_str(&self.parts[3..].join(":"));
                } else if self.kind == TemporalKind::DateTime {
                    // A DateTime that stops at the date still carries the 'T'.
                    s.push('T');
                }
            }
        }
        match self.offset {
            None => {}
            Some(0) => s.push('Z'),
            Some(o) => {
                let sign = if o < 0 { '-' } else { '+' };
                s.push_str(&format!("{}{:02}:{:02}", sign, o.abs() / 60, o.abs() % 60));
            }
        }
        s
    }

    /// Number of significant digits, as `precision()` reports it: 4 for a
    /// year, 6 for a month, 8 for a day, then two per time component and
    /// three for milliseconds. A time starts from its hour instead.
    pub fn precision(&self) -> i64 {
        let mut total = 0i64;
        for (i, part) in self.parts.iter().enumerate() {
            let index = if self.kind == TemporalKind::Time { i + 3 } else { i };
            total += match index {
                0 => 4,
                5 | 6 => {
                    // Seconds, possibly with a fraction.
                    match part.split_once('.') {
                        Some((_, frac)) => 2 + frac.len() as i64,
                        None => 2,
                    }
                }
                _ => 2,
            };
        }
        total
    }

    /// Add a duration expressed in `unit`, keeping the original precision.
    ///
    /// Calendar units (year, month) move the corresponding component;
    /// everything else is added as elapsed time.
    pub fn add_duration(&self, amount: f64, unit: &str) -> Option<Temporal> {
        let canonical = ucum_for_time_unit(unit).unwrap_or(unit);
        // A duration counts whole units of its own unit: `+ 7.7 days` moves
        // seven days, `+ 0.1 's'` moves nothing.
        let amount = amount.trunc();

        if matches!(canonical, "a" | "mo") {
            return self.add_calendar(amount as i64, canonical);
        }

        let (factor, base) = ucum_scale(canonical)?;
        if base != "s" {
            return None;
        }
        let seconds = amount * factor;
        self.add_seconds(seconds)
    }

    fn add_calendar(&self, amount: i64, unit: &str) -> Option<Temporal> {
        let mut parts = self.parts.clone();
        if self.kind == TemporalKind::Time {
            return None;
        }
        let years: i64 = parts.first()?.parse().ok()?;
        if unit == "a" {
            parts[0] = format!("{:04}", years + amount);
            return Some(Temporal { parts, ..self.clone() });
        }
        // Months: only meaningful once the value carries a month.
        if parts.len() < 2 {
            return None;
        }
        let months: i64 = parts[1].parse().ok()?;
        let total = years * 12 + (months - 1) + amount;
        parts[0] = format!("{:04}", total.div_euclid(12));
        parts[1] = format!("{:02}", total.rem_euclid(12) + 1);
        // Clamp the day to the new month's length (31 January + 1 month).
        if parts.len() >= 3 {
            let day: u32 = parts[2].parse().ok()?;
            let last = days_in_month(total.div_euclid(12) as i32, total.rem_euclid(12) as u32 + 1)?;
            parts[2] = format!("{:02}", day.min(last));
        }
        Some(Temporal { parts, ..self.clone() })
    }

    fn add_seconds(&self, seconds: f64) -> Option<Temporal> {
        use chrono::{Datelike, Duration, NaiveDate};

        // Below day precision the arithmetic is on whole days, so the value
        // keeps its shape instead of gaining components it never had.
        if self.kind != TemporalKind::Time && self.parts.len() <= 3 {
            let days = (seconds / 86_400.0).trunc() as i64;
            return self.add_days(days);
        }

        let nanos = (seconds * 1e9).round() as i64;
        let delta = Duration::nanoseconds(nanos);

        if self.kind == TemporalKind::Time {
            let (h, m, s, frac_len) = self.time_components()?;
            let base = NaiveDate::from_ymd_opt(2000, 1, 1)?.and_hms_nano_opt(h, m, s.0, s.1)?;
            let moved = base + delta;
            return Some(Temporal {
                parts: render_time(&moved, self.parts.len(), frac_len),
                ..self.clone()
            });
        }

        let (y, mo, d) = (
            self.parts[0].parse::<i32>().ok()?,
            self.parts[1].parse::<u32>().ok()?,
            self.parts[2].parse::<u32>().ok()?,
        );
        let (h, m, s, frac_len) = self.time_components()?;
        let base = NaiveDate::from_ymd_opt(y, mo, d)?.and_hms_nano_opt(h, m, s.0, s.1)?;
        let moved = base + delta;
        let mut parts = vec![
            format!("{:04}", moved.year()),
            format!("{:02}", moved.month()),
            format!("{:02}", moved.day()),
        ];
        parts.extend(render_time(&moved, self.parts.len() - 3, frac_len));
        Some(Temporal { parts, ..self.clone() })
    }

    fn add_days(&self, days: i64) -> Option<Temporal> {
        use chrono::{Datelike, Duration, NaiveDate};
        let mut parts = self.parts.clone();
        let y: i32 = parts.first()?.parse().ok()?;
        let mo: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1);
        let d: u32 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(1);
        let moved = NaiveDate::from_ymd_opt(y, mo, d)? + Duration::days(days);
        parts[0] = format!("{:04}", moved.year());
        if parts.len() > 1 {
            parts[1] = format!("{:02}", moved.month());
        }
        if parts.len() > 2 {
            parts[2] = format!("{:02}", moved.day());
        }
        Some(Temporal { parts, ..self.clone() })
    }

    /// (hour, minute, (second, nanosecond), fractional digits)
    fn time_components(&self) -> Option<(u32, u32, (u32, u32), usize)> {
        let offset = if self.kind == TemporalKind::Time { 0 } else { 3 };
        let get = |i: usize| self.parts.get(offset + i);
        let h: u32 = get(0).and_then(|p| p.parse().ok()).unwrap_or(0);
        let m: u32 = get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let (s, nanos, frac_len) = match get(2) {
            None => (0, 0, 0),
            Some(text) => match text.split_once('.') {
                None => (text.parse().ok()?, 0, 0),
                Some((whole, frac)) => {
                    let scaled: f64 = format!("0.{}", frac).parse().ok()?;
                    (whole.parse().ok()?, (scaled * 1e9).round() as u32, frac.len())
                }
            },
        };
        Some((h, m, (s, nanos), frac_len))
    }
}

fn render_time(dt: &chrono::NaiveDateTime, components: usize, frac_len: usize) -> Vec<String> {
    use chrono::Timelike;
    let mut out = Vec::new();
    if components >= 1 {
        out.push(format!("{:02}", dt.hour()));
    }
    if components >= 2 {
        out.push(format!("{:02}", dt.minute()));
    }
    if components >= 3 {
        if frac_len > 0 {
            let frac = dt.nanosecond() as f64 / 1e9;
            let text = format!("{:.*}", frac_len, frac);
            out.push(format!("{:02}{}", dt.second(), &text[1..]));
        } else {
            out.push(format!("{:02}", dt.second()));
        }
    }
    out
}

fn days_in_month(year: i32, month: u32) -> Option<u32> {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = chrono::NaiveDate::from_ymd_opt(ny, nm, 1)?;
    let last = first_next.pred_opt()?;
    Some(chrono::Datelike::day(&last))
}

/// `=` on two temporals. `None` when their precisions differ and everything
/// they share is equal, which FHIRPath treats as indeterminate.
pub fn temporal_equal(a: &Temporal, b: &Temporal) -> Option<bool> {
    if !comparable(a.kind, b.kind) {
        return Some(false);
    }
    match temporal_compare(a, b) {
        Some(Ordering::Equal) => Some(true),
        Some(_) => Some(false),
        None => None,
    }
}

/// A Date and a DateTime are the same axis at different precisions; a Time is
/// a different one.
fn comparable(a: TemporalKind, b: TemporalKind) -> bool {
    (a == TemporalKind::Time) == (b == TemporalKind::Time)
}

/// Ordering on two temporals, `None` when indeterminate.
pub fn temporal_compare(a: &Temporal, b: &Temporal) -> Option<Ordering> {
    if !comparable(a.kind, b.kind) {
        return None;
    }

    // Once a value carries a time of day its timezone decides the instant,
    // so a zoned value and a naive one are simply not comparable. Values
    // that stop at the date have no time for a zone to shift, and compare
    // normally — which is what lets `now() > Patient.birthDate` answer.
    let has_time = |t: &Temporal| t.kind == TemporalKind::Time || t.parts.len() > 3;
    if has_time(a) && has_time(b) && a.offset.is_some() != b.offset.is_some() {
        return None;
    }

    let (x, y) = (a.normalized(), b.normalized());
    let shared = x.len().min(y.len());
    for i in 0..shared {
        let ord = compare_component(i, &x[i], &y[i]);
        if ord != Ordering::Equal {
            return Some(ord);
        }
    }

    if x.len() != y.len() {
        // Equal so far but one side is more precise: indeterminate.
        return None;
    }

    // Equal wall-clock text in different zones that could not be normalised
    // (a zoned value below minute precision): the instants stay unknown.
    if a.offset != b.offset && x == a.parts && y == b.parts {
        return None;
    }

    Some(Ordering::Equal)
}

/// Seconds may carry a fraction, so compare numerically whenever either side
/// has one — `00` and `00.0` are the same second. Every other component is
/// fixed-width and compares as text.
fn compare_component(_index: usize, a: &str, b: &str) -> Ordering {
    if a.contains('.') || b.contains('.') {
        let (x, y): (f64, f64) = (a.parse().unwrap_or(0.0), b.parse().unwrap_or(0.0));
        return x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    }
    a.cmp(b)
}

/// `~` on two temporals: compare only the precision they share, so
/// `@2012-04-15T15:30:31` and `@2012-04-15T15:30:31.0` are equivalent.
pub fn temporal_equivalent(a: &Temporal, b: &Temporal) -> bool {
    if !comparable(a.kind, b.kind) {
        return false;
    }
    let (x, y) = (a.normalized(), b.normalized());
    // Equivalence needs the same components — a date is not equivalent to a
    // dateTime — but compares them numerically, so `:31` and `:31.0` match.
    x.len() == y.len() && (0..x.len()).all(|i| compare_component(i, &x[i], &y[i]) == Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infers_system_types_from_json() {
        assert_eq!(type_name(&json!(true)), "Boolean");
        assert_eq!(type_name(&json!(3)), "Integer");
        assert_eq!(type_name(&json!(3.5)), "Decimal");
        assert_eq!(type_name(&json!("hello")), "String");
        assert_eq!(type_name(&json!("2015-02-04")), "Date");
        assert_eq!(type_name(&json!("2015-02-04T14:34:28Z")), "DateTime");
    }

    #[test]
    fn a_resource_matches_its_resource_type_and_the_abstract_ones() {
        let p = json!({"resourceType": "Patient", "id": "p1"});
        assert!(is_type(&p, "Patient"));
        assert!(is_type(&p, "FHIR.Patient"));
        assert!(is_type(&p, "Resource"));
        assert!(is_type(&p, "DomainResource"));
        assert!(!is_type(&p, "Observation"));
        assert!(!is_type(&p, "Quantity"));
    }

    #[test]
    fn recognises_quantities_by_shape() {
        let q = json!({"value": 6.0, "unit": "mg", "system": "http://unitsofmeasure.org", "code": "mg"});
        assert!(is_quantity(&q));
        assert!(is_type(&q, "Quantity"));
        assert_eq!(quantity_parts(&q), Some((6.0, "mg".into())));
        // An Identifier has a value but no numeric one.
        assert!(!is_quantity(&json!({"system": "urn:oid:1.2", "value": "12345"})));
    }

    #[test]
    fn quantity_unit_prefers_the_ucum_code() {
        let q = json!({"value": 1, "unit": "milligram", "code": "mg"});
        assert_eq!(quantity_parts(&q).unwrap().1, "mg");
    }

    #[test]
    fn parses_partial_precision_dates() {
        assert_eq!(as_temporal("2015").unwrap().kind, TemporalKind::Date);
        assert_eq!(as_temporal("2015-02").unwrap().kind, TemporalKind::Date);
        assert_eq!(as_temporal("2015-02-04").unwrap().kind, TemporalKind::Date);
        assert_eq!(
            as_temporal("2015-02-04T14:34:28Z").unwrap().kind,
            TemporalKind::DateTime
        );
        assert_eq!(as_temporal("T14:34").unwrap().kind, TemporalKind::Time);
        assert!(as_temporal("hello").is_none());
        assert!(as_temporal("2015-2-4").is_none());
        assert!(as_temporal("").is_none());
    }

    #[test]
    fn differing_precision_is_indeterminate_only_while_equal() {
        let a = as_temporal("2015-02-04").unwrap();
        let b = as_temporal("2015-02").unwrap();
        assert_eq!(temporal_equal(&a, &b), None, "equal so far, less precise");

        let c = as_temporal("2016-03").unwrap();
        assert_eq!(
            temporal_equal(&a, &c),
            Some(false),
            "already differs at a shared component"
        );
        assert_eq!(temporal_compare(&a, &c), Some(Ordering::Less));
    }

    #[test]
    fn same_precision_dates_compare_normally() {
        let a = as_temporal("2015-02-04").unwrap();
        let b = as_temporal("2015-02-04").unwrap();
        assert_eq!(temporal_equal(&a, &b), Some(true));
        let c = as_temporal("2015-02-05").unwrap();
        assert_eq!(temporal_compare(&a, &c), Some(Ordering::Less));
    }

    #[test]
    fn a_date_never_equals_a_time() {
        let d = as_temporal("2015-02-04").unwrap();
        let t = as_temporal("T14:34:28").unwrap();
        assert_eq!(temporal_equal(&d, &t), Some(false));
    }

    #[test]
    fn fractional_seconds_compare_numerically() {
        let a = as_temporal("T14:34:28.1").unwrap();
        let b = as_temporal("T14:34:28.09").unwrap();
        assert_eq!(temporal_compare(&a, &b), Some(Ordering::Greater));
    }
}
