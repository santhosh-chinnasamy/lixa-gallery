//! EXIF shooting settings via `kamadak-exif`.
//!
//! The crate is pure Rust (MIT OR Apache-2.0) and reads JPEG, TIFF, and the
//! TIFF-based RAW containers. Formats it cannot open (CR3, some RAF files)
//! fall back to the EXIF segment inside the embedded preview JPEG.

use gallery_core::formats::is_raw_path;
use gallery_core::models::ExifInfo;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ParsedExif {
    pub info: ExifInfo,
    /// Raw orientation tag, 1–8, when the file has one.
    pub orientation_value: Option<u16>,
}

impl ParsedExif {
    pub(crate) fn has_shooting_settings(&self) -> bool {
        self.info.camera.is_some()
            || self.info.date.is_some()
            || self.info.shutter.is_some()
            || self.info.iso.is_some()
    }

    pub(crate) fn fill_gaps_from(&mut self, other: &ParsedExif) {
        fn take<T: Clone>(slot: &mut Option<T>, extra: &Option<T>) {
            if slot.is_none() {
                *slot = extra.clone();
            }
        }
        take(&mut self.info.camera, &other.info.camera);
        take(&mut self.info.lens, &other.info.lens);
        take(&mut self.info.focal_length, &other.info.focal_length);
        take(&mut self.info.aperture, &other.info.aperture);
        take(&mut self.info.shutter, &other.info.shutter);
        take(&mut self.info.iso, &other.info.iso);
        take(&mut self.info.date, &other.info.date);
        take(&mut self.info.captured_at, &other.info.captured_at);
        take(&mut self.info.captured_subsec, &other.info.captured_subsec);
        take(&mut self.info.width, &other.info.width);
        take(&mut self.info.height, &other.info.height);
        take(&mut self.info.orientation, &other.info.orientation);
        if self.orientation_value.is_none() {
            self.orientation_value = other.orientation_value;
        }
    }

    fn empty() -> Self {
        Self {
            info: ExifInfo::default(),
            orientation_value: None,
        }
    }
}

pub fn from_path(path: &Path) -> ExifInfo {
    parse_path(path).info
}

pub fn orientation_from_bytes(bytes: &[u8]) -> Option<u16> {
    parse_bytes(bytes).orientation_value
}

pub fn parse_path(path: &Path) -> ParsedExif {
    let mut parsed = read_container_file(path);
    if is_raw_path(path) && !parsed.has_shooting_settings() {
        if let Ok(bytes) = std::fs::read(path) {
            if let Some(preview) = crate::raw_preview::extract_largest_jpeg(&bytes) {
                parsed.fill_gaps_from(&parse_bytes(&preview.bytes));
            }
        }
    }
    parsed
}

pub fn parse_bytes(bytes: &[u8]) -> ParsedExif {
    let mut cursor = Cursor::new(bytes);
    match read_container(&mut cursor) {
        Some(exif) => parsed_from_exif(&exif),
        None => ParsedExif::empty(),
    }
}

fn read_container_file(path: &Path) -> ParsedExif {
    let Ok(file) = File::open(path) else {
        return ParsedExif::empty();
    };
    let mut reader = BufReader::new(file);
    match read_container(&mut reader) {
        Some(exif) => parsed_from_exif(&exif),
        None => ParsedExif::empty(),
    }
}

fn read_container<R: std::io::BufRead + std::io::Seek>(reader: &mut R) -> Option<exif::Exif> {
    exif::Reader::new()
        .continue_on_error(true)
        .read_from_container(reader)
        .or_else(|err| err.distill_partial_result(|_| {}))
        .ok()
}

