use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset,
    TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::LazyLock;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Errors returned by engine operations.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The request JSON was malformed or contained invalid field values.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// The provided IANA time-zone identifier could not be resolved.
    #[error("invalid zone: {0}")]
    InvalidZone(String),
    /// A datetime string could not be parsed into a valid instant or local time.
    #[error("invalid datetime: {0}")]
    InvalidDateTime(String),
    /// The local time is ambiguous (e.g. during a DST fall-back transition).
    #[error("ambiguous local time: {0}")]
    AmbiguousLocalTime(String),
    /// The local time does not exist (e.g. during a DST spring-forward transition).
    #[error("non-existent local time: {0}")]
    NonExistentLocalTime(String),
    /// The natural-language input did not match any supported grammar.
    #[error("unsupported intent grammar")]
    UnsupportedIntent,
    /// An ISO 8601 duration string could not be parsed.
    #[error("invalid duration: {0}")]
    InvalidDuration(String),
}

/// Structured error payload included in an [`EngineResponse`] when `ok` is `false`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// Machine-readable error code (e.g. `"invalid_zone"`).
    pub code: String,
    /// Human-readable description of the error.
    pub message: String,
}

/// Top-level response envelope returned by every engine operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineResponse {
    /// `true` when the operation succeeded; `false` on error.
    pub ok: bool,
    /// The operation result payload, present only on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// The error details, present only on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
}

impl EngineResponse {
    fn ok(value: Value) -> Self {
        Self {
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    fn err(err: EngineError) -> Self {
        let (code, message) = match err {
            EngineError::InvalidRequest(m) => ("invalid_request".to_string(), m),
            EngineError::InvalidZone(m) => ("invalid_zone".to_string(), m),
            EngineError::InvalidDateTime(m) => ("invalid_datetime".to_string(), m),
            EngineError::AmbiguousLocalTime(m) => ("ambiguous_local_time".to_string(), m),
            EngineError::NonExistentLocalTime(m) => ("non_existent_local_time".to_string(), m),
            EngineError::UnsupportedIntent => (
                "unsupported_intent".to_string(),
                "input does not match deterministic grammar".to_string(),
            ),
            EngineError::InvalidDuration(m) => ("invalid_duration".to_string(), m),
        };

        Self {
            ok: false,
            value: None,
            error: Some(ErrorPayload { code, message }),
        }
    }
}

/// Strategy for resolving ambiguous or non-existent local times during DST transitions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Disambiguation {
    /// Use the offset that keeps wall-clock time moving forward (default).
    #[default]
    Compatible,
    /// Pick the earlier of two ambiguous instants.
    Earlier,
    /// Pick the later of two ambiguous instants.
    Later,
    /// Return an error instead of silently choosing.
    Reject,
}

/// Controls how duration arithmetic is performed relative to a time zone.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticMode {
    /// Add exact elapsed seconds, ignoring calendar boundaries and DST.
    Absolute,
    /// Add calendar units in local time, then re-resolve through the time zone (default).
    #[default]
    Calendar,
}

/// Calendar unit to which an instant can be snapped (truncated or ceiled).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapUnit {
    /// Snap to the boundary of an hour.
    Hour,
    /// Snap to the boundary of a day.
    Day,
    /// Snap to the boundary of a week.
    Week,
    /// Snap to the boundary of a calendar month.
    Month,
    /// Snap to the boundary of a calendar quarter.
    Quarter,
    /// Snap to the boundary of a calendar year.
    Year,
}

/// Which edge of the calendar unit to snap to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapEdge {
    /// Snap to the start (floor) of the unit.
    Start,
    /// Snap to the end (ceiling) of the unit.
    End,
}

/// Kind of relationship test to perform between two time intervals.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntervalCheckMode {
    /// Check whether the two intervals overlap.
    Overlap,
    /// Check whether one interval fully contains the other.
    Contains,
    /// Compute the gap (or overlap) between the two intervals.
    Gap,
}

/// A half-open time interval defined by two RFC 3339 instant strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeInterval {
    /// Inclusive start instant (RFC 3339).
    pub start: String,
    /// Exclusive end instant (RFC 3339).
    pub end: String,
}

fn default_week_starts_on() -> String {
    "monday".to_string()
}

/// A decomposed duration with separate calendar and clock fields.
///
/// Fields default to zero when omitted from JSON input.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DurationSpec {
    /// Number of years.
    #[serde(default)]
    pub years: i32,
    /// Number of months.
    #[serde(default)]
    pub months: i32,
    /// Number of weeks.
    #[serde(default)]
    pub weeks: i64,
    /// Number of days.
    #[serde(default)]
    pub days: i64,
    /// Number of hours.
    #[serde(default)]
    pub hours: i64,
    /// Number of minutes.
    #[serde(default)]
    pub minutes: i64,
    /// Number of seconds.
    #[serde(default)]
    pub seconds: i64,
}

/// Configuration for skipping non-business days in recurrence calculations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BusinessCalendar {
    /// When `true`, Saturday and Sunday are excluded from occurrences.
    #[serde(default)]
    pub exclude_weekends: bool,
    /// ISO 8601 date strings (`YYYY-MM-DD`) of additional days to skip.
    #[serde(default)]
    pub holidays: Vec<String>,
}

/// Recurrence frequency for [`RecurrenceRule`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    /// Repeat every N days.
    Daily,
    /// Repeat every N weeks.
    Weekly,
    /// Repeat every N months.
    Monthly,
}

/// Describes a repeating schedule used by the `recurrence_preview` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRule {
    /// How often the recurrence repeats (daily, weekly, or monthly).
    pub frequency: Frequency,
    /// Step size between occurrences (e.g. `2` means every other period). Defaults to `1`.
    #[serde(default = "default_interval")]
    pub interval: u32,
    /// Maximum number of occurrences to generate.
    pub count: usize,
    /// Optional weekday filter (e.g. `["monday", "wednesday"]`).
    #[serde(default)]
    pub by_weekdays: Vec<String>,
}

fn default_interval() -> u32 {
    1
}

fn default_format() -> String {
    "extended".to_string()
}

