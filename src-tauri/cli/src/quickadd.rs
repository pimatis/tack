use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

// natural language quick-add parser; mirror of src/lib/quickadd/parse.ts.
// both sides read the shared vocabulary in src/lib/quickadd/langs.json so a
// new language is a data change, not a code change. every language is active
// at once - users can mix ("yarın at 5pm"). see the ts file for the grammar table
#[derive(Debug, Default)]
pub struct Parsed {
    pub title: String,
    pub project_key: Option<String>,
    pub label_names: Vec<String>,
    pub priority: Option<i32>,
    pub pinned: bool,
    pub due_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    // local naive datetime; callers convert to the utc iso storage form
    pub reminder: Option<NaiveDateTime>,
    pub recurrence: Option<String>,
    pub estimate_minutes: Option<i64>,
}

#[derive(Deserialize, Default)]
struct Lang {
    #[serde(default)]
    weekdays: Vec<Vec<String>>,
    #[serde(default)]
    today: Vec<String>,
    #[serde(default)]
    tomorrow: Vec<String>,
    #[serde(default, rename = "nextWeek")]
    next_week: Vec<String>,
    #[serde(default, rename = "nextMonth")]
    next_month: Vec<String>,
    #[serde(default)]
    next: Vec<String>,
    #[serde(default, rename = "in")]
    in_words: Vec<String>,
    #[serde(default)]
    every: Vec<String>,
    #[serde(default)]
    day: Vec<String>,
    #[serde(default)]
    week: Vec<String>,
    #[serde(default)]
    month: Vec<String>,
    #[serde(default)]
    end: Vec<String>,
    #[serde(default)]
    due: Vec<String>,
    #[serde(default)]
    at: Vec<String>,
    #[serde(default, rename = "namedTimes")]
    named_times: HashMap<String, u32>,
    #[serde(default)]
    rules: HashMap<String, Vec<String>>,
    #[serde(default)]
    priorities: HashMap<String, Vec<String>>,
    #[serde(default)]
    pin: Vec<String>,
}

#[derive(Default)]
struct Vocab {
    weekdays: HashMap<String, u32>,
    today: HashSet<String>,
    tomorrow: HashSet<String>,
    next_week: HashSet<String>,
    next_month: HashSet<String>,
    next: HashSet<String>,
    in_words: HashSet<String>,
    every: HashSet<String>,
    end: HashSet<String>,
    due: HashSet<String>,
    at: HashSet<String>,
    // localized unit words plus the d/w/m letters -> day / week / month
    units: HashMap<String, char>,
    named_times: HashMap<String, u32>,
    rules: HashMap<String, String>,
    priorities: HashMap<String, i32>,
    pin: HashSet<String>,
}

static VOCAB: OnceLock<Vocab> = OnceLock::new();

fn vocab() -> &'static Vocab {
    VOCAB.get_or_init(|| {
        let langs: HashMap<String, Lang> =
            serde_json::from_str(include_str!("../../../src/lib/quickadd/langs.json"))
                .expect("langs.json must stay valid");
        let mut v = Vocab::default();
        for lang in langs.values() {
            for (i, names) in lang.weekdays.iter().enumerate() {
                for n in names {
                    v.weekdays.insert(n.clone(), i as u32 + 1);
                }
            }
            v.today.extend(lang.today.iter().cloned());
            v.tomorrow.extend(lang.tomorrow.iter().cloned());
            v.next_week.extend(lang.next_week.iter().cloned());
            v.next_month.extend(lang.next_month.iter().cloned());
            v.next.extend(lang.next.iter().cloned());
            v.in_words.extend(lang.in_words.iter().cloned());
            v.every.extend(lang.every.iter().cloned());
            v.end.extend(lang.end.iter().cloned());
            v.due.extend(lang.due.iter().cloned());
            v.at.extend(lang.at.iter().cloned());
            v.units.extend(lang.day.iter().map(|w| (w.clone(), 'd')));
            v.units.extend(lang.week.iter().map(|w| (w.clone(), 'w')));
            v.units.extend(lang.month.iter().map(|w| (w.clone(), 'm')));
            v.named_times.extend(lang.named_times.iter().map(|(k, m)| (k.clone(), *m)));
            for (kind, words) in &lang.rules {
                for w in words {
                    v.rules.insert(w.clone(), kind.clone());
                }
            }
            for (p, words) in &lang.priorities {
                if let Ok(p) = p.parse::<i32>() {
                    for w in words {
                        v.priorities.insert(w.clone(), p);
                    }
                }
            }
            v.pin.extend(lang.pin.iter().cloned());
        }
        // language-neutral forms: unit letters and p0..p4 / digit priorities
        for (w, k) in [("d", 'd'), ("w", 'w'), ("m", 'm')] {
            v.units.insert(w.to_string(), k);
        }
        for p in 0..=4i32 {
            v.priorities.insert(format!("p{}", p), p);
            v.priorities.insert(p.to_string(), p);
        }
        v
    })
}

