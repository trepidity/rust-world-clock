use crate::tz_abbrev::abbreviation_map;
use chrono_tz::Tz;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimezoneSearchMatch {
    pub display: String,
    pub zone_name: String,
}

#[derive(Clone, Debug)]
struct ScoredMatch {
    score: usize,
    order: usize,
    item: TimezoneSearchMatch,
}

const HUMAN_ZONE_ALIASES: &[(&str, &str)] = &[
    ("CT", "America/Chicago"),
    ("Central Time", "America/Chicago"),
    ("Central Standard Time", "America/Chicago"),
    ("Central Daylight Time", "America/Chicago"),
    ("ET", "America/New_York"),
    ("Eastern Time", "America/New_York"),
    ("Eastern Standard Time", "America/New_York"),
    ("Eastern Daylight Time", "America/New_York"),
    ("MT", "America/Denver"),
    ("Mountain Time", "America/Denver"),
    ("Mountain Standard Time", "America/Denver"),
    ("Mountain Daylight Time", "America/Denver"),
    ("PT", "America/Los_Angeles"),
    ("Pacific Time", "America/Los_Angeles"),
    ("Pacific Standard Time", "America/Los_Angeles"),
    ("Pacific Daylight Time", "America/Los_Angeles"),
    ("India Standard Time", "Asia/Kolkata"),
    ("Indian Standard Time", "Asia/Kolkata"),
    ("Turkey", "Europe/Istanbul"),
    ("Turkiye", "Europe/Istanbul"),
    ("Turkey Standard Time", "Europe/Istanbul"),
    ("Turkiye Standard Time", "Europe/Istanbul"),
    ("Istanbul", "Europe/Istanbul"),
    ("Greenwich Mean Time", "UTC"),
    ("Coordinated Universal Time", "UTC"),
];

pub fn all_timezones() -> Vec<TimezoneSearchMatch> {
    chrono_tz::TZ_VARIANTS
        .iter()
        .map(|timezone| {
            let name = timezone.name().to_string();
            TimezoneSearchMatch {
                display: name.clone(),
                zone_name: name,
            }
        })
        .collect()
}

pub fn search_timezones(query: &str, limit: usize) -> Vec<TimezoneSearchMatch> {
    let normalized_query = normalize_search_text(query);
    if normalized_query.is_empty() {
        return truncate_matches(all_timezones(), limit);
    }

    let mut matches: HashMap<String, ScoredMatch> = HashMap::new();
    let mut order = 0;

    for &(alias, zone_name) in HUMAN_ZONE_ALIASES {
        add_candidate(
            &mut matches,
            &normalized_query,
            alias,
            zone_name,
            format!("{zone_name} ({alias})"),
            0,
            order,
        );
        order += 1;
    }

    for (abbreviation, timezones) in abbreviation_map() {
        for timezone in timezones {
            let zone_name = timezone.name();
            add_candidate(
                &mut matches,
                &normalized_query,
                &format!("{abbreviation} {zone_name}"),
                zone_name,
                format!("{zone_name} ({abbreviation})"),
                8,
                order,
            );
            order += 1;
        }
    }

    for timezone in chrono_tz::TZ_VARIANTS {
        let zone_name = timezone.name();
        add_candidate(
            &mut matches,
            &normalized_query,
            zone_name,
            zone_name,
            zone_name.to_string(),
            24,
            order,
        );
        order += 1;
    }

    let mut scored: Vec<ScoredMatch> = matches.into_values().collect();
    scored.sort_by(|left, right| {
        left.score
            .cmp(&right.score)
            .then_with(|| left.order.cmp(&right.order))
            .then_with(|| left.item.zone_name.cmp(&right.item.zone_name))
    });

    let results = scored.into_iter().map(|scored| scored.item).collect();
    truncate_matches(results, limit)
}

pub fn resolve_timezone(query: &str) -> Option<Tz> {
    search_timezones(query, 1)
        .into_iter()
        .next()
        .and_then(|matched| matched.zone_name.parse::<Tz>().ok())
}

fn add_candidate(
    matches: &mut HashMap<String, ScoredMatch>,
    normalized_query: &str,
    searchable: &str,
    zone_name: &str,
    display: String,
    priority: usize,
    order: usize,
) {
    let Ok(timezone) = zone_name.parse::<Tz>() else {
        return;
    };
    let canonical_zone = timezone.name().to_string();
    let Some(score) = score_match(
        normalized_query,
        &normalize_search_text(searchable),
        priority,
    ) else {
        return;
    };

    let item = TimezoneSearchMatch {
        display,
        zone_name: canonical_zone.clone(),
    };

    match matches.get(&canonical_zone) {
        Some(existing)
            if existing.score < score || (existing.score == score && existing.order <= order) => {}
        _ => {
            matches.insert(canonical_zone, ScoredMatch { score, order, item });
        }
    }
}