/// A tagged-union request dispatched to the engine via the `"op"` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Parse an RFC 3339 or ISO 8601 string into a UTC instant.
    ParseInstant {
        input: String,
    },
    /// Format a UTC instant in a given time zone.
    FormatInstant {
        instant: String,
        zone: String,
        #[serde(default = "default_format")]
        format: String,
    },
    /// Resolve a wall-clock local time to a UTC instant in the given zone.
    ResolveLocal {
        local: String,
        zone: String,
        #[serde(default)]
        disambiguation: Disambiguation,
    },
    /// Add a duration to a UTC instant using the specified arithmetic mode.
    AddDuration {
        start: String,
        zone: String,
        #[serde(default)]
        duration: DurationSpec,
        #[serde(default)]
        arithmetic: ArithmeticMode,
        #[serde(default)]
        disambiguation: Disambiguation,
    },
    /// Generate a series of recurring occurrences from a start time and rule.
    RecurrencePreview {
        start_local: String,
        zone: String,
        rule: RecurrenceRule,
        #[serde(default)]
        business_calendar: Option<BusinessCalendar>,
        #[serde(default)]
        disambiguation: Disambiguation,
    },
    /// Convert a natural-language scheduling phrase into a structured request.
    NormalizeIntent {
        input: String,
        reference_local: String,
        default_zone: String,
    },
    /// Compute the calendar difference between two instants.
    DiffInstants {
        start: String,
        end: String,
        zone: String,
    },
    /// Compare two instants and return their ordering.
    CompareInstants {
        a: String,
        b: String,
    },
    /// Snap (floor or ceil) an instant to a calendar-unit boundary in a given zone.
    SnapTo {
        instant: String,
        zone: String,
        unit: SnapUnit,
        edge: SnapEdge,
        #[serde(default = "default_week_starts_on")]
        week_starts_on: String,
    },
    /// Parse an ISO 8601 duration string into a [`DurationSpec`].
    ParseDuration {
        input: String,
    },
    /// Format a [`DurationSpec`] into an ISO 8601 duration string.
    FormatDuration {
        duration: DurationSpec,
    },
    /// Test a relationship (overlap, containment, or gap) between two intervals.
    IntervalCheck {
        interval_a: TimeInterval,
        interval_b: TimeInterval,
        mode: IntervalCheckMode,
    },
    /// Return metadata (offset, abbreviation) for a time zone at a given instant.
    ZoneInfo {
        zone: String,
        #[serde(default)]
        at: Option<String>,
    },
    /// List all known IANA time zones, optionally filtered by region prefix.
    ListZones {
        #[serde(default)]
        region_filter: Option<String>,
    },
    /// Return the current time, optionally projected into a specific zone.
    Now {
        #[serde(default)]
        zone: Option<String>,
    },
}

/// Evaluate a typed [`Request`] and return an [`EngineResponse`].
pub fn evaluate_request(request: Request) -> EngineResponse {
    match evaluate_request_inner(request) {
        Ok(value) => EngineResponse::ok(value),
        Err(err) => EngineResponse::err(err),
    }
}

/// Deserialize a [`serde_json::Value`] into a [`Request`] and evaluate it.
pub fn evaluate_request_value(value: Value) -> EngineResponse {
    let parsed = serde_json::from_value::<Request>(value);
    match parsed {
        Ok(request) => evaluate_request(request),
        Err(err) => EngineResponse::err(EngineError::InvalidRequest(err.to_string())),
    }
}

/// Parse a JSON string, evaluate the request, and return the response as a JSON string.
pub fn evaluate_json(input: &str) -> Result<String, EngineError> {
    let value: Value =
        serde_json::from_str(input).map_err(|e| EngineError::InvalidRequest(e.to_string()))?;
    let response = evaluate_request_value(value);
    serde_json::to_string(&response).map_err(|e| EngineError::InvalidRequest(e.to_string()))
}

/// WASM entry-point: parse a JSON request string and return a JSON response string.
///
/// Always returns valid JSON; serialization failures produce a hard-coded error object.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn evaluate_json_wasm(input: &str) -> String {
    let response = match serde_json::from_str::<Value>(input) {
        Ok(value) => evaluate_request_value(value),
        Err(err) => EngineResponse::err(EngineError::InvalidRequest(err.to_string())),
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"ok\":false,\"error\":{\"code\":\"serialization_error\",\"message\":\"failed to encode response\"}}"
            .to_string()
    })
}

fn evaluate_request_inner(request: Request) -> Result<Value, EngineError> {
    match request {
        Request::ParseInstant { input } => op_parse_instant(&input),
        Request::FormatInstant {
            instant,
            zone,
            format,
        } => op_format_instant(&instant, &zone, &format),
        Request::ResolveLocal {
            local,
            zone,
            disambiguation,
        } => op_resolve_local(&local, &zone, disambiguation),
        Request::AddDuration {
            start,
            zone,
            duration,
            arithmetic,
            disambiguation,
        } => op_add_duration(&start, &zone, &duration, arithmetic, disambiguation),
        Request::RecurrencePreview {
            start_local,
            zone,
            rule,
            business_calendar,
            disambiguation,
        } => op_recurrence_preview(
            &start_local,
            &zone,
            &rule,
            business_calendar.as_ref(),
            disambiguation,
        ),
        Request::NormalizeIntent {
            input,
            reference_local,
            default_zone,
        } => op_normalize_intent(&input, &reference_local, &default_zone),
        Request::DiffInstants { start, end, zone } => op_diff_instants(&start, &end, &zone),
        Request::CompareInstants { a, b } => op_compare_instants(&a, &b),
        Request::SnapTo {
            instant,
            zone,
            unit,
            edge,
            week_starts_on,
        } => op_snap_to(&instant, &zone, unit, edge, &week_starts_on),
        Request::ParseDuration { input } => op_parse_duration(&input),
        Request::FormatDuration { duration } => op_format_duration(&duration),
        Request::IntervalCheck {
            interval_a,
            interval_b,
            mode,
        } => op_interval_check(&interval_a, &interval_b, mode),
        Request::ZoneInfo { zone, at } => op_zone_info(&zone, at.as_deref()),
        Request::ListZones { region_filter } => op_list_zones(region_filter.as_deref()),
        Request::Now { zone } => op_now(zone.as_deref()),
    }
}

fn op_parse_instant(input: &str) -> Result<Value, EngineError> {
    let dt = parse_instant_str(input)?;
    Ok(json!({
        "instant": dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "epoch_seconds": dt.timestamp(),
        "epoch_millis": dt.timestamp_millis(),
    }))
}

fn op_format_instant(instant: &str, zone: &str, format: &str) -> Result<Value, EngineError> {
    let utc = parse_instant_str(instant)?;
    let tz = parse_zone(zone)?;
    let local = utc.with_timezone(&tz);
    let formatted = format_zoned(local, format)?;

    Ok(json!({
        "input_instant": utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "zone": zone,
        "format": format,
        "formatted": formatted,
        "offset_seconds": local.offset().fix().local_minus_utc(),
    }))
}

