use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};

// natural language quick-add parser; mirror of src/lib/quickadd/parse.ts,
// kept in sync through the shared fixture in src/lib/quickadd/cases.json.
// see the ts file for the full grammar table
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

fn weekday(token: &str) -> Option<u32> {
    let iso = match token {
        "mon" | "monday" | "pazartesi" | "pzt" => 1,
        "tue" | "tues" | "tuesday" | "salı" | "sal" => 2,
        "wed" | "wednesday" | "çarşamba" | "çar" => 3,
        "thu" | "thur" | "thurs" | "thursday" | "perşembe" | "per" => 4,
        "fri" | "friday" | "cuma" | "cum" => 5,
        "sat" | "saturday" | "cumartesi" | "cmt" => 6,
        "sun" | "sunday" | "pazar" | "paz" => 7,
        _ => return None,
    };
    Some(iso)
}

fn priority(flag: &str) -> Option<i32> {
    let p = match flag {
        "none" | "no" | "p0" | "0" => 0,
        "urgent" | "p1" | "1" => 1,
        "high" | "p2" | "2" => 2,
        "medium" | "med" | "p3" | "3" => 3,
        "low" | "p4" | "4" => 4,
        _ => return None,
    };
    Some(p)
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

fn parse_date_token(token: &str, today: NaiveDate) -> Option<NaiveDate> {
    match token {
        "today" | "bugün" => return Some(today),
        "tomorrow" | "yarın" => return Some(today + Duration::days(1)),
        "haftaya" => return Some(today + Duration::days(7)),
        _ => {}
    }
    // strict shape so the parse stays identical to the ts regex
    let bytes = token.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        if let Ok(d) = NaiveDate::parse_from_str(token, "%Y-%m-%d") {
            return Some(d);
        }
    }
    weekday(token).map(|iso| next_weekday(today, iso))
}

// "15:00", "9:30", "3pm", "3:45pm" -> minutes since midnight
fn parse_time_token(token: &str) -> Option<u32> {
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

// named times of day; consumed only after an explicit date token so plain
// titles like "akşam yemeği" survive
fn named_time(token: &str) -> Option<u32> {
    Some(match token {
        "sabah" => 9 * 60,
        "öğle" | "öğlen" => 12 * 60,
        "akşam" => 19 * 60,
        "gece" => 22 * 60,
        _ => return None,
    })
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
fn normalize_rule(token: &str, today: NaiveDate, is_last: bool) -> Option<String> {
    match token {
        "daily" if is_last => return Some("daily".to_string()),
        "weekly" if is_last => {
            return Some(format!(
                "weekly:{}",
                today.weekday().num_days_from_monday() + 1
            ))
        }
        "monthly" if is_last => return Some(format!("monthly:{}", today.day())),
        _ => {}
    }
    let (kind, arg) = token.split_once(':')?;
    match kind {
        "weekly" => {
            let iso = weekday(arg).or_else(|| {
                arg.parse::<u32>().ok().filter(|n| (1..=7).contains(n))
            })?;
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

fn parse_in(token: &str, today: NaiveDate) -> Option<NaiveDate> {
    if token.len() < 2 {
        return None;
    }
    let (num, unit) = token.split_at(token.len() - 1);
    let n: i64 = num.parse().ok()?;
    match unit {
        "d" => Some(today + Duration::days(n)),
        "w" => Some(today + Duration::weeks(n)),
        "m" => Some(add_months(today, n)),
        _ => None,
    }
}

fn every_rule(token: &str, today: NaiveDate) -> Option<String> {
    match token {
        "day" | "gün" => Some("daily".to_string()),
        "week" | "hafta" => Some(format!(
            "weekly:{}",
            today.weekday().num_days_from_monday() + 1
        )),
        "month" | "ay" => Some(format!("monthly:{}", today.day())),
        _ => weekday(token)
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
            match t {
                "next" | "gelecek" | "haftaya" => {
                    if let Some(iso) = weekday(n) {
                        due = Some(next_weekday(today, iso) + Duration::days(7));
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                    if n == "week" {
                        due = Some(today + Duration::days(7));
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                    if n == "month" {
                        due = Some(add_months(today, 1));
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                }
                "in" => {
                    if let Some(d) = parse_in(n, today) {
                        due = Some(d);
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                }
                "every" | "her" => {
                    if let Some(rule) = every_rule(n, today) {
                        out.recurrence = Some(rule);
                        i += 2;
                        continue;
                    }
                }
                "end" | "until" => {
                    if let Some(d) = parse_date_token(n, today) {
                        out.end_date = Some(d);
                        i += 2;
                        continue;
                    }
                }
                "on" | "by" | "due" => {
                    if let Some(d) = parse_date_token(n, today) {
                        due = Some(d);
                        due_explicit = true;
                        i += 2;
                        continue;
                    }
                }
                "at" => {
                    if let Some(m) = parse_time_token(n) {
                        time_of_day = Some(m);
                        i += 2;
                        continue;
                    }
                }
                "saat" => {
                    // "saat 18" accepts a bare hour on top of regular time forms
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
                _ => {}
            }
        }

        if let Some(d) = parse_date_token(t, today) {
            due = Some(d);
            due_explicit = true;
            i += 1;
            continue;
        }
        if let Some(rule) = normalize_rule(t, today, i == tokens.len() - 1) {
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
            if let Some(m) = named_time(t) {
                time_of_day = Some(m);
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
            if flag == "pin" || flag == "pinned" {
                out.pinned = true;
                i += 1;
                continue;
            }
            if let Some(p) = priority(flag) {
                out.priority = Some(p);
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

