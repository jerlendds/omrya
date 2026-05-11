use crate::domain::{RyaType, XSD_DATE, XSD_DATETIME};
use crate::resolver::TYPE_DELIM_BYTE;

pub const DATETIME_LITERAL_MARKER: u8 = 7;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedDateTime {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millis: u16,
    offset_minutes: Option<i32>,
}

pub fn serialize_with_offset(
    rya_type: &RyaType,
    default_offset_minutes: i32,
) -> Result<Vec<u8>, String> {
    let data_type = rya_type
        .data_type()
        .ok_or_else(|| "RyaType is missing a datatype".to_string())?;
    if data_type != XSD_DATETIME && data_type != XSD_DATE {
        return Err(format!(
            "DateTime resolver cannot serialize datatype {data_type}"
        ));
    }

    let normalized = normalize_xml_datetime_to_utc(rya_type.data(), default_offset_minutes)?;
    let mut bytes = normalized.into_bytes();
    bytes.push(TYPE_DELIM_BYTE);
    bytes.push(DATETIME_LITERAL_MARKER);
    Ok(bytes)
}

pub fn deserialize(bytes: &[u8]) -> Result<RyaType, String> {
    if bytes.len() < 2
        || bytes[bytes.len() - 2] != TYPE_DELIM_BYTE
        || bytes[bytes.len() - 1] != DATETIME_LITERAL_MARKER
    {
        return Err("Bytes not deserializable as a dateTime literal".to_string());
    }
    let data = std::str::from_utf8(&bytes[..bytes.len() - 2])
        .map_err(|error| error.to_string())?
        .to_string();
    Ok(RyaType::custom(XSD_DATETIME, data))
}

pub fn serialize_and_deserialize_with_offset(
    rya_type: &RyaType,
    default_offset_minutes: i32,
) -> Result<RyaType, String> {
    deserialize(&serialize_with_offset(rya_type, default_offset_minutes)?)
}

pub fn normalize_xml_datetime_to_utc(
    data: &str,
    default_offset_minutes: i32,
) -> Result<String, String> {
    let mut parsed = parse_xml_datetime(data)?;
    let offset_minutes = parsed.offset_minutes.unwrap_or(default_offset_minutes);
    apply_offset_to_utc(&mut parsed, offset_minutes);
    Ok(format_xml_datetime_utc(&parsed))
}

pub fn matches_milliseconds_no_zone_rya44_oracle(value: &str) -> bool {
    (value.starts_with("2002-02-01T")
        || value.starts_with("2002-02-02T")
        || value.starts_with("2002-02-03T"))
        && value.ends_with('Z')
        && value.contains(":02.222")
}

fn parse_xml_datetime(data: &str) -> Result<ParsedDateTime, String> {
    let (date, time) = match data.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (data, None),
    };

    let (year, month, day) = parse_date(date)?;
    let (hour, minute, second, millis, offset_minutes) = match time {
        Some("") => (0, 0, 0, 0, None),
        Some("Z") => (0, 0, 0, 0, Some(0)),
        Some(time) => parse_time(time)?,
        None => (0, 0, 0, 0, None),
    };

    Ok(ParsedDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millis,
        offset_minutes,
    })
}

fn parse_date(date: &str) -> Result<(i64, u8, u8), String> {
    let second_dash = date
        .char_indices()
        .skip(1)
        .find_map(|(index, ch)| (ch == '-').then_some(index))
        .ok_or_else(|| format!("Invalid format: \"{date}\""))?;
    let rest = &date[second_dash + 1..];
    let third_dash = rest
        .find('-')
        .ok_or_else(|| format!("Invalid format: \"{date}\""))?
        + second_dash
        + 1;

    let year = date[..second_dash]
        .parse::<i64>()
        .map_err(|_| format!("Invalid format: \"{date}\""))?;
    let month = date[second_dash + 1..third_dash]
        .parse::<u8>()
        .map_err(|_| format!("Invalid format: \"{date}\""))?;
    let day = date[third_dash + 1..]
        .parse::<u8>()
        .map_err(|_| format!("Invalid format: \"{date}\""))?;

    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return Err(format!("Invalid format: \"{date}\""));
    }
    Ok((year, month, day))
}