fn op_resolve_local(local: &str, zone: &str, disambiguation: Disambiguation) -> Result<Value, EngineError> {
    let naive = parse_local_datetime(local)?;
    let tz = parse_zone(zone)?;
    let (resolved, applied) = resolve_local_datetime(naive, tz, disambiguation)?;

    Ok(json!({
        "zone": zone,
        "input_local": local,
        "resolved_local": resolved.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "zoned": fmt_zoned(resolved),
        "instant": resolved.with_timezone(&Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "offset_seconds": resolved.offset().fix().local_minus_utc(),
        "disambiguation_applied": applied,
    }))
}

fn op_add_duration(
    start: &str,
    zone: &str,
    duration: &DurationSpec,
    arithmetic: ArithmeticMode,
    disambiguation: Disambiguation,
) -> Result<Value, EngineError> {
    let start_utc = parse_instant_str(start)?;
    let tz = parse_zone(zone)?;

    let result = match arithmetic {
        ArithmeticMode::Absolute => {
            if duration.years != 0 || duration.months != 0 {
                return Err(EngineError::InvalidRequest(
                    "absolute arithmetic does not support months/years".to_string(),
                ));
            }
            let total_secs = duration.seconds
                + duration.minutes * 60
                + duration.hours * 3600
                + duration.days * 86_400
                + duration.weeks * 604_800;
            start_utc + Duration::seconds(total_secs)
        }
        ArithmeticMode::Calendar => {
            let start_local = start_utc.with_timezone(&tz).naive_local();
            let mut local = add_months_clamped(
                start_local,
                duration.years.saturating_mul(12) + duration.months,
            )?;
            local += Duration::days(duration.days + duration.weeks * 7);
            local += Duration::hours(duration.hours);
            local += Duration::minutes(duration.minutes);
            local += Duration::seconds(duration.seconds);
            let (resolved, _) = resolve_local_datetime(local, tz, disambiguation)?;
            resolved.with_timezone(&Utc)
        }
    };

    let result_local = result.with_timezone(&tz);

    Ok(json!({
        "zone": zone,
        "arithmetic": arithmetic,
        "start": start_utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "duration": duration,
        "result": {
            "instant": result.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "zoned": fmt_zoned(result_local),
            "local": result_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "offset_seconds": result_local.offset().fix().local_minus_utc(),
        }
    }))
}

fn op_recurrence_preview(
    start_local: &str,
    zone: &str,
    rule: &RecurrenceRule,
    business_calendar: Option<&BusinessCalendar>,
    disambiguation: Disambiguation,
) -> Result<Value, EngineError> {
    if rule.count == 0 {
        return Err(EngineError::InvalidRequest(
            "recurrence count must be > 0".to_string(),
        ));
    }
    if rule.interval == 0 {
        return Err(EngineError::InvalidRequest(
            "recurrence interval must be > 0".to_string(),
        ));
    }

    let tz = parse_zone(zone)?;
    let start = parse_local_datetime(start_local)?;
    let mut out: Vec<Value> = Vec::with_capacity(rule.count);

    let weekday_filter: Option<HashSet<Weekday>> = if rule.by_weekdays.is_empty() {
        None
    } else {
        let mut set = HashSet::new();
        for item in &rule.by_weekdays {
            set.insert(parse_weekday(item)?);
        }
        Some(set)
    };

    let bc = business_calendar.cloned().unwrap_or_default();
    let holidays = parse_holidays(&bc.holidays)?;

    match rule.frequency {
        Frequency::Daily => {
            let mut i: i64 = 0;
            while out.len() < rule.count {
                let candidate = start + Duration::days(i * rule.interval as i64);
                maybe_push_occurrence(
                    &mut out,
                    candidate,
                    tz,
                    &weekday_filter,
                    &bc,
                    &holidays,
                    disambiguation,
                )?;
                i += 1;
                if i > (rule.count as i64) * 32 {
                    break;
                }
            }
        }
        Frequency::Weekly => {
            let start_date = start.date();
            let mut day = start_date;
            let start_time = start.time();
            let mut safety = 0usize;
            while out.len() < rule.count {
                let day_delta = (day - start_date).num_days();
                let week_index = day_delta.div_euclid(7);
                let in_week_stride = week_index >= 0 && week_index % (rule.interval as i64) == 0;
                if in_week_stride {
                    let weekday_ok = weekday_filter
                        .as_ref()
                        .map(|set| set.contains(&day.weekday()))
                        .unwrap_or(day.weekday() == start_date.weekday());
                    if weekday_ok {
                        let candidate = day.and_time(start_time);
                        maybe_push_occurrence(
                            &mut out,
                            candidate,
                            tz,
                            &weekday_filter,
                            &bc,
                            &holidays,
                            disambiguation,
                        )?;
                    }
                }
                day = day
                    .checked_add_days(chrono::Days::new(1))
                    .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?;
                safety += 1;
                if safety > rule.count * 730 {
                    break;
                }
            }
        }
        Frequency::Monthly => {
            let mut i: i32 = 0;
            while out.len() < rule.count {
                let candidate = add_months_clamped(start, i.saturating_mul(rule.interval as i32))?;
                maybe_push_occurrence(
                    &mut out,
                    candidate,
                    tz,
                    &weekday_filter,
                    &bc,
                    &holidays,
                    disambiguation,
                )?;
                i += 1;
                if i > (rule.count as i32) * 36 {
                    break;
                }
            }
        }
    }

    Ok(json!({
        "zone": zone,
        "start_local": start_local,
        "rule": rule,
        "business_calendar": bc,
        "occurrences": out,
    }))
}

fn op_normalize_intent(
    input: &str,
    reference_local: &str,
    default_zone: &str,
) -> Result<Value, EngineError> {
    let reference = parse_local_datetime(reference_local)?;
    let _ = parse_zone(default_zone)?;
    let raw = input.trim();

    static TOMORROW_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^tomorrow at (?P<time>\d{1,2}:\d{2})(?: in (?P<zone>[A-Za-z0-9_./+-]+))?$",
        )
        .unwrap()
    });

    static NEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^next (?P<weekday>monday|tuesday|wednesday|thursday|friday|saturday|sunday) at (?P<time>\d{1,2}:\d{2})(?: in (?P<zone>[A-Za-z0-9_./+-]+))?$")
            .unwrap()
    });

    static IN_DAYS_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^in (?P<days>\d+) days? at (?P<time>\d{1,2}:\d{2})(?: in (?P<zone>[A-Za-z0-9_./+-]+))?$",
        )
        .unwrap()
    });

    static ON_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^on (?P<date>\d{4}-\d{2}-\d{2}) at (?P<time>\d{1,2}:\d{2})(?: in (?P<zone>[A-Za-z0-9_./+-]+))?$",
        )
        .unwrap()
    });

    static EVERY_WEEKDAY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^every weekday at (?P<time>\d{1,2}:\d{2})(?: in (?P<zone>[A-Za-z0-9_./+-]+))?(?: for (?P<count>\d+) occurrences?)?$",
        )
        .unwrap()
    });

    static EVERY_DAY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)^every (?P<weekday>monday|tuesday|wednesday|thursday|friday|saturday|sunday) at (?P<time>\d{1,2}:\d{2})(?: in (?P<zone>[A-Za-z0-9_./+-]+))?(?: for (?P<count>\d+) occurrences?)?$",
        )
        .unwrap()
    });

    if let Some(caps) = TOMORROW_RE.captures(raw) {
        let time = parse_hhmm(caps.name("time").expect("time capture exists").as_str())?;
        let zone = caps.name("zone").map(|m| m.as_str()).unwrap_or(default_zone);
        let _ = parse_zone(zone)?;
        let target = reference
            .date()
            .checked_add_days(chrono::Days::new(1))
            .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?
            .and_time(time);
        return Ok(one_off_intent(raw, zone, target));
    }

    if let Some(caps) = NEXT_RE.captures(raw) {
        let time = parse_hhmm(caps.name("time").expect("time capture exists").as_str())?;
        let zone = caps.name("zone").map(|m| m.as_str()).unwrap_or(default_zone);
        let _ = parse_zone(zone)?;
        let wanted = parse_weekday(caps.name("weekday").expect("weekday capture exists").as_str())?;

        let mut day = reference.date();
        for _ in 0..7 {
            day = day
                .checked_add_days(chrono::Days::new(1))
                .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?;
            if day.weekday() == wanted {
                break;
            }
        }

        return Ok(one_off_intent(raw, zone, day.and_time(time)));
    }

    if let Some(caps) = IN_DAYS_RE.captures(raw) {
        let time = parse_hhmm(caps.name("time").expect("time capture exists").as_str())?;
        let zone = caps.name("zone").map(|m| m.as_str()).unwrap_or(default_zone);
        let _ = parse_zone(zone)?;
        let days: u64 = caps
            .name("days")
            .expect("days capture exists")
            .as_str()
            .parse()
            .map_err(|e| EngineError::InvalidRequest(format!("invalid day count: {e}")))?;

        let day = reference
            .date()
            .checked_add_days(chrono::Days::new(days))
            .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?;
        return Ok(one_off_intent(raw, zone, day.and_time(time)));
    }

    if let Some(caps) = ON_DATE_RE.captures(raw) {
        let time = parse_hhmm(caps.name("time").expect("time capture exists").as_str())?;
        let zone = caps.name("zone").map(|m| m.as_str()).unwrap_or(default_zone);
        let _ = parse_zone(zone)?;
        let date = NaiveDate::parse_from_str(
            caps.name("date").expect("date capture exists").as_str(),
            "%Y-%m-%d",
        )
        .map_err(|e| EngineError::InvalidDateTime(e.to_string()))?;
        return Ok(one_off_intent(raw, zone, date.and_time(time)));
    }

    if let Some(caps) = EVERY_WEEKDAY_RE.captures(raw) {
        let time = parse_hhmm(caps.name("time").expect("time capture exists").as_str())?;
        let zone = caps.name("zone").map(|m| m.as_str()).unwrap_or(default_zone);
        let _ = parse_zone(zone)?;
        let count = caps
            .name("count")
            .map(|m| m.as_str().parse::<usize>())
            .transpose()
            .map_err(|e| EngineError::InvalidRequest(format!("invalid count: {e}")))?
            .unwrap_or(10);

        let start_day = reference
            .date()
            .checked_add_days(chrono::Days::new(1))
            .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?;
        let start_local = start_day.and_time(time);

        return Ok(json!({
            "kind": "recurrence",
            "source": raw,
            "zone": zone,
            "start_local": start_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "normalized_request": {
                "op": "recurrence_preview",
                "start_local": start_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "zone": zone,
                "rule": {
                    "frequency": "weekly",
                    "interval": 1,
                    "count": count,
                    "by_weekdays": ["monday", "tuesday", "wednesday", "thursday", "friday"]
                },
                "business_calendar": {
                    "exclude_weekends": true,
                    "holidays": []
                },
                "disambiguation": "compatible"
            }
        }));
    }

    if let Some(caps) = EVERY_DAY_RE.captures(raw) {
        let time = parse_hhmm(caps.name("time").expect("time capture exists").as_str())?;
        let zone = caps.name("zone").map(|m| m.as_str()).unwrap_or(default_zone);
        let _ = parse_zone(zone)?;
        let weekday = caps.name("weekday").expect("weekday capture exists").as_str();
        let count = caps
            .name("count")
            .map(|m| m.as_str().parse::<usize>())
            .transpose()
            .map_err(|e| EngineError::InvalidRequest(format!("invalid count: {e}")))?
            .unwrap_or(8);

        let start_day = reference
            .date()
            .checked_add_days(chrono::Days::new(1))
            .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?;
        let start_local = start_day.and_time(time);

        return Ok(json!({
            "kind": "recurrence",
            "source": raw,
            "zone": zone,
            "start_local": start_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "normalized_request": {
                "op": "recurrence_preview",
                "start_local": start_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "zone": zone,
                "rule": {
                    "frequency": "weekly",
                    "interval": 1,
                    "count": count,
                    "by_weekdays": [weekday.to_lowercase()]
                },
                "business_calendar": {
                    "exclude_weekends": false,
                    "holidays": []
                },
                "disambiguation": "compatible"
            }
        }));
    }

    Err(EngineError::UnsupportedIntent)
}