// first occurrence of the iso weekday strictly after today
fn next_weekday(today: NaiveDate, iso: u32) -> NaiveDate {
    let cur = today.weekday().num_days_from_monday() + 1;
    let delta = (iso + 7 - cur) % 7;
    today + Duration::days(if delta == 0 { 7 } else { delta as i64 })
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (y, m) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    (NaiveDate::from_ymd_opt(y, m, 1).unwrap() - Duration::days(1)).day()
}

// shift the calendar month, clamping to the target month's last day
fn add_months(date: NaiveDate, months: i64) -> NaiveDate {
    let total = date.year() as i64 * 12 + date.month() as i64 - 1 + months;
    let (year, month) = (total.div_euclid(12) as i32, total.rem_euclid(12) as u32 + 1);
    let day = date.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
}

fn parse_date_token(token: &str, today: NaiveDate, v: &Vocab) -> Option<NaiveDate> {
    if v.today.contains(token) {
        return Some(today);
    }
    if v.tomorrow.contains(token) {
        return Some(today + Duration::days(1));
    }
    if v.next_week.contains(token) {
        return Some(today + Duration::days(7));
    }
    if v.next_month.contains(token) {
        return Some(add_months(today, 1));
    }
    // strict shape so the parse stays identical to the ts regex
    let bytes = token.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        if let Ok(d) = NaiveDate::parse_from_str(token, "%Y-%m-%d") {
            return Some(d);
        }
    }
    v.weekdays.get(token).map(|iso| next_weekday(today, *iso))
}

// "15:00", "9:30", "3pm", "3:45pm", "18h30" -> minutes since midnight
fn parse_time_token(token: &str) -> Option<u32> {
    // "18h" / "18h30" (fr)
    if let Some((h_str, m_str)) = token.split_once('h') {
        if !h_str.is_empty()
            && h_str.bytes().all(|b| b.is_ascii_digit())
            && (m_str.is_empty()
                || (m_str.len() == 2 && m_str.bytes().all(|b| b.is_ascii_digit())))
        {
            let h: u32 = h_str.parse().ok()?;
            let m: u32 = if m_str.is_empty() { 0 } else { m_str.parse().ok()? };
            if h <= 23 && m <= 59 {
                return Some(h * 60 + m);
            }
        }
    }
    let (body, suffix) = if let Some(b) = token.strip_suffix("am") {
        (b, Some("am"))
    } else if let Some(b) = token.strip_suffix("pm") {
        (b, Some("pm"))
    } else {
        (token, None)
    };
    // a bare number without suffix or colon is not a time token
    if suffix.is_none() && !body.contains(':') {
        return None;
    }
    let (h_str, m_str) = match body.split_once(':') {
        Some((h, m)) => (h, m),
        None => (body, "0"),
    };
    if h_str.is_empty() || !m_str.is_empty() && !m_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !h_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut h: u32 = h_str.parse().ok()?;
    let m: u32 = if m_str.is_empty() { 0 } else { m_str.parse().ok()? };
    if m > 59 {
        return None;
    }
    match suffix {
        None => {
            if h > 23 {
                return None;
            }
        }
        Some("am") => {
            if !(1..=12).contains(&h) {
                return None;
            }
            if h == 12 {
                h = 0;
            }
        }
        _ => {
            if !(1..=12).contains(&h) {
                return None;
            }
            if h != 12 {
                h += 12;
            }
        }
    }
    Some(h * 60 + m)
}