fn parsed_from_exif(exif: &exif::Exif) -> ParsedExif {
    let orientation_value = field(exif, exif::Tag::Orientation)
        .and_then(uint)
        .and_then(|value| {
            if (1..=8).contains(&value) {
                Some(value as u16)
            } else {
                None
            }
        });

    let make = field(exif, exif::Tag::Make).and_then(text);
    let model = field(exif, exif::Tag::Model).and_then(text);
    let lens = field(exif, exif::Tag::LensModel)
        .and_then(text)
        .or_else(|| field(exif, exif::Tag::LensMake).and_then(text));

    let focal_length = field(exif, exif::Tag::FocalLength)
        .and_then(rational)
        .map(|(num, denom)| format_focal_length(num, denom));
    let aperture = field(exif, exif::Tag::FNumber)
        .and_then(rational)
        .map(|(num, denom)| format_aperture(num, denom));
    let shutter = field(exif, exif::Tag::ExposureTime)
        .and_then(rational)
        .map(|(num, denom)| format_shutter(num, denom));
    let iso = field(exif, exif::Tag::PhotographicSensitivity)
        .and_then(uint)
        .filter(|iso| *iso > 0);

    let date_text = field(exif, exif::Tag::DateTimeOriginal).and_then(text);
    let subsec_text = field(exif, exif::Tag::SubSecTimeOriginal).and_then(text);
    let captured_subsec = subsec_text.as_deref().and_then(subsec_micros);
    let (captured_at, date) = match date_text.as_deref().and_then(parse_capture_date) {
        Some((unix, display)) => {
            let display = match subsec_text.as_deref().and_then(subsec_display) {
                Some(fraction) => format!("{display}.{fraction}"),
                None => display,
            };
            (Some(unix), Some(display))
        }
        None => (None, None),
    };

    let mut width = field(exif, exif::Tag::PixelXDimension)
        .and_then(uint)
        .or_else(|| field(exif, exif::Tag::ImageWidth).and_then(uint));
    let mut height = field(exif, exif::Tag::PixelYDimension)
        .and_then(uint)
        .or_else(|| field(exif, exif::Tag::ImageLength).and_then(uint));
    if matches!(orientation_value, Some(5..=8)) {
        std::mem::swap(&mut width, &mut height);
    }

    ParsedExif {
        info: ExifInfo {
            camera: camera_name(make, model),
            lens,
            focal_length,
            aperture,
            shutter,
            iso,
            date,
            captured_at,
            captured_subsec,
            width,
            height,
            orientation: orientation_value.and_then(orientation_label),
        },
        orientation_value,
    }
}

fn field(exif: &exif::Exif, tag: exif::Tag) -> Option<&exif::Field> {
    let mut fallback = None;
    for candidate in exif.fields() {
        if candidate.tag != tag {
            continue;
        }
        if candidate.ifd_num == exif::In::PRIMARY {
            return Some(candidate);
        }
        if fallback.is_none() {
            fallback = Some(candidate);
        }
    }
    fallback
}