fn op_diff_instants(start: &str, end: &str, zone: &str) -> Result<Value, EngineError> {
    let start_utc = parse_instant_str(start)?;
    let end_utc = parse_instant_str(end)?;
    let tz = parse_zone(zone)?;

    let start_local = start_utc.with_timezone(&tz).naive_local();
    let end_local = end_utc.with_timezone(&tz).naive_local();

    let total_seconds = end_utc.timestamp() - start_utc.timestamp();
    let negative = end_local < start_local;
    let (from, to) = if negative {
        (end_local, start_local)
    } else {
        (start_local, end_local)
    };

    // Greedy month subtraction with end-of-month clamping
    let mut years = to.year() - from.year();
    let mut months = to.month() as i32 - from.month() as i32;

    let mut total_months = years * 12 + months;
    let after_months = add_months_clamped(from, total_months)
        .unwrap_or(from);
    if after_months > to {
        total_months -= 1;
    }
    years = total_months / 12;
    months = total_months % 12;

    let after_months = add_months_clamped(from, total_months)
        .unwrap_or(from);
    let remaining = to - after_months;
    let days = remaining.num_days();
    let leftover_secs = remaining.num_seconds() - days * 86400;
    let hours = leftover_secs / 3600;
    let mins = (leftover_secs % 3600) / 60;
    let secs = leftover_secs % 60;

    let sign = if negative { -1i64 } else { 1 };

    Ok(json!({
        "years": sign * years as i64,
        "months": sign * months as i64,
        "days": sign * days,
        "hours": sign * hours,
        "minutes": sign * mins,
        "seconds": sign * secs,
        "total_seconds": total_seconds,
    }))
}

fn op_compare_instants(a: &str, b: &str) -> Result<Value, EngineError> {
    let a_utc = parse_instant_str(a)?;
    let b_utc = parse_instant_str(b)?;

    let result = if a_utc < b_utc {
        -1
    } else if a_utc > b_utc {
        1
    } else {
        0
    };

    Ok(json!({
        "result": result,
    }))
}