// "~90", "~90m", "~2h", "~1h30m", "~1.5h" -> minutes, same format as --estimate
fn parse_estimate_token(token: &str) -> Option<i64> {
    let inner = token.strip_prefix('~')?;
    if inner.is_empty() {
        return None;
    }
    if let Some((h, rest)) = inner.split_once('h') {
        let hours: f64 = h.parse().ok()?;
        let mins: i64 = rest.trim_end_matches('m').parse().unwrap_or(0);
        return Some((hours * 60.0).round() as i64 + mins);
    }
    inner.trim_end_matches('m').parse().ok()
}

// bare "daily" / "weekly" / "monthly" only count as a rule as the last word,
// so titles like "weekly sync" survive; "weekly:N" / "monthly:N" works anywhere
fn normalize_rule(token: &str, today: NaiveDate, is_last: bool, v: &Vocab) -> Option<String> {
    if let Some(kind) = v.rules.get(token) {
        if !is_last {
            return None;
        }
        return Some(match kind.as_str() {
            "daily" => "daily".to_string(),
            "weekly" => format!("weekly:{}", today.weekday().num_days_from_monday() + 1),
            _ => format!("monthly:{}", today.day()),
        });
    }
    let (kind, arg) = token.split_once(':')?;
    match kind {
        "weekly" => {
            let iso = v
                .weekdays
                .get(arg)
                .copied()
                .or_else(|| arg.parse::<u32>().ok().filter(|n| (1..=7).contains(n)))?;
            Some(format!("weekly:{}", iso))
        }
        "monthly" => {
            let day: u32 = arg.parse().ok()?;
            (1..=31)
                .contains(&day)
                .then(|| format!("monthly:{}", day))
        }
        _ => None,
    }
}

// "<in> 3d" / "<in> 3 days" -> (due date, tokens consumed)
fn parse_in(
    token: &str,
    lookahead: Option<&str>,
    today: NaiveDate,
    v: &Vocab,
) -> Option<(NaiveDate, usize)> {
    let digits: usize = token.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let (num, suffix) = token.split_at(digits);
    let n: i64 = num.parse().ok()?;
    let (unit, consumed) = if suffix.is_empty() {
        (v.units.get(lookahead?)?, 3)
    } else {
        (v.units.get(suffix)?, 2)
    };
    let date = match unit {
        'd' => today + Duration::days(n),
        'w' => today + Duration::weeks(n),
        'm' => add_months(today, n),
        _ => return None,
    };
    Some((date, consumed))
}

fn every_rule(token: &str, today: NaiveDate, v: &Vocab) -> Option<String> {
    match v.units.get(token) {
        Some('d') => Some("daily".to_string()),
        Some('w') => Some(format!(
            "weekly:{}",
            today.weekday().num_days_from_monday() + 1
        )),
        Some('m') => Some(format!("monthly:{}", today.day())),
        _ => v
            .weekdays
            .get(token)
            .map(|iso| format!("weekly:{}", iso))
            .or_else(|| {
                token
                    .parse::<u32>()
                    .ok()
                    .filter(|d| (1..=31).contains(d))
                    .map(|d| format!("monthly:{}", d))
            }),
    }
}