fn text(field: &exif::Field) -> Option<String> {
    let exif::Value::Ascii(values) = &field.value else {
        return None;
    };
    let bytes = values.first()?;
    let raw = String::from_utf8_lossy(bytes);
    let trimmed = raw.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    if trimmed.is_empty() || trimmed.chars().all(|c| c == '-') {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn uint(field: &exif::Field) -> Option<u32> {
    field.value.get_uint(0)
}

fn rational(field: &exif::Field) -> Option<(u32, u32)> {
    match &field.value {
        exif::Value::Rational(values) => {
            let ratio = values.first()?;
            if ratio.denom == 0 {
                None
            } else {
                Some((ratio.num, ratio.denom))
            }
        }
        _ => None,
    }
}

fn camera_name(make: Option<String>, model: Option<String>) -> Option<String> {
    match (make, model) {
        (Some(make), Some(model)) => {
            let make_l = make.to_lowercase();
            let model_l = model.to_lowercase();
            if model_l.starts_with(make_l.trim()) {
                return Some(model);
            }
            let make_first = make_l.split_whitespace().next().unwrap_or("");
            let model_first = model_l.split_whitespace().next().unwrap_or("");
            if !make_first.is_empty() && make_first == model_first {
                Some(model)
            } else {
                Some(format!("{make} {model}"))
            }
        }
        (Some(make), None) => Some(make),
        (None, Some(model)) => Some(model),
        (None, None) => None,
    }
}

fn format_focal_length(num: u32, denom: u32) -> String {
    let mm = num as f64 / denom as f64;
    if (mm - mm.round()).abs() < 0.05 {
        format!("{} mm", mm.round() as u32)
    } else {
        format!("{mm:.1} mm")
    }
}

fn format_aperture(num: u32, denom: u32) -> String {
    let value = num as f64 / denom as f64;
    let tenths = (value * 10.0).round() / 10.0;
    if (tenths - tenths.round()).abs() < 0.05 {
        format!("f/{:.0}", tenths)
    } else {
        format!("f/{tenths:.1}")
    }
}

fn format_shutter(num: u32, denom: u32) -> String {
    if num == 0 {
        return "0 s".to_string();
    }
    if num >= denom {
        if num.is_multiple_of(denom) {
            format!("{} s", num / denom)
        } else {
            format!("{:.1} s", num as f64 / denom as f64)
        }
    } else {
        let inverse = (denom as f64 / num as f64).round().max(1.0) as u32;
        format!("1/{inverse}")
    }
}

fn parse_capture_date(value: &str) -> Option<(i64, String)> {
    let value = value.trim().trim_matches('\0');
    let (date, time) = value.split_once(' ')?;
    let mut date_parts = date.split(':');
    let year: i32 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;
    let mut time_parts = time.split(':');
    let hour: u32 = time_parts.next()?.parse().ok()?;
    let minute: u32 = time_parts.next()?.parse().ok()?;
    let second: u32 = time_parts.next()?.parse().ok()?;
    let naive =
        chrono::NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, minute, second)?;
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let display = format!(
        "{day:02} {} {year}, {hour:02}:{minute:02}:{second:02}",
        month_name.get(month as usize - 1)?
    );
    Some((naive.and_utc().timestamp(), display))
}

/// EXIF SubSecTime is a fractional-second digit string: "12" is 0.12s.
fn subsec_micros(value: &str) -> Option<u32> {
    let digits: String = value
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(6)
        .collect();
    if digits.is_empty() {
        return None;
    }
    let number: u32 = digits.parse().ok()?;
    let scale = 10u32.pow(6 - digits.len() as u32);
    Some(number.saturating_mul(scale))
}

fn subsec_display(value: &str) -> Option<String> {
    let digits: String = value
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(3)
        .collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn orientation_label(value: u16) -> Option<String> {
    let label = match value {
        2 => "Flipped horizontal",
        3 => "Rotated 180°",
        4 => "Flipped vertical",
        5 => "Rotated 90° and flipped",
        6 => "Rotated 90°",
        7 => "Rotated 270° and flipped",
        8 => "Rotated 270°",
        _ => return None,
    };
    Some(label.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::jpeg_with_shooting_exif;

    #[test]
    fn reads_shooting_settings_from_a_generated_jpeg() {
        let jpeg = jpeg_with_shooting_exif(4, 2, 6);
        let parsed = parse_bytes(&jpeg);
        let info = parsed.info;
        assert_eq!(info.camera.as_deref(), Some("Canon EOS R5"));
        assert_eq!(info.lens.as_deref(), Some("RF85mm F1.2 L USM"));
        assert_eq!(info.focal_length.as_deref(), Some("85 mm"));
        assert_eq!(info.aperture.as_deref(), Some("f/1.2"));
        assert_eq!(info.shutter.as_deref(), Some("1/200"));
        assert_eq!(info.iso, Some(400));
        assert_eq!(info.date.as_deref(), Some("15 Jan 2024, 14:30:45.12"));
        assert_eq!(info.captured_subsec, Some(120_000));
        assert!(info.captured_at.is_some());
        // Orientation 6 swaps the stored 4×2 sensor size.
        assert_eq!(info.width, Some(2));
        assert_eq!(info.height, Some(4));
        assert_eq!(info.orientation.as_deref(), Some("Rotated 90°"));
        assert_eq!(parsed.orientation_value, Some(6));
    }

    #[test]
    fn subsec_digits_are_a_fraction_of_a_second() {
        assert_eq!(subsec_micros("1"), Some(100_000));
        assert_eq!(subsec_micros("12"), Some(120_000));
        assert_eq!(subsec_micros("123456"), Some(123_456));
        assert_eq!(subsec_micros(""), None);
    }

    #[test]
    fn shutter_and_aperture_use_photographer_notation() {
        assert_eq!(format_shutter(1, 200), "1/200");
        assert_eq!(format_shutter(5, 2), "2.5 s");
        assert_eq!(format_shutter(10, 1), "10 s");
        assert_eq!(format_aperture(12, 10), "f/1.2");
        assert_eq!(format_aperture(4, 1), "f/4");
    }

    #[test]
    fn camera_name_does_not_repeat_the_make() {
        assert_eq!(
            camera_name(Some("Canon".into()), Some("Canon EOS R5".into())).as_deref(),
            Some("Canon EOS R5")
        );
        assert_eq!(
            camera_name(Some("NIKON CORPORATION".into()), Some("NIKON Z 8".into())).as_deref(),
            Some("NIKON Z 8")
        );
        assert_eq!(
            camera_name(Some("Canon".into()), Some("EOS R5".into())).as_deref(),
            Some("Canon EOS R5")
        );
    }

    #[test]
    fn upright_orientation_is_not_a_panel_field() {
        assert!(orientation_label(1).is_none());
        assert_eq!(orientation_label(8).as_deref(), Some("Rotated 270°"));
    }
}