fn op_snap_to(
    instant: &str,
    zone: &str,
    unit: SnapUnit,
    edge: SnapEdge,
    week_starts_on: &str,
) -> Result<Value, EngineError> {
    let utc = parse_instant_str(instant)?;
    let tz = parse_zone(zone)?;
    let local = utc.with_timezone(&tz).naive_local();

    let snapped = match (unit, edge) {
        (SnapUnit::Hour, SnapEdge::Start) => {
            local.date().and_hms_opt(local.hour(), 0, 0).unwrap()
        }
        (SnapUnit::Hour, SnapEdge::End) => {
            local.date().and_hms_opt(local.hour(), 59, 59).unwrap()
        }
        (SnapUnit::Day, SnapEdge::Start) => {
            local.date().and_hms_opt(0, 0, 0).unwrap()
        }
        (SnapUnit::Day, SnapEdge::End) => {
            local.date().and_hms_opt(23, 59, 59).unwrap()
        }
        (SnapUnit::Week, SnapEdge::Start) => {
            let wk_start = parse_weekday(week_starts_on)?;
            let mut d = local.date();
            while d.weekday() != wk_start {
                d = d.pred_opt().ok_or_else(|| EngineError::InvalidDateTime("date underflow".to_string()))?;
            }
            d.and_hms_opt(0, 0, 0).unwrap()
        }
        (SnapUnit::Week, SnapEdge::End) => {
            let wk_start = parse_weekday(week_starts_on)?;
            let mut d = local.date();
            while d.weekday() != wk_start {
                d = d.pred_opt().ok_or_else(|| EngineError::InvalidDateTime("date underflow".to_string()))?;
            }
            // End of week = start + 6 days, at 23:59:59
            d = d.checked_add_days(chrono::Days::new(6))
                .ok_or_else(|| EngineError::InvalidDateTime("date overflow".to_string()))?;
            d.and_hms_opt(23, 59, 59).unwrap()
        }
        (SnapUnit::Month, SnapEdge::Start) => {
            NaiveDate::from_ymd_opt(local.year(), local.month(), 1)
                .ok_or_else(|| EngineError::InvalidDateTime("invalid month start".to_string()))?
                .and_hms_opt(0, 0, 0).unwrap()
        }
        (SnapUnit::Month, SnapEdge::End) => {
            let last = days_in_month(local.year(), local.month());
            NaiveDate::from_ymd_opt(local.year(), local.month(), last)
                .ok_or_else(|| EngineError::InvalidDateTime("invalid month end".to_string()))?
                .and_hms_opt(23, 59, 59).unwrap()
        }
        (SnapUnit::Quarter, SnapEdge::Start) => {
            let q_month = ((local.month() - 1) / 3) * 3 + 1;
            NaiveDate::from_ymd_opt(local.year(), q_month, 1)
                .ok_or_else(|| EngineError::InvalidDateTime("invalid quarter start".to_string()))?
                .and_hms_opt(0, 0, 0).unwrap()
        }
        (SnapUnit::Quarter, SnapEdge::End) => {
            let q_end_month = ((local.month() - 1) / 3) * 3 + 3;
            let last = days_in_month(local.year(), q_end_month);
            NaiveDate::from_ymd_opt(local.year(), q_end_month, last)
                .ok_or_else(|| EngineError::InvalidDateTime("invalid quarter end".to_string()))?
                .and_hms_opt(23, 59, 59).unwrap()
        }
        (SnapUnit::Year, SnapEdge::Start) => {
            NaiveDate::from_ymd_opt(local.year(), 1, 1)
                .ok_or_else(|| EngineError::InvalidDateTime("invalid year start".to_string()))?
                .and_hms_opt(0, 0, 0).unwrap()
        }
        (SnapUnit::Year, SnapEdge::End) => {
            NaiveDate::from_ymd_opt(local.year(), 12, 31)
                .ok_or_else(|| EngineError::InvalidDateTime("invalid year end".to_string()))?
                .and_hms_opt(23, 59, 59).unwrap()
        }
    };

    let (resolved, _) = resolve_local_datetime(snapped, tz, Disambiguation::Compatible)?;
    let result_utc = resolved.with_timezone(&Utc);

    Ok(json!({
        "instant": result_utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "local": snapped.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "zoned": fmt_zoned(resolved),
        "offset_seconds": resolved.offset().fix().local_minus_utc(),
    }))
}

fn op_parse_duration(input: &str) -> Result<Value, EngineError> {
    // Support negative durations: a leading `-` negates all components.
    let (negative, rest) = if let Some(stripped) = input.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, input)
    };

    let re = Regex::new(r"^P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$")
        .map_err(|e| EngineError::InvalidRequest(e.to_string()))?;

    let caps = re.captures(rest)
        .ok_or_else(|| EngineError::InvalidDuration(format!("invalid ISO 8601 duration: {input}")))?;

    // Ensure not just "P" or "PT" with nothing
    let has_any = caps.get(1).is_some()
        || caps.get(2).is_some()
        || caps.get(3).is_some()
        || caps.get(4).is_some()
        || caps.get(5).is_some()
        || caps.get(6).is_some()
        || caps.get(7).is_some();

    if !has_any {
        return Err(EngineError::InvalidDuration(format!("empty duration: {input}")));
    }

    let parse_group = |idx: usize| -> i64 {
        caps.get(idx).map(|m| m.as_str().parse::<i64>().unwrap_or(0)).unwrap_or(0)
    };

    let sign: i64 = if negative { -1 } else { 1 };

    let years = (parse_group(1) * sign) as i32;
    let months = (parse_group(2) * sign) as i32;
    let weeks = parse_group(3) * sign;
    let days = parse_group(4) * sign;
    let hours = parse_group(5) * sign;
    let minutes = parse_group(6) * sign;
    let seconds = parse_group(7) * sign;

    Ok(json!({
        "years": years,
        "months": months,
        "weeks": weeks,
        "days": days,
        "hours": hours,
        "minutes": minutes,
        "seconds": seconds,
    }))
}

fn op_format_duration(duration: &DurationSpec) -> Result<Value, EngineError> {
    // Detect negative duration: if any field is negative, output with leading `-`
    // and use absolute values for all fields.
    let is_negative = duration.years < 0
        || duration.months < 0
        || duration.weeks < 0
        || duration.days < 0
        || duration.hours < 0
        || duration.minutes < 0
        || duration.seconds < 0;

    let abs_years = duration.years.unsigned_abs();
    let abs_months = duration.months.unsigned_abs();
    let abs_total_days = (duration.days.abs() + duration.weeks.abs() * 7) as u64;
    let abs_hours = duration.hours.unsigned_abs();
    let abs_minutes = duration.minutes.unsigned_abs();
    let abs_seconds = duration.seconds.unsigned_abs();

    let date_part = format!(
        "{}Y{}M{}D",
        abs_years, abs_months, abs_total_days
    );
    let time_part = format!(
        "{}H{}M{}S",
        abs_hours, abs_minutes, abs_seconds
    );

    let prefix = if is_negative { "-" } else { "" };
    let formatted = format!("{prefix}P{date_part}T{time_part}");

    Ok(json!({
        "formatted": formatted,
    }))
}

fn op_interval_check(
    interval_a: &TimeInterval,
    interval_b: &TimeInterval,
    mode: IntervalCheckMode,
) -> Result<Value, EngineError> {
    let a_start = parse_instant_str(&interval_a.start)?;
    let a_end = parse_instant_str(&interval_a.end)?;
    let b_start = parse_instant_str(&interval_b.start)?;
    let b_end = parse_instant_str(&interval_b.end)?;

    // Normalize: swap if start > end
    let (a_s, a_e) = if a_start <= a_end { (a_start, a_end) } else { (a_end, a_start) };
    let (b_s, b_e) = if b_start <= b_end { (b_start, b_end) } else { (b_end, b_start) };

    match mode {
        IntervalCheckMode::Overlap => {
            // Strict: max(starts) < min(ends). Touching is NOT overlapping.
            let overlap = a_s.max(b_s) < a_e.min(b_e);
            Ok(json!({
                "result": overlap,
                "mode": "overlap",
            }))
        }
        IntervalCheckMode::Contains => {
            // a contains b: a.start <= b.start && b.end <= a.end
            let contains = a_s <= b_s && b_e <= a_e;
            Ok(json!({
                "result": contains,
                "mode": "contains",
            }))
        }
        IntervalCheckMode::Gap => {
            // Gap exists when no overlap: max(starts) >= min(ends)
            let max_start = a_s.max(b_s);
            let min_end = a_e.min(b_e);
            if max_start >= min_end {
                let gap_start = a_e.min(b_e);
                let gap_end = a_s.max(b_s);
                let gap_seconds = (gap_end - gap_start).num_seconds();
                Ok(json!({
                    "result": true,
                    "mode": "gap",
                    "gap": {
                        "start": gap_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        "end": gap_end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        "seconds": gap_seconds,
                    },
                }))
            } else {
                Ok(json!({
                    "result": false,
                    "mode": "gap",
                    "gap": null,
                }))
            }
        }
    }
}

