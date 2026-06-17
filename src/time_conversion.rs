use crate::tz_abbrev::abbreviation_map;
use chrono::{DateTime, LocalResult, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct TimeConversion {
    pub source_label: String,
    pub target_label: String,
    pub source_time: DateTime<Tz>,
    pub target_time: DateTime<Tz>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    MissingTime,
    MissingSourceZone,
    MissingTargetZone,
    AmbiguousLocalTime(String),
    NonexistentLocalTime(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::MissingTime => {
                write!(f, "Enter a time like 7AM, 7:30 PM, or 19:00.")
            }
            ConversionError::MissingSourceZone => {
                write!(f, "Add the source timezone after the time, like 7AM CT.")
            }
            ConversionError::MissingTargetZone => {
                write!(
                    f,
                    "Add the target timezone, like to IST or in Asia/Kolkata."
                )
            }
            ConversionError::AmbiguousLocalTime(zone) => {
                write!(
                    f,
                    "That local time is ambiguous in {zone}. Add a clearer date/time."
                )
            }
            ConversionError::NonexistentLocalTime(zone) => {
                write!(
                    f,
                    "That local time does not exist in {zone} because of a timezone transition."
                )
            }
        }
    }
}

pub fn convert_query(query: &str, date: NaiveDate) -> Result<TimeConversion, ConversionError> {
    let tokens = tokenize(query);
    let parsed_time = find_time(&tokens).ok_or(ConversionError::MissingTime)?;
    let source = find_source_zone(&tokens, parsed_time.next_index)
        .ok_or(ConversionError::MissingSourceZone)?;
    let target =
        find_target_zone(&tokens, source.index).ok_or(ConversionError::MissingTargetZone)?;

    let local_time = match source
        .timezone
        .from_local_datetime(&date.and_time(parsed_time.time))
    {
        LocalResult::Single(time) => time,
        LocalResult::Ambiguous(_, _) => {
            return Err(ConversionError::AmbiguousLocalTime(
                source.timezone.name().to_string(),
            ));
        }
        LocalResult::None => {
            return Err(ConversionError::NonexistentLocalTime(
                source.timezone.name().to_string(),
            ));
        }
    };

    Ok(TimeConversion {
        source_label: source.label,
        target_label: target.label,
        source_time: local_time,
        target_time: local_time.with_timezone(&target.timezone),
    })
}

pub fn format_time(time: DateTime<Tz>) -> String {
    time.format("%-I:%M %p").to_string()
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | ':' | '+' | '-') {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone)]
struct ParsedTime {
    time: NaiveTime,
    next_index: usize,
}

fn find_time(tokens: &[String]) -> Option<ParsedTime> {
    for index in 0..tokens.len() {
        if let Some(parsed) = parse_time_at(tokens, index) {
            return Some(parsed);
        }
    }

    None
}

fn parse_time_at(tokens: &[String], index: usize) -> Option<ParsedTime> {
    let token = tokens.get(index)?;
    let (base, suffix) = split_meridiem(token);
    let mut next_index = index + 1;
    let meridiem = if suffix.is_some() {
        suffix
    } else if let Some(next) = tokens
        .get(next_index)
        .and_then(|token| parse_meridiem(token))
    {
        next_index += 1;
        Some(next)
    } else {
        None
    };

    let (hour, minute) = parse_hour_minute(base)?;
    let hour = if let Some(meridiem) = meridiem {
        match meridiem {
            Meridiem::Am if hour == 12 => 0,
            Meridiem::Am => hour,
            Meridiem::Pm if hour == 12 => 12,
            Meridiem::Pm => hour + 12,
        }
    } else {
        hour
    };

    NaiveTime::from_hms_opt(hour, minute, 0).map(|time| ParsedTime { time, next_index })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Meridiem {
    Am,
    Pm,
}

fn split_meridiem(token: &str) -> (&str, Option<Meridiem>) {
    let lowercase = token.to_ascii_lowercase();

    if lowercase.ends_with("am") {
        (&token[..token.len() - 2], Some(Meridiem::Am))
    } else if lowercase.ends_with("pm") {
        (&token[..token.len() - 2], Some(Meridiem::Pm))
    } else {
        (token, None)
    }
}

fn parse_meridiem(token: &str) -> Option<Meridiem> {
    match token.to_ascii_lowercase().as_str() {
        "am" => Some(Meridiem::Am),
        "pm" => Some(Meridiem::Pm),
        _ => None,
    }
}

fn parse_hour_minute(value: &str) -> Option<(u32, u32)> {
    if value.is_empty() {
        return None;
    }

    if let Some((hour, minute)) = value.split_once(':') {
        return Some((hour.parse().ok()?, minute.parse().ok()?));
    }

    Some((value.parse().ok()?, 0))
}

#[derive(Debug, Clone)]
struct ZoneMatch {
    index: usize,
    label: String,
    timezone: Tz,
}

fn find_source_zone(tokens: &[String], start_index: usize) -> Option<ZoneMatch> {
    (start_index..tokens.len()).find_map(|index| resolve_zone_at(tokens, index))
}

fn find_target_zone(tokens: &[String], source_index: usize) -> Option<ZoneMatch> {
    for index in source_index + 1..tokens.len().saturating_sub(1) {
        if is_target_marker(&tokens[index])
            && let Some(zone_match) = resolve_zone_at(tokens, index + 1)
        {
            return Some(zone_match);
        }
    }

    (source_index + 1..tokens.len())
        .filter_map(|index| resolve_zone_at(tokens, index))
        .next_back()
}

fn is_target_marker(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "to" | "in" | "into" | "for" | "as"
    )
}

fn resolve_zone_at(tokens: &[String], index: usize) -> Option<ZoneMatch> {
    let token = tokens.get(index)?;
    let upper = token.to_ascii_uppercase();

    if let Ok(timezone) = token.parse::<Tz>() {
        return Some(ZoneMatch {
            index,
            label: token.clone(),
            timezone,
        });
    }

    if let Some(timezone) = common_zone(&upper) {
        return Some(ZoneMatch {
            index,
            label: upper,
            timezone,
        });
    }

    abbreviation_map()
        .get(upper.as_str())
        .and_then(|zones| zones.first().copied())
        .map(|timezone| ZoneMatch {
            index,
            label: upper,
            timezone,
        })
}

fn common_zone(upper: &str) -> Option<Tz> {
    match upper {
        "CT" | "CENTRAL" => Some(chrono_tz::America::Chicago),
        "ET" | "EASTERN" => Some(chrono_tz::America::New_York),
        "MT" | "MOUNTAIN" => Some(chrono_tz::America::Denver),
        "PT" | "PACIFIC" => Some(chrono_tz::America::Los_Angeles),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_user_story_from_central_to_india() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 17).unwrap();
        let conversion =
            convert_query("I can meet at 7AM CT, what time is that in IST", date).unwrap();

        assert_eq!(
            conversion.source_time.time(),
            NaiveTime::from_hms_opt(7, 0, 0).unwrap()
        );
        assert_eq!(conversion.source_time.timezone().name(), "America/Chicago");
        assert_eq!(
            conversion.target_time.time(),
            NaiveTime::from_hms_opt(17, 30, 0).unwrap()
        );
        assert_eq!(conversion.target_time.timezone().name(), "Asia/Kolkata");
    }

    #[test]
    fn accepts_iana_timezones() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let conversion = convert_query("19:00 America/Chicago to Europe/London", date).unwrap();

        assert_eq!(format_time(conversion.target_time), "1:00 AM");
    }
}
