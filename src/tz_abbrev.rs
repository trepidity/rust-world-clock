use chrono_tz::Tz;
use std::collections::HashMap;

/// Returns a map of common timezone abbreviations to their IANA timezone(s).
/// Abbreviations are stored uppercase. Some abbreviations map to multiple zones
/// (e.g., IST -> India, Ireland, Israel).
pub fn abbreviation_map() -> HashMap<&'static str, Vec<Tz>> {
    let entries: &[(&str, &[&str])] = &[
        // North America
        ("EST", &["America/New_York"]),
        ("EDT", &["America/New_York"]),
        ("CST", &["America/Chicago"]),
        ("CDT", &["America/Chicago"]),
        ("MST", &["America/Denver"]),
        ("MDT", &["America/Denver"]),
        ("PST", &["America/Los_Angeles"]),
        ("PDT", &["America/Los_Angeles"]),
        ("AKST", &["America/Anchorage"]),
        ("AKDT", &["America/Anchorage"]),
        ("HST", &["Pacific/Honolulu"]),
        ("AST", &["America/Halifax", "America/Puerto_Rico"]),
        ("NST", &["America/St_Johns"]),
        ("NDT", &["America/St_Johns"]),
        // Europe
        ("GMT", &["Europe/London"]),
        ("UTC", &["UTC"]),
        ("BST", &["Europe/London"]),
        ("CET", &["Europe/Paris"]),
        ("CEST", &["Europe/Paris"]),
        ("EET", &["Europe/Bucharest"]),
        ("EEST", &["Europe/Bucharest"]),
        ("WET", &["Europe/Lisbon"]),
        ("WEST", &["Europe/Lisbon"]),
        ("MSK", &["Europe/Moscow"]),
        // Asia
        ("IST", &["Asia/Kolkata", "Europe/Dublin", "Asia/Jerusalem"]),
        ("PKT", &["Asia/Karachi"]),
        ("NPT", &["Asia/Kathmandu"]),
        ("ICT", &["Asia/Bangkok"]),
        ("WIB", &["Asia/Jakarta"]),
        ("SGT", &["Asia/Singapore"]),
        ("HKT", &["Asia/Hong_Kong"]),
        ("JST", &["Asia/Tokyo"]),
        ("KST", &["Asia/Seoul"]),
        ("PHT", &["Asia/Manila"]),
        ("AFT", &["Asia/Kabul"]),
        ("IRST", &["Asia/Tehran"]),
        ("GST", &["Asia/Dubai"]),
        // Australia / NZ
        ("AEST", &["Australia/Sydney"]),
        ("AEDT", &["Australia/Sydney"]),
        ("ACST", &["Australia/Adelaide"]),
        ("ACDT", &["Australia/Adelaide"]),
        ("AWST", &["Australia/Perth"]),
        ("NZST", &["Pacific/Auckland"]),
        ("NZDT", &["Pacific/Auckland"]),
        // Africa
        ("WAT", &["Africa/Lagos"]),
        ("CAT", &["Africa/Harare"]),
        ("EAT", &["Africa/Nairobi"]),
        ("SAST", &["Africa/Johannesburg"]),
        // South America
        ("ART", &["America/Argentina/Buenos_Aires"]),
        ("BRT", &["America/Sao_Paulo"]),
        ("BRST", &["America/Sao_Paulo"]),
        ("CLT", &["America/Santiago"]),
        ("COT", &["America/Bogota"]),
        ("PET", &["America/Lima"]),
        ("VET", &["America/Caracas"]),
    ];

    let mut map: HashMap<&'static str, Vec<Tz>> = HashMap::new();
    for &(abbrev, iana_names) in entries {
        let tzs: Vec<Tz> = iana_names
            .iter()
            .filter_map(|name| name.parse::<Tz>().ok())
            .collect();
        if !tzs.is_empty() {
            map.insert(abbrev, tzs);
        }
    }
    map
}