fn op_zone_info(zone: &str, at: Option<&str>) -> Result<Value, EngineError> {
    let tz = parse_zone(zone)?;
    let at_utc = match at {
        Some(s) => parse_instant_str(s)?,
        None => Utc::now(),
    };

    let local = at_utc.with_timezone(&tz);
    let offset_secs = local.offset().fix().local_minus_utc();
    let abbreviation = local.format("%Z").to_string();

    // Detect DST by comparing to January offset
    let jan_dt = NaiveDate::from_ymd_opt(local.year(), 1, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let jan_utc = Utc.from_utc_datetime(&jan_dt);
    let jan_local = jan_utc.with_timezone(&tz);
    let jan_offset = jan_local.offset().fix().local_minus_utc();
    let is_dst = offset_secs != jan_offset && offset_secs > jan_offset;

    // Find next transition: step 6h from `at`, up to 1 year
    let step = Duration::hours(6);
    let limit = at_utc + Duration::days(366);
    let mut prev = at_utc;
    let mut prev_offset = offset_secs;
    let mut t = at_utc + step;
    let mut next_transition: Option<Value> = None;

    while t <= limit {
        let t_local = t.with_timezone(&tz);
        let t_offset = t_local.offset().fix().local_minus_utc();
        if t_offset != prev_offset {
            // Binary search for exact transition
            let mut lo = prev;
            let mut hi = t;
            while (hi - lo).num_seconds() > 60 {
                let mid = lo + (hi - lo) / 2;
                let mid_offset = mid.with_timezone(&tz).offset().fix().local_minus_utc();
                if mid_offset == prev_offset {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            next_transition = Some(json!({
                "at": hi.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "offset_before": prev_offset,
                "offset_after": t_offset,
            }));
            break;
        }
        prev = t;
        prev_offset = t_offset;
        t = t + step;
    }

    Ok(json!({
        "zone": zone,
        "at": at_utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "offset_seconds": offset_secs,
        "is_dst": is_dst,
        "abbreviation": abbreviation,
        "next_transition": next_transition,
    }))
}

fn op_list_zones(region_filter: Option<&str>) -> Result<Value, EngineError> {
    let mut zones: Vec<&str> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|tz| tz.name())
        .collect();

    if let Some(filter) = region_filter {
        zones.retain(|name| name.starts_with(filter));
    }

    zones.sort();

    Ok(json!({
        "zones": zones,
        "count": zones.len(),
    }))
}

fn op_now(zone: Option<&str>) -> Result<Value, EngineError> {
    let now = Utc::now();

    if let Some(zone_name) = zone {
        let tz = parse_zone(zone_name)?;
        let local = now.with_timezone(&tz);
        Ok(json!({
            "instant": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "epoch_seconds": now.timestamp(),
            "zone": zone_name,
            "local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "zoned": fmt_zoned(local),
            "offset_seconds": local.offset().fix().local_minus_utc(),
        }))
    } else {
        Ok(json!({
            "instant": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "epoch_seconds": now.timestamp(),
        }))
    }
}

fn one_off_intent(source: &str, zone: &str, local: NaiveDateTime) -> Value {
    json!({
        "kind": "one_off",
        "source": source,
        "zone": zone,
        "local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "normalized_request": {
            "op": "resolve_local",
            "local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "zone": zone,
            "disambiguation": "compatible"
        }
    })
}

fn maybe_push_occurrence(
    out: &mut Vec<Value>,
    candidate: NaiveDateTime,
    tz: Tz,
    weekday_filter: &Option<HashSet<Weekday>>,
    calendar: &BusinessCalendar,
    holidays: &HashSet<NaiveDate>,
    disambiguation: Disambiguation,
) -> Result<(), EngineError> {
    if let Some(filter) = weekday_filter {
        if !filter.contains(&candidate.date().weekday()) {
            return Ok(());
        }
    }

    if calendar.exclude_weekends {
        let wd = candidate.date().weekday();
        if wd == Weekday::Sat || wd == Weekday::Sun {
            return Ok(());
        }
    }

    if holidays.contains(&candidate.date()) {
        return Ok(());
    }

    let (resolved, applied) = resolve_local_datetime(candidate, tz, disambiguation)?;
    out.push(json!({
        "local": candidate.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "resolved_local": resolved.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "zoned": fmt_zoned(resolved),
        "instant": resolved.with_timezone(&Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "offset_seconds": resolved.offset().fix().local_minus_utc(),
        "disambiguation_applied": applied,
    }));
    Ok(())
}

fn parse_holidays(input: &[String]) -> Result<HashSet<NaiveDate>, EngineError> {
    let mut out = HashSet::new();
    for h in input {
        let date = NaiveDate::parse_from_str(h, "%Y-%m-%d")
            .map_err(|e| EngineError::InvalidDateTime(format!("invalid holiday {h}: {e}")))?;
        out.insert(date);
    }
    Ok(out)
}

fn parse_instant_str(input: &str) -> Result<DateTime<Utc>, EngineError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt.with_timezone(&Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| EngineError::InvalidDateTime("invalid midnight".to_string()))?;
        return Ok(Utc.from_utc_datetime(&naive));
    }

    if let Ok(naive) = parse_local_datetime(input) {
        return Ok(Utc.from_utc_datetime(&naive));
    }

    Err(EngineError::InvalidDateTime(format!(
        "unable to parse instant: {input}"
    )))
}

fn parse_local_datetime(input: &str) -> Result<NaiveDateTime, EngineError> {
    let formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ];

    for fmt in formats {
        if fmt == "%Y-%m-%d" {
            if let Ok(date) = NaiveDate::parse_from_str(input, fmt) {
                return date.and_hms_opt(0, 0, 0).ok_or_else(|| {
                    EngineError::InvalidDateTime(format!("failed to parse local datetime: {input}"))
                });
            }
        } else if let Ok(dt) = NaiveDateTime::parse_from_str(input, fmt) {
            return Ok(dt);
        }
    }

    Err(EngineError::InvalidDateTime(format!(
        "failed to parse local datetime: {input}"
    )))
}

fn parse_zone(zone: &str) -> Result<Tz, EngineError> {
    zone.parse::<Tz>()
        .map_err(|_| EngineError::InvalidZone(zone.to_string()))
}

fn resolve_local_datetime(
    naive: NaiveDateTime,
    tz: Tz,
    disambiguation: Disambiguation,
) -> Result<(DateTime<Tz>, &'static str), EngineError> {
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Ok((dt, "exact")),
        LocalResult::Ambiguous(a, b) => match disambiguation {
            Disambiguation::Compatible | Disambiguation::Earlier => Ok((a, "earlier")),
            Disambiguation::Later => Ok((b, "later")),
            Disambiguation::Reject => Err(EngineError::AmbiguousLocalTime(naive.to_string())),
        },
        LocalResult::None => match disambiguation {
            Disambiguation::Reject => Err(EngineError::NonExistentLocalTime(naive.to_string())),
            Disambiguation::Compatible | Disambiguation::Later => {
                let dt = shift_to_valid(naive, tz, true)?;
                Ok((dt, "shift_forward"))
            }
            Disambiguation::Earlier => {
                let dt = shift_to_valid(naive, tz, false)?;
                Ok((dt, "shift_backward"))
            }
        },
    }
}

fn shift_to_valid(
    naive: NaiveDateTime,
    tz: Tz,
    forward: bool,
) -> Result<DateTime<Tz>, EngineError> {
    let step = if forward { 1 } else { -1 };
    for minute in 1..=720i64 {
        let candidate = naive + Duration::minutes(step * minute);
        match tz.from_local_datetime(&candidate) {
            LocalResult::Single(dt) => return Ok(dt),
            LocalResult::Ambiguous(a, b) => {
                return Ok(if forward { b } else { a });
            }
            LocalResult::None => {}
        }
    }

    Err(EngineError::NonExistentLocalTime(naive.to_string()))
}