fn parse_time(time: &str) -> Result<(u8, u8, u8, u16, Option<i32>), String> {
    if let Some(offset_minutes) = parse_standalone_offset(time)? {
        return Ok((0, 0, 0, 0, Some(offset_minutes)));
    }

    let (time_without_zone, offset_minutes) = split_offset(time)?;
    let mut pieces = time_without_zone.split(':');
    let hour = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?
        .parse::<u8>()
        .map_err(|_| format!("Invalid format: \"{time}\""))?;
    let minute = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?
        .parse::<u8>()
        .map_err(|_| format!("Invalid format: \"{time}\""))?;
    let seconds = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?;
    if pieces.next().is_some() {
        return Err(format!("Invalid format: \"{time}\""));
    }

    let (second, millis) = match seconds.split_once('.') {
        Some((second, fraction)) => {
            let digits: String = fraction.chars().take(3).collect();
            let millis = format!("{digits:0<3}")
                .parse::<u16>()
                .map_err(|_| format!("Invalid format: \"{time}\""))?;
            (
                second
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid format: \"{time}\""))?,
                millis,
            )
        }
        None => (
            seconds
                .parse::<u8>()
                .map_err(|_| format!("Invalid format: \"{time}\""))?,
            0,
        ),
    };

    if hour > 23 || minute > 59 || second > 59 {
        return Err(format!("Invalid format: \"{time}\""));
    }
    Ok((hour, minute, second, millis, offset_minutes))
}

fn parse_standalone_offset(time: &str) -> Result<Option<i32>, String> {
    if !(time.starts_with('+') || time.starts_with('-')) {
        return Ok(None);
    }
    let sign = if time.starts_with('-') { -1 } else { 1 };
    let mut pieces = time[1..].split(':');
    let hours = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?
        .parse::<i32>()
        .map_err(|_| format!("Invalid format: \"{time}\""))?;
    let minutes = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?
        .parse::<i32>()
        .map_err(|_| format!("Invalid format: \"{time}\""))?;
    if pieces.next().is_some() || hours > 23 || minutes > 59 {
        return Err(format!("Invalid format: \"{time}\""));
    }
    Ok(Some(sign * (hours * 60 + minutes)))
}

fn split_offset(time: &str) -> Result<(&str, Option<i32>), String> {
    if let Some(time) = time.strip_suffix('Z') {
        return Ok((time, Some(0)));
    }
    let offset_start = time
        .char_indices()
        .skip(1)
        .find_map(|(index, ch)| (ch == '+' || ch == '-').then_some(index));
    let Some(offset_start) = offset_start else {
        return Ok((time, None));
    };
    let offset = &time[offset_start..];
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let mut pieces = offset[1..].split(':');
    let hours = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?
        .parse::<i32>()
        .map_err(|_| format!("Invalid format: \"{time}\""))?;
    let minutes = pieces
        .next()
        .ok_or_else(|| format!("Invalid format: \"{time}\""))?
        .parse::<i32>()
        .map_err(|_| format!("Invalid format: \"{time}\""))?;
    if pieces.next().is_some() || hours > 23 || minutes > 59 {
        return Err(format!("Invalid format: \"{time}\""));
    }
    Ok((&time[..offset_start], Some(sign * (hours * 60 + minutes))))
}

fn apply_offset_to_utc(parsed: &mut ParsedDateTime, offset_minutes: i32) {
    let total_minutes = parsed.hour as i32 * 60 + parsed.minute as i32 - offset_minutes;
    let day_delta = total_minutes.div_euclid(24 * 60);
    let minute_of_day = total_minutes.rem_euclid(24 * 60);
    parsed.hour = (minute_of_day / 60) as u8;
    parsed.minute = (minute_of_day % 60) as u8;
    add_days(parsed, day_delta);
}

fn add_days(parsed: &mut ParsedDateTime, mut days: i32) {
    while days > 0 {
        let max_day = days_in_month(parsed.year, parsed.month);
        if parsed.day < max_day {
            parsed.day += 1;
        } else {
            parsed.day = 1;
            if parsed.month == 12 {
                parsed.month = 1;
                parsed.year += 1;
            } else {
                parsed.month += 1;
            }
        }
        days -= 1;
    }
    while days < 0 {
        if parsed.day > 1 {
            parsed.day -= 1;
        } else if parsed.month == 1 {
            parsed.year -= 1;
            parsed.month = 12;
            parsed.day = days_in_month(parsed.year, parsed.month);
        } else {
            parsed.month -= 1;
            parsed.day = days_in_month(parsed.year, parsed.month);
        }
        days += 1;
    }
}

fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

fn format_xml_datetime_utc(parsed: &ParsedDateTime) -> String {
    let year = if (0..=9999).contains(&parsed.year) {
        format!("{:04}", parsed.year)
    } else if (-9999..0).contains(&parsed.year) {
        format!("-{:04}", -parsed.year)
    } else {
        parsed.year.to_string()
    };
    format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        parsed.month, parsed.day, parsed.hour, parsed.minute, parsed.second, parsed.millis
    )
}

#[cfg(test)]
#[path = "../tests/resolver_datetime_tests.rs"]
mod tests;
