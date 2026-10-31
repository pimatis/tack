use crate::db::Result;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

// the rule strings shared with the app: daily, weekly:<iso weekday 1-7> or
// monthly:<day 1-31>; three letter weekday names are accepted as a convenience
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    Daily,
    Weekly(u32),
    Monthly(u32),
}

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

fn parse(rule: &str) -> Option<Rule> {
    let value = rule.trim().to_ascii_lowercase();
    if value == "daily" {
        return Some(Rule::Daily);
    }
    if let Some(day) = value.strip_prefix("weekly:") {
        let weekday = weekday_number(day)?;
        return Some(Rule::Weekly(weekday));
    }
    if let Some(day) = value.strip_prefix("monthly:") {
        let day: u32 = day.parse().ok()?;
        if (1..=31).contains(&day) {
            return Some(Rule::Monthly(day));
        }
    }
    None
}

fn weekday_number(value: &str) -> Option<u32> {
    let named = [
        ("mon", 1),
        ("tue", 2),
        ("wed", 3),
        ("thu", 4),
        ("fri", 5),
        ("sat", 6),
        ("sun", 7),
    ];
    if let Some((_, number)) = named.iter().find(|(name, _)| value.starts_with(name)) {
        return Some(*number);
    }
    let number: u32 = value.parse().ok()?;
    (1..=7).contains(&number).then_some(number)
}

// validate and canonicalize a user supplied rule; an empty value means the
// task does not repeat and is passed through so callers can clear the column
pub fn normalize(rule: &str) -> Result<String> {
    let value = rule.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    match parse(value) {
        Some(Rule::Daily) => Ok("daily".to_string()),
        Some(Rule::Weekly(day)) => Ok(format!("weekly:{}", day)),
        Some(Rule::Monthly(day)) => Ok(format!("monthly:{}", day)),
        None => Err(format!(
            "Invalid repeat rule '{}': use daily, weekly:<mon..sun|1-7> or monthly:<1-31>",
            rule
        )),
    }
}

pub fn label(rule: &str) -> Option<String> {
    match parse(rule)? {
        Rule::Daily => Some("Every day".to_string()),
        Rule::Weekly(day) => Some(format!(
            "Every {}",
            WEEKDAYS.get(day as usize - 1).copied().unwrap_or("week")
        )),
        Rule::Monthly(day) => Some(format!("Every month on the {}", ordinal(day))),
    }
}

fn ordinal(n: u32) -> String {
    let suffix = if (11..=13).contains(&(n % 100)) {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{}{}", n, suffix)
}

pub fn parse_date(value: &str) -> Option<NaiveDate> {
    let value = value.trim();
    let end = value.find('T').unwrap_or(value.len());
    NaiveDate::parse_from_str(&value[..end], "%Y-%m-%d").ok()
}

// the date the next occurrence lands on; a stale occurrence counts from today
// so completing an old task never spawns a date in the past
pub fn next_due(rule: &str, due: Option<&str>, today: NaiveDate) -> Option<(NaiveDate, i64)> {
    let parsed = parse(rule)?;
    let due_date = due.and_then(parse_date);
    let base = match due_date {
        Some(date) if date > today => date,
        _ => today,
    };

    let next = match parsed {
        Rule::Daily => base + Duration::days(1),
        Rule::Weekly(weekday) => {
            let mut date = base + Duration::days(1);
            while date.weekday().number_from_monday() != weekday {
                date += Duration::days(1);
            }
            date
        }
        Rule::Monthly(day) => {
            let (year, month) = if base.month() == 12 {
                (base.year() + 1, 1)
            } else {
                (base.year(), base.month() + 1)
            };
            let last = last_day(year, month);
            NaiveDate::from_ymd_opt(year, month, day.min(last))?
        }
    };

    let previous = due_date.unwrap_or(today);
    Some((next, (next - previous).num_days()))
}

fn last_day(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(28)
}

// reminders are utc instants: shift the calendar day, keep the clock time
pub fn shift_reminder(reminder: Option<&str>, days: i64) -> Option<String> {
    let value = reminder?;
    let parsed = DateTime::parse_from_rfc3339(value).ok()?;
    Some(
        (parsed.with_timezone(&Utc) + Duration::days(days))
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        parse_date(value).unwrap()
    }

    #[test]
    fn normalizes_rules() {
        assert_eq!(normalize("daily").unwrap(), "daily");
        assert_eq!(normalize("weekly:mon").unwrap(), "weekly:1");
        assert_eq!(normalize("weekly:7").unwrap(), "weekly:7");
        assert_eq!(normalize("monthly:15").unwrap(), "monthly:15");
        assert_eq!(normalize("  ").unwrap(), "");
        assert!(normalize("weekly").is_err());
        assert!(normalize("monthly:0").is_err());
    }

    #[test]
    fn advances_on_schedule() {
        let today = date("2026-10-24");
        assert_eq!(
            next_due("daily", Some("2026-10-20"), today).unwrap().0,
            date("2026-10-25")
        );
        // monday after a saturday due date
        assert_eq!(
            next_due("weekly:1", Some("2026-10-24"), today).unwrap().0,
            date("2026-10-26")
        );
        // stale occurrence starts from today, not from the old due date
        assert_eq!(
            next_due("weekly:1", Some("2026-01-05"), today).unwrap().0,
            date("2026-10-26")
        );
        assert_eq!(
            next_due("monthly:1", Some("2026-10-24"), today).unwrap().0,
            date("2026-11-01")
        );
        // months shorter than the rule's day clamp to their last day
        assert_eq!(
            next_due("monthly:31", Some("2026-10-31"), date("2026-10-31"))
                .unwrap()
                .0,
            date("2026-11-30")
        );
    }

    #[test]
    fn labels_repeat_rules() {
        assert_eq!(label("daily").unwrap(), "Every day");
        assert_eq!(label("weekly:1").unwrap(), "Every Monday");
        assert_eq!(label("monthly:1").unwrap(), "Every month on the 1st");
        assert_eq!(label("monthly:22").unwrap(), "Every month on the 22nd");
    }
}