fn fmt_zoned(dt: DateTime<Tz>) -> String {
    let body = dt.format("%Y-%m-%dT%H:%M:%S").to_string();
    let offset = format_offset(dt.offset().fix().local_minus_utc(), false);
    format!("{body}{offset}")
}

fn format_zoned(dt: DateTime<Tz>, format: &str) -> Result<String, EngineError> {
    let offset = dt.offset().fix().local_minus_utc();

    match format {
        "extended" => Ok(format!(
            "{}{}",
            dt.format("%Y-%m-%dT%H:%M:%S"),
            format_offset(offset, false)
        )),
        "basic" => Ok(format!(
            "{}{}",
            dt.format("%Y%m%dT%H%M%S"),
            format_offset(offset, true)
        )),
        "date" => Ok(dt.format("%Y-%m-%d").to_string()),
        "time" => Ok(format!(
            "{}{}",
            dt.format("%H:%M:%S"),
            format_offset(offset, false)
        )),
        other => Err(EngineError::InvalidRequest(format!(
            "unknown format {other}, expected extended|basic|date|time"
        ))),
    }
}

fn format_offset(seconds: i32, basic: bool) -> String {
    if seconds == 0 {
        return "Z".to_string();
    }

    let sign = if seconds >= 0 { '+' } else { '-' };
    let abs = seconds.abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;

    if basic {
        format!("{sign}{hours:02}{minutes:02}")
    } else {
        format!("{sign}{hours:02}:{minutes:02}")
    }
}

fn parse_weekday(value: &str) -> Result<Weekday, EngineError> {
    match value.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        other => Err(EngineError::InvalidRequest(format!(
            "unknown weekday: {other}"
        ))),
    }
}

fn parse_hhmm(value: &str) -> Result<NaiveTime, EngineError> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 2 {
        return Err(EngineError::InvalidRequest(format!(
            "time must be HH:MM: {value}"
        )));
    }
    let hour: u32 = parts[0]
        .parse()
        .map_err(|e| EngineError::InvalidRequest(format!("invalid hour: {e}")))?;
    let minute: u32 = parts[1]
        .parse()
        .map_err(|e| EngineError::InvalidRequest(format!("invalid minute: {e}")))?;
    NaiveTime::from_hms_opt(hour, minute, 0)
        .ok_or_else(|| EngineError::InvalidRequest(format!("invalid HH:MM: {value}")))
}