fn score_match(query: &str, candidate: &str, priority: usize) -> Option<usize> {
    if query == candidate {
        return Some(priority);
    }

    if candidate.contains(query) {
        let starts_penalty = if candidate.starts_with(query) { 4 } else { 12 };
        return Some(priority + starts_penalty + candidate.len().saturating_sub(query.len()));
    }

    if query.contains(candidate) && candidate.split_whitespace().count() > 1 {
        return Some(priority + 16 + query.len().saturating_sub(candidate.len()));
    }

    if let Some(token_score) = score_tokens(query, candidate) {
        return Some(priority + 40 + token_score);
    }

    let query_compact = compact(query);
    let candidate_compact = compact(candidate);
    if query_compact.len() >= 3
        && let Some(fuzzy_score) = fuzzy_subsequence_score(&query_compact, &candidate_compact)
    {
        return Some(priority + 120 + fuzzy_score);
    }

    None
}

fn score_tokens(query: &str, candidate: &str) -> Option<usize> {
    let query_tokens: Vec<&str> = query.split_whitespace().collect();
    let candidate_tokens: Vec<&str> = candidate.split_whitespace().collect();
    if query_tokens.is_empty() || candidate_tokens.is_empty() {
        return None;
    }

    let mut total = 0;
    for query_token in query_tokens {
        let best = candidate_tokens
            .iter()
            .filter_map(|candidate_token| score_token(query_token, candidate_token))
            .min()?;
        total += best;
    }

    Some(total + candidate.len().saturating_sub(query.len()))
}

fn score_token(query: &str, candidate: &str) -> Option<usize> {
    if query == candidate {
        return Some(0);
    }

    if candidate.starts_with(query) {
        return Some(3 + candidate.len().saturating_sub(query.len()));
    }

    if candidate.contains(query) {
        return Some(8 + candidate.len().saturating_sub(query.len()));
    }

    if query.len() >= 4 && edit_distance_at_most(query, candidate, 1) {
        return Some(16);
    }

    if query.len() >= 4 && fuzzy_subsequence_score(query, candidate).is_some() {
        return Some(28 + candidate.len().saturating_sub(query.len()));
    }

    None
}

fn fuzzy_subsequence_score(query: &str, candidate: &str) -> Option<usize> {
    let mut query_chars = query.chars();
    let mut current = query_chars.next()?;
    let mut matched = 0;
    let mut gaps = 0;
    let mut last_match_index: Option<usize> = None;

    for (index, candidate_char) in candidate.chars().enumerate() {
        if candidate_char == current {
            if let Some(last_index) = last_match_index {
                gaps += index.saturating_sub(last_index + 1);
            } else {
                gaps += index;
            }
            matched += 1;
            last_match_index = Some(index);

            match query_chars.next() {
                Some(next) => current = next,
                None => return Some(gaps + candidate.len().saturating_sub(matched)),
            }
        }
    }

    None
}

fn edit_distance_at_most(left: &str, right: &str, max_distance: usize) -> bool {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    if left_chars.len().abs_diff(right_chars.len()) > max_distance {
        return false;
    }

    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left_chars.iter().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];

        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
            row_min = row_min.min(current[right_index + 1]);
        }

        if row_min > max_distance {
            return false;
        }

        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()] <= max_distance
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|character| *character != ' ')
        .collect()
}

fn normalize_search_text(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_space = true;

    for character in value.chars() {
        let mapped = normalize_character(character);
        for mapped_character in mapped.chars() {
            if mapped_character == ' ' {
                if !previous_was_space {
                    normalized.push(' ');
                    previous_was_space = true;
                }
            } else {
                normalized.push(mapped_character);
                previous_was_space = false;
            }
        }
    }

    normalized.trim().to_string()
}

fn normalize_character(character: char) -> String {
    match character {
        'A'..='Z' => character.to_ascii_lowercase().to_string(),
        'a'..='z' | '0'..='9' => character.to_string(),
        '\u{00c7}' | '\u{00e7}' => "c".to_string(),
        '\u{011e}' | '\u{011f}' => "g".to_string(),
        '\u{0130}' | '\u{0131}' => "i".to_string(),
        '\u{00d6}' | '\u{00f6}' => "o".to_string(),
        '\u{015e}' | '\u{015f}' => "s".to_string(),
        '\u{00dc}' | '\u{00fc}' => "u".to_string(),
        _ if character.is_ascii_alphanumeric() => character.to_ascii_lowercase().to_string(),
        _ => " ".to_string(),
    }
}

fn truncate_matches(
    mut matches: Vec<TimezoneSearchMatch>,
    limit: usize,
) -> Vec<TimezoneSearchMatch> {
    if limit > 0 {
        matches.truncate(limit);
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_pasted_turkiye_display_name() {
        let timezone =
            resolve_timezone("T\u{00fc}rkiye Standard Time\nTime zone in T\u{00fc}rkiye (GMT+3)")
                .expect("display label should resolve");

        assert_eq!(timezone.name(), "Europe/Istanbul");
    }

    #[test]
    fn resolves_windows_turkey_display_name() {
        let timezone =
            resolve_timezone("Turkey Standard Time").expect("windows label should resolve");

        assert_eq!(timezone.name(), "Europe/Istanbul");
    }

    #[test]
    fn fuzzy_matches_city_typo() {
        let timezone = resolve_timezone("istnbul").expect("city typo should resolve");

        assert_eq!(timezone.name(), "Europe/Istanbul");
    }

    #[test]
    fn fuzzy_matches_spaced_city_name() {
        let timezone = resolve_timezone("new york").expect("spaced city should resolve");

        assert_eq!(timezone.name(), "America/New_York");
    }
}