pub fn parse(input: &str, today: NaiveDate, now: NaiveTime) -> Parsed {
    let v = vocab();
    // lowercase copy drives matching; the original casing feeds the title
    let raw_tokens: Vec<&str> = input.split_whitespace().collect();
    let tokens: Vec<String> = raw_tokens.iter().map(|t| t.to_lowercase()).collect();
    let mut title: Vec<&str> = Vec::new();
    let mut out = Parsed::default();
    let mut due: Option<NaiveDate> = None;
    let mut due_explicit = false;
    let mut time_of_day: Option<u32> = None;

    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        // lead words consume the following token only when it matches, so a
        // plain "meet at home" keeps both words in the title
        if let Some(n) = tokens.get(i + 1).map(|s| s.as_str()) {
            if v.next.contains(t) {
                if let Some(iso) = v.weekdays.get(n) {
                    due = Some(next_weekday(today, *iso) + Duration::days(7));
                    due_explicit = true;
                    i += 2;
                    continue;
                }
                match v.units.get(n) {
                    Some('w') => {
                        due = Some(today + Duration::days(7));
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                    Some('m') => {
                        due = Some(add_months(today, 1));
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }
            if v.in_words.contains(t) {
                if let Some((d, consumed)) =
                    parse_in(n, tokens.get(i + 2).map(|s| s.as_str()), today, v)
                {
                    due = Some(d);
                    due_explicit = true;
                    i += consumed;
                    continue;
                }
            }
            if v.every.contains(t) {
                if let Some(rule) = every_rule(n, today, v) {
                    out.recurrence = Some(rule);
                    i += 2;
                    continue;
                }
            }
            if v.end.contains(t) {
                if let Some(d) = parse_date_token(n, today, v) {
                    out.end_date = Some(d);
                    i += 2;
                    continue;
                }
            }
            if v.due.contains(t) {
                if let Some(d) = parse_date_token(n, today, v) {
                    due = Some(d);
                    due_explicit = true;
                    i += 2;
                    continue;
                }
            }
            if v.at.contains(t) {
                // "<at> 18" accepts a bare hour on top of regular time forms
                let bare = n
                    .parse::<u32>()
                    .ok()
                    .filter(|h| *h <= 23)
                    .map(|h| h * 60);
                if let Some(m) = parse_time_token(n).or(bare) {
                    time_of_day = Some(m);
                    i += 2;
                    continue;
                }
            }
        }

        if let Some(d) = parse_date_token(t, today, v) {
            due = Some(d);
            due_explicit = true;
            i += 1;
            continue;
        }
        if let Some(rule) = normalize_rule(t, today, i == tokens.len() - 1, v) {
            out.recurrence = Some(rule);
            i += 1;
            continue;
        }
        if let Some(m) = parse_time_token(t) {
            time_of_day = Some(m);
            i += 1;
            continue;
        }
        if due_explicit {
            if let Some(m) = v.named_times.get(t) {
                time_of_day = Some(*m);
                i += 1;
                continue;
            }
        }
        if let Some(key) = t.strip_prefix('#').filter(|s| !s.is_empty()) {
            out.project_key = Some(key.to_string());
            i += 1;
            continue;
        }
        if let Some(name) = t.strip_prefix('+').filter(|s| !s.is_empty()) {
            out.label_names.push(name.to_string());
            i += 1;
            continue;
        }
        if let Some(flag) = t.strip_prefix('!').filter(|s| !s.is_empty()) {
            if v.pin.contains(flag) {
                out.pinned = true;
                i += 1;
                continue;
            }
            if let Some(p) = v.priorities.get(flag) {
                out.priority = Some(*p);
                i += 1;
                continue;
            }
        }
        if let Some(est) = parse_estimate_token(t) {
            out.estimate_minutes = Some(est);
            i += 1;
            continue;
        }

        title.push(raw_tokens[i]);
        i += 1;
    }

    out.title = title.join(" ");
    out.due_date = due;

    if let Some(m) = time_of_day {
        let mut day = due.unwrap_or(today);
        let time = NaiveTime::from_hms_opt(m / 60, m % 60, 0).unwrap();
        if !due_explicit && day.and_time(time) <= today.and_time(now) {
            day += Duration::days(1);
        }
        out.reminder = Some(day.and_time(time));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        // sunday 2026-11-01
        NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
    }

    fn now() -> NaiveTime {
        NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    }

    fn date(d: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(2026, 11, d)
    }

    #[test]
    fn english_still_works() {
        let p = parse("buy milk tomorrow !high #home", today(), now());
        assert_eq!(p.title, "buy milk");
        assert_eq!(p.due_date, date(2));
        assert_eq!(p.priority, Some(2));
        assert_eq!(p.project_key.as_deref(), Some("home"));
    }

    #[test]
    fn turkish_still_works() {
        let p = parse("raporu yar\u{131}n saat 18 !acil", today(), now());
        assert_eq!(p.title, "raporu");
        assert_eq!(p.due_date, date(2));
        assert_eq!(p.reminder.unwrap().time(), NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        assert_eq!(p.priority, Some(1));
    }

    #[test]
    fn turkish_every() {
        let p = parse("spor her pazartesi", today(), now());
        assert_eq!(p.recurrence.as_deref(), Some("weekly:1"));
    }

    #[test]
    fn german_next_week() {
        let p = parse("review n\u{e4}chste woche", today(), now());
        assert_eq!(p.title, "review");
        assert_eq!(p.due_date, date(8));
    }

    #[test]
    fn german_due_and_time() {
        let p = parse("anruf am montag um 9:00", today(), now());
        assert_eq!(p.title, "anruf");
        assert_eq!(p.due_date, date(2));
        assert_eq!(p.reminder.unwrap().time(), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    }

    #[test]
    fn french_date_and_h_time() {
        let p = parse("r\u{e9}union demain \u{e0} 18h30", today(), now());
        assert_eq!(p.title, "r\u{e9}union");
        assert_eq!(p.due_date, date(2));
        assert_eq!(p.reminder.unwrap().time(), NaiveTime::from_hms_opt(18, 30, 0).unwrap());
    }

    #[test]
    fn french_in_words() {
        let p = parse("appeler dans 3 jours", today(), now());
        assert_eq!(p.due_date, date(4));
    }

    #[test]
    fn spanish_every() {
        let p = parse("gym cada lunes", today(), now());
        assert_eq!(p.recurrence.as_deref(), Some("weekly:1"));
    }

    #[test]
    fn in_three_days_words() {
        let p = parse("call in 3 days", today(), now());
        assert_eq!(p.due_date, date(4));
    }

    #[test]
    fn russian_in_and_weekday() {
        let p = parse(
            "\u{43f}\u{43e}\u{437}\u{432}\u{43e}\u{43d}\u{438}\u{442}\u{44c} \u{447}\u{435}\u{440}\u{435}\u{437} 2 \u{43d}\u{435}\u{434}\u{435}\u{43b}\u{438}",
            today(),
            now(),
        );
        assert_eq!(p.due_date, date(15));
        let p = parse(
            "\u{432} \u{441}\u{440}\u{435}\u{434}\u{443}",
            today(),
            now(),
        );
        assert_eq!(p.due_date, date(4)); // wednesday
    }

    #[test]
    fn japanese_tomorrow() {
        let p = parse("\u{4f1a}\u{8b70} \u{660e}\u{65e5}", today(), now());
        assert_eq!(p.title, "\u{4f1a}\u{8b70}");
        assert_eq!(p.due_date, date(2));
    }

    #[test]
    fn chinese_every_day() {
        let p = parse("\u{8dd1}\u{6b65} \u{6bcf}\u{5929}", today(), now());
        assert_eq!(p.recurrence.as_deref(), Some("daily"));
    }

    #[test]
    fn title_survives() {
        let p = parse("meet at home", today(), now());
        assert_eq!(p.title, "meet at home");
        assert_eq!(p.due_date, None);
        assert_eq!(p.reminder, None);
    }
}