fn add_months_clamped(dt: NaiveDateTime, delta_months: i32) -> Result<NaiveDateTime, EngineError> {
    let year = dt.year();
    let month0 = dt.month0() as i32;
    let total = year.saturating_mul(12) + month0 + delta_months;

    let new_year = total.div_euclid(12);
    let new_month0 = total.rem_euclid(12);

    let last_day = days_in_month(new_year, (new_month0 + 1) as u32);
    let new_day = dt.day().min(last_day);

    let date = NaiveDate::from_ymd_opt(new_year, (new_month0 + 1) as u32, new_day)
        .ok_or_else(|| EngineError::InvalidDateTime("month arithmetic overflow".to_string()))?;

    date.and_hms_opt(dt.hour(), dt.minute(), dt.second())
        .ok_or_else(|| EngineError::InvalidDateTime("invalid resulting time".to_string()))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid first day");
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid next first day");
    (next - first).num_days() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_gap_with_compatible_policy() {
        let req = Request::ResolveLocal {
            local: "2023-03-12T02:00:00".to_string(),
            zone: "America/New_York".to_string(),
            disambiguation: Disambiguation::Compatible,
        };
        let response = evaluate_request(req);
        assert!(response.ok);
        let zoned = response
            .value
            .as_ref()
            .and_then(|v| v.get("zoned"))
            .and_then(Value::as_str)
            .expect("zoned string");
        assert_eq!(zoned, "2023-03-12T03:00:00-04:00");
    }

    #[test]
    fn resolves_fold_with_later_policy() {
        let req = Request::ResolveLocal {
            local: "2023-11-05T01:30:00".to_string(),
            zone: "America/New_York".to_string(),
            disambiguation: Disambiguation::Later,
        };
        let response = evaluate_request(req);
        assert!(response.ok);
        let zoned = response
            .value
            .as_ref()
            .and_then(|v| v.get("zoned"))
            .and_then(Value::as_str)
            .expect("zoned string");
        assert_eq!(zoned, "2023-11-05T01:30:00-05:00");
    }

    #[test]
    fn parses_and_formats_rfc3339() {
        let parsed = evaluate_request(Request::ParseInstant {
            input: "2024-01-05T12:34:56+02:00".to_string(),
        });
        assert!(parsed.ok);

        let formatted = evaluate_request(Request::FormatInstant {
            instant: "2024-01-05T10:34:56Z".to_string(),
            zone: "Asia/Singapore".to_string(),
            format: "extended".to_string(),
        });
        assert!(formatted.ok);
        let value = formatted
            .value
            .as_ref()
            .and_then(|v| v.get("formatted"))
            .and_then(Value::as_str)
            .expect("formatted value");
        assert_eq!(value, "2024-01-05T18:34:56+08:00");
    }

    #[test]
    fn diff_instants_same_month() {
        let resp = evaluate_request_value(json!({
            "op": "diff_instants",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-20T14:30:00Z",
            "zone": "UTC",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["days"], 5);
        assert_eq!(v["hours"], 4);
        assert_eq!(v["minutes"], 30);
    }

    #[test]
    fn diff_instants_month_clamping() {
        // Jan 31 -> Feb 29 (leap year) = 0 months, 29 days in non-leap; but 2024 is leap
        // add_months_clamped(Jan 31, 1) = Feb 29, so Jan 31 -> Feb 29 = 1 month
        let resp = evaluate_request_value(json!({
            "op": "diff_instants",
            "start": "2024-01-31T00:00:00Z",
            "end": "2024-02-29T00:00:00Z",
            "zone": "UTC",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["months"], 1);
        assert_eq!(v["days"], 0);

        // Non-leap year: Jan 31 -> Feb 28 = 1 month (clamped)
        let resp = evaluate_request_value(json!({
            "op": "diff_instants",
            "start": "2023-01-31T00:00:00Z",
            "end": "2023-02-28T00:00:00Z",
            "zone": "UTC",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["months"], 1);
        assert_eq!(v["days"], 0);
    }

    #[test]
    fn diff_instants_negative() {
        let resp = evaluate_request_value(json!({
            "op": "diff_instants",
            "start": "2024-03-01T00:00:00Z",
            "end": "2024-01-01T00:00:00Z",
            "zone": "UTC",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert!(v["total_seconds"].as_i64().unwrap() < 0);
        assert!(v["months"].as_i64().unwrap() < 0);
    }

    #[test]
    fn compare_instants_ordering() {
        let resp = evaluate_request_value(json!({
            "op": "compare_instants",
            "a": "2024-01-01T00:00:00Z",
            "b": "2024-06-01T00:00:00Z",
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["result"], -1);

        let resp = evaluate_request_value(json!({
            "op": "compare_instants",
            "a": "2024-06-01T00:00:00Z",
            "b": "2024-01-01T00:00:00Z",
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["result"], 1);

        let resp = evaluate_request_value(json!({
            "op": "compare_instants",
            "a": "2024-01-01T00:00:00Z",
            "b": "2024-01-01T00:00:00Z",
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["result"], 0);
    }

    #[test]
    fn snap_to_month_start() {
        let resp = evaluate_request_value(json!({
            "op": "snap_to",
            "instant": "2024-03-15T14:30:00Z",
            "zone": "UTC",
            "unit": "month",
            "edge": "start",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["local"], "2024-03-01T00:00:00");
    }

    #[test]
    fn snap_to_year_end() {
        let resp = evaluate_request_value(json!({
            "op": "snap_to",
            "instant": "2024-06-15T10:00:00Z",
            "zone": "UTC",
            "unit": "year",
            "edge": "end",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["local"], "2024-12-31T23:59:59");
    }

    #[test]
    fn parse_duration_iso() {
        let resp = evaluate_request_value(json!({
            "op": "parse_duration",
            "input": "P1Y2M3DT4H5M6S",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["years"], 1);
        assert_eq!(v["months"], 2);
        assert_eq!(v["days"], 3);
        assert_eq!(v["hours"], 4);
        assert_eq!(v["minutes"], 5);
        assert_eq!(v["seconds"], 6);
    }

    #[test]
    fn parse_duration_month_vs_minute() {
        // P1M = 1 month, PT1M = 1 minute
        let resp = evaluate_request_value(json!({
            "op": "parse_duration",
            "input": "P1M",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["months"], 1);
        assert_eq!(v["minutes"], 0);

        let resp = evaluate_request_value(json!({
            "op": "parse_duration",
            "input": "PT1M",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["months"], 0);
        assert_eq!(v["minutes"], 1);
    }

    #[test]
    fn format_duration_round_trip() {
        let resp = evaluate_request_value(json!({
            "op": "format_duration",
            "duration": {"years": 1, "months": 2, "days": 3, "hours": 4, "minutes": 5, "seconds": 6},
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["formatted"], "P1Y2M3DT4H5M6S");
    }

    #[test]
    fn parse_negative_duration() {
        let resp = evaluate_request_value(json!({
            "op": "parse_duration",
            "input": "-P1DT2H",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["days"], -1);
        assert_eq!(v["hours"], -2);
        assert_eq!(v["years"], 0);
        assert_eq!(v["months"], 0);
        assert_eq!(v["minutes"], 0);
        assert_eq!(v["seconds"], 0);
    }

    #[test]
    fn parse_negative_duration_full() {
        let resp = evaluate_request_value(json!({
            "op": "parse_duration",
            "input": "-P1Y2M3DT4H5M6S",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["years"], -1);
        assert_eq!(v["months"], -2);
        assert_eq!(v["days"], -3);
        assert_eq!(v["hours"], -4);
        assert_eq!(v["minutes"], -5);
        assert_eq!(v["seconds"], -6);
    }

    #[test]
    fn format_negative_duration() {
        let resp = evaluate_request_value(json!({
            "op": "format_duration",
            "duration": {"years": -1, "months": -2, "days": -3, "hours": -4, "minutes": -5, "seconds": -6},
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["formatted"], "-P1Y2M3DT4H5M6S");
    }

    #[test]
    fn negative_duration_round_trip() {
        // Parse a negative duration, then format it, and verify it round-trips.
        let parse_resp = evaluate_request_value(json!({
            "op": "parse_duration",
            "input": "-P1Y2M3DT4H5M6S",
        }));
        assert!(parse_resp.ok);
        let parsed = parse_resp.value.unwrap();

        let format_resp = evaluate_request_value(json!({
            "op": "format_duration",
            "duration": parsed,
        }));
        assert!(format_resp.ok);
        assert_eq!(format_resp.value.unwrap()["formatted"], "-P1Y2M3DT4H5M6S");
    }

    #[test]
    fn add_negative_duration_absolute() {
        // Subtracting 1 hour from 2024-01-01T12:00:00Z should yield 11:00:00
        let resp = evaluate_request_value(json!({
            "op": "add_duration",
            "start": "2024-01-01T12:00:00Z",
            "zone": "UTC",
            "duration": {"hours": -1},
            "arithmetic": "absolute",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["result"]["instant"], "2024-01-01T11:00:00Z");
    }

    #[test]
    fn interval_overlap_strict() {
        // Touching endpoints are NOT overlapping
        let resp = evaluate_request_value(json!({
            "op": "interval_check",
            "interval_a": {"start": "2024-01-01T00:00:00Z", "end": "2024-01-10T00:00:00Z"},
            "interval_b": {"start": "2024-01-10T00:00:00Z", "end": "2024-01-20T00:00:00Z"},
            "mode": "overlap",
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["result"], false);

        // True overlap
        let resp = evaluate_request_value(json!({
            "op": "interval_check",
            "interval_a": {"start": "2024-01-01T00:00:00Z", "end": "2024-01-10T00:00:00Z"},
            "interval_b": {"start": "2024-01-05T00:00:00Z", "end": "2024-01-20T00:00:00Z"},
            "mode": "overlap",
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["result"], true);
    }

    #[test]
    fn interval_contains() {
        let resp = evaluate_request_value(json!({
            "op": "interval_check",
            "interval_a": {"start": "2024-01-01T00:00:00Z", "end": "2024-01-31T00:00:00Z"},
            "interval_b": {"start": "2024-01-05T00:00:00Z", "end": "2024-01-20T00:00:00Z"},
            "mode": "contains",
        }));
        assert!(resp.ok);
        assert_eq!(resp.value.unwrap()["result"], true);
    }

    #[test]
    fn zone_info_utc() {
        let resp = evaluate_request_value(json!({
            "op": "zone_info",
            "zone": "UTC",
            "at": "2024-06-15T12:00:00Z",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["offset_seconds"], 0);
        assert_eq!(v["is_dst"], false);
        assert!(v["next_transition"].is_null());
    }

    #[test]
    fn zone_info_dst_zone() {
        let resp = evaluate_request_value(json!({
            "op": "zone_info",
            "zone": "America/New_York",
            "at": "2024-06-15T12:00:00Z",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert_eq!(v["offset_seconds"], -14400);
        assert_eq!(v["is_dst"], true);
        assert!(!v["next_transition"].is_null());
    }

    #[test]
    fn list_zones_all() {
        let resp = evaluate_request_value(json!({
            "op": "list_zones",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert!(v["count"].as_u64().unwrap() > 400);
    }

    #[test]
    fn list_zones_filtered() {
        let resp = evaluate_request_value(json!({
            "op": "list_zones",
            "region_filter": "America/New",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        let zones = v["zones"].as_array().unwrap();
        assert!(zones.iter().all(|z| z.as_str().unwrap().starts_with("America/New")));
    }

    #[test]
    fn now_returns_instant() {
        let resp = evaluate_request_value(json!({
            "op": "now",
        }));
        assert!(resp.ok);
        let v = resp.value.unwrap();
        assert!(v["instant"].as_str().is_some());
        assert!(v["epoch_seconds"].as_i64().is_some());
    }

    #[test]
    fn normalizes_supported_intent() {
        let response = evaluate_request(Request::NormalizeIntent {
            input: "tomorrow at 09:30 in America/Los_Angeles".to_string(),
            reference_local: "2026-01-10T12:00:00".to_string(),
            default_zone: "UTC".to_string(),
        });
        assert!(response.ok);
        let kind = response
            .value
            .as_ref()
            .and_then(|v| v.get("kind"))
            .and_then(Value::as_str)
            .expect("kind");
        assert_eq!(kind, "one_off");
    }
}
