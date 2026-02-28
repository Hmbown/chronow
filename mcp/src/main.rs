use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        AnnotateAble, Annotated, Implementation, ListResourcesResult, PaginatedRequestParams,
        RawResource, ReadResourceRequestParams, ReadResourceResult, ResourceContents,
        ResourcesCapability, ServerCapabilities, ServerInfo, ToolsCapability,
    },
    schemars,
    service::RequestContext,
    service::RoleServer,
    tool, tool_handler, tool_router,
    transport::io::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ============================================================================ //
// Timezone detection
// ============================================================================ //

/// Detect the user's timezone with fallback chain:
/// 1. `CHRONOW_DEFAULT_ZONE` env var (explicit user config)
/// 2. System timezone via `iana_time_zone` crate
/// 3. `TZ` env var
/// 4. "UTC" as final fallback
fn detect_default_zone() -> String {
    // 1. Explicit env var (highest priority — user configured this)
    if let Ok(zone) = std::env::var("CHRONOW_DEFAULT_ZONE") && !zone.trim().is_empty() {
        return zone.trim().to_string();
    }

    // 2. System timezone (OS-level detection)
    if let Ok(zone) = iana_time_zone::get_timezone() && !zone.trim().is_empty() {
        return zone;
    }

    // 3. TZ env var
    if let Ok(zone) = std::env::var("TZ") && !zone.trim().is_empty() {
        return zone.trim().to_string();
    }

    // 4. Final fallback
    "UTC".to_string()
}

// ============================================================================ //
// MCP Resources: prompt templates for common temporal workflows
// ============================================================================ //

mod resources {
    /// URI scheme for all Chronow resources.
    pub const DISAMBIGUATION_URI: &str = "chronow://policies/disambiguation";
    pub const TIMEZONE_CONVERSION_URI: &str = "chronow://workflows/timezone-conversion";
    pub const SCHEDULE_MEETING_URI: &str = "chronow://workflows/schedule-meeting";
    pub const NEXT_BUSINESS_DAY_URI: &str = "chronow://workflows/next-business-day";

    pub const DISAMBIGUATION_CONTENT: &str = "\
Chronow Disambiguation Policies
================================

When converting a local datetime to a UTC instant, the local time may be
ambiguous (it occurs twice during a DST fall-back) or may not exist at all
(it is skipped during a DST spring-forward). The `disambiguation` parameter
on `resolve_local` and `add_duration` controls how these cases are handled.

Policies
--------

1. compatible (default)
   - Ambiguous: picks the offset that was in effect *before* the transition
     (i.e. the first occurrence / summer-time offset during fall-back).
   - Skipped: picks the offset that will be in effect *after* the transition
     (i.e. the clock jumps forward, so the time is moved forward).
   - Use when: you want the most intuitive behavior for end users and do not
     need to distinguish between the two possible instants.

2. earlier
   - Ambiguous: always selects the *earlier* UTC instant (the first
     occurrence).
   - Skipped: selects the *earlier* valid instant (the moment just before the
     gap, which is the last instant at the old offset).
   - Use when: you need the earliest possible interpretation, e.g. scheduling
     a deadline where \"no later than\" semantics matter.

3. later
   - Ambiguous: always selects the *later* UTC instant (the second
     occurrence).
   - Skipped: selects the *later* valid instant (the moment the clock jumps
     to, which is the first instant at the new offset).
   - Use when: you prefer the most recent interpretation, e.g. a billing
     cut-off where you want maximum elapsed time.

4. reject
   - Ambiguous: returns an error instead of silently choosing.
   - Skipped: returns an error instead of silently adjusting.
   - Use when: your application requires the user to explicitly handle DST
     conflicts (e.g. a calendar UI that should prompt the user).

Example
-------
Local time 2024-11-03T01:30:00 in America/New_York is ambiguous (fall-back).

  compatible -> 2024-11-03T05:30:00Z (EDT, UTC-4, the first occurrence)
  earlier    -> 2024-11-03T05:30:00Z (EDT, UTC-4)
  later      -> 2024-11-03T06:30:00Z (EST, UTC-5)
  reject     -> error: ambiguous_local_datetime
";

    pub const TIMEZONE_CONVERSION_CONTENT: &str = "\
Workflow: Timezone Conversion
==============================

Convert a datetime from one timezone to another using the Chronow MCP tools.

Steps
-----
1. Resolve the source local datetime to a UTC instant:

   Tool: resolve_local
   Params:
     local: \"<source datetime, e.g. 2024-06-15T14:30:00>\"
     zone:  \"<source IANA zone, e.g. America/New_York>\"
     disambiguation: \"compatible\"

   This gives you the canonical UTC instant.

2. Format the UTC instant in the target timezone:

   Tool: format_instant
   Params:
     instant: \"<UTC instant from step 1, e.g. 2024-06-15T18:30:00Z>\"
     zone:    \"<target IANA zone, e.g. Asia/Tokyo>\"
     format:  \"extended\"

   This gives you the local representation in the target zone.

Example
-------
Convert 2024-06-15 2:30 PM New York time to Tokyo time:

  Step 1: resolve_local(local=\"2024-06-15T14:30:00\", zone=\"America/New_York\")
          -> instant: 2024-06-15T18:30:00Z

  Step 2: format_instant(instant=\"2024-06-15T18:30:00Z\", zone=\"Asia/Tokyo\")
          -> 2024-06-16T03:30:00+09:00

Tips
----
- Always go through UTC. Never try to add/subtract offset hours manually.
- Use zone_info to check whether DST is active if you need offset details.
- Use list_zones with a region_filter to discover valid IANA zone names.
";

    pub const SCHEDULE_MEETING_CONTENT: &str = "\
Workflow: Schedule a Meeting Across Timezones
===============================================

Find a meeting time that works for participants in multiple timezones.

Steps
-----
1. Get the current time to establish a baseline:

   Tool: now
   Params:
     zone: \"<organizer's IANA zone>\"

2. Pick a candidate local time in the organizer's timezone and resolve it:

   Tool: resolve_local
   Params:
     local: \"<candidate datetime, e.g. 2024-06-17T10:00:00>\"
     zone:  \"<organizer's IANA zone>\"
     disambiguation: \"compatible\"

3. For each participant's timezone, format the UTC instant to see their local time:

   Tool: format_instant
   Params:
     instant: \"<UTC instant from step 2>\"
     zone:    \"<participant's IANA zone>\"
     format:  \"extended\"

4. Verify all local times are within acceptable business hours (e.g. 08:00-18:00).
   If not, adjust the candidate time and repeat from step 2.

5. Optionally, compute the duration until the meeting:

   Tool: diff_instants
   Params:
     start: \"<current UTC instant from step 1>\"
     end:   \"<meeting UTC instant from step 2>\"
     zone:  \"<organizer's IANA zone>\"

Example
-------
Schedule a meeting for a team in New York, London, and Tokyo:

  Step 1: now(zone=\"America/New_York\") -> 2024-06-14T16:00:00-04:00

  Step 2: resolve_local(local=\"2024-06-17T09:00:00\", zone=\"America/New_York\")
          -> 2024-06-17T13:00:00Z

  Step 3:
    format_instant(instant=\"2024-06-17T13:00:00Z\", zone=\"America/New_York\")
    -> 2024-06-17T09:00:00-04:00  (9 AM -- good)

    format_instant(instant=\"2024-06-17T13:00:00Z\", zone=\"Europe/London\")
    -> 2024-06-17T14:00:00+01:00  (2 PM -- good)

    format_instant(instant=\"2024-06-17T13:00:00Z\", zone=\"Asia/Tokyo\")
    -> 2024-06-17T22:00:00+09:00  (10 PM -- too late!)

  Adjust: try 2024-06-17T08:00:00 New York -> 12:00Z -> 21:00 Tokyo (still late).
  Try: 2024-06-17T21:00:00 Tokyo = 2024-06-17T12:00:00Z = 08:00 New York, 13:00 London.
  This works if New York can do 8 AM.

Tips
----
- Use zone_info to check DST offsets for each zone at the meeting date.
- Use interval_check to verify no scheduling conflicts with existing meetings.
- For recurring meetings, use recurrence_preview to generate the full series
  and verify each occurrence falls in business hours (DST shifts can move them).
";

    pub const NEXT_BUSINESS_DAY_CONTENT: &str = "\
Workflow: Find the Next Business Day
======================================

Find the next business day from a given date, skipping weekends and holidays.

Steps
-----
1. Get the current time (or start from a known date):

   Tool: now
   Params:
     zone: \"<IANA zone for business calendar>\"

2. Use recurrence_preview with a daily recurrence and business calendar to
   find the next N business days:

   Tool: recurrence_preview
   Params:
     start_local: \"<starting local date/time, e.g. 2024-06-14T09:00:00>\"
     zone: \"<IANA zone>\"
     rule:
       frequency: \"daily\"
       interval: 1
       count: 1           # increase to get more business days ahead
     business_calendar:
       exclude_weekends: true
       holidays:          # list any holidays to skip
         - \"2024-06-19\"   # e.g. Juneteenth
         - \"2024-07-04\"   # e.g. Independence Day
     disambiguation: \"compatible\"

   The first occurrence in the result is the next business day.

3. Optionally, snap to the start of that business day:

   Tool: snap_to
   Params:
     instant: \"<UTC instant of the business day from step 2>\"
     zone: \"<IANA zone>\"
     unit: \"day\"
     edge: \"start\"

Example
-------
Find the next business day after Friday 2024-06-14 in New York:

  recurrence_preview(
    start_local=\"2024-06-15T09:00:00\",   # start from the day after
    zone=\"America/New_York\",
    rule={ frequency: \"daily\", interval: 1, count: 1 },
    business_calendar={ exclude_weekends: true, holidays: [\"2024-06-19\"] }
  )
  -> First occurrence: 2024-06-17T09:00:00 (Monday) -- the next business day.

Tips
----
- Set start_local to the day *after* the reference date if you want \"next\"
  business day (not including the reference date itself).
- Increase `count` to get multiple upcoming business days at once.
- Add country-specific holidays to the holidays list for accurate results.
- Use diff_instants between now and the result to compute how many calendar
  days away the next business day is.
";
}

#[derive(Clone)]
struct ChronowServer {
    tool_router: ToolRouter<Self>,
    default_zone: String,
}

impl ChronowServer {
    /// Resolve an optional zone parameter to a concrete IANA zone name.
    /// Falls back to the server's detected default zone.
    fn resolve_zone(&self, zone: Option<String>) -> String {
        match zone {
            Some(z) if !z.trim().is_empty() => z,
            _ => self.default_zone.clone(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ParseInstantParams {
    /// ISO 8601/RFC 3339 datetime string to parse
    input: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FormatInstantParams {
    /// UTC instant (RFC 3339) to format
    instant: String,
    /// IANA timezone name (e.g. "America/New_York"). Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
    /// Output format: extended, basic, date, or time
    #[serde(default = "default_extended")]
    format: String,
}

fn default_extended() -> String {
    "extended".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ResolveLocalParams {
    /// Local datetime string (YYYY-MM-DDTHH:MM:SS)
    local: String,
    /// IANA timezone name. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
    /// Disambiguation policy: compatible, earlier, later, reject
    #[serde(default = "default_compatible")]
    disambiguation: String,
}

fn default_compatible() -> String {
    "compatible".to_string()
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
struct DurationSpecParams {
    #[serde(default)]
    years: i32,
    #[serde(default)]
    months: i32,
    #[serde(default)]
    weeks: i64,
    #[serde(default)]
    days: i64,
    #[serde(default)]
    hours: i64,
    #[serde(default)]
    minutes: i64,
    #[serde(default)]
    seconds: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddDurationParams {
    /// UTC instant (RFC 3339) to add duration to
    start: String,
    /// IANA timezone name for calendar arithmetic. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
    /// Duration to add
    #[serde(default)]
    duration: DurationSpecParams,
    /// Arithmetic mode: absolute or calendar
    #[serde(default = "default_calendar")]
    arithmetic: String,
    /// Disambiguation policy for DST conflicts
    #[serde(default = "default_compatible")]
    disambiguation: String,
}

fn default_calendar() -> String {
    "calendar".to_string()
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct RecurrenceRuleParams {
    frequency: String,
    #[serde(default = "default_one")]
    interval: u32,
    count: usize,
    #[serde(default)]
    by_weekdays: Vec<String>,
}

fn default_one() -> u32 {
    1
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
struct BusinessCalendarParams {
    #[serde(default)]
    exclude_weekends: bool,
    #[serde(default)]
    holidays: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RecurrencePreviewParams {
    /// Local start datetime
    start_local: String,
    /// IANA timezone name. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
    /// Recurrence rule
    rule: RecurrenceRuleParams,
    /// Business calendar configuration
    #[serde(default)]
    business_calendar: Option<BusinessCalendarParams>,
    /// Disambiguation policy
    #[serde(default = "default_compatible")]
    disambiguation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NormalizeIntentParams {
    /// Natural language input (e.g. "tomorrow at 09:30")
    input: String,
    /// Reference local datetime for relative calculations
    reference_local: String,
    /// Default IANA timezone if not specified in input. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    default_zone: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DiffInstantsParams {
    /// Start instant (RFC 3339)
    start: String,
    /// End instant (RFC 3339)
    end: String,
    /// IANA timezone for calendar diff computation. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CompareInstantsParams {
    /// First instant (RFC 3339)
    a: String,
    /// Second instant (RFC 3339)
    b: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SnapToParams {
    /// UTC instant (RFC 3339) to snap
    instant: String,
    /// IANA timezone name. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
    /// Unit to snap to: day, week, month, quarter, year, hour
    unit: String,
    /// Edge: start or end
    edge: String,
    /// First day of week (default: monday)
    #[serde(default = "default_monday")]
    week_starts_on: String,
}

fn default_monday() -> String {
    "monday".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ParseDurationParams {
    /// ISO 8601 duration string (e.g. "P1Y2M3DT4H5M6S")
    input: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FormatDurationParams {
    /// Duration components to format
    duration: DurationSpecParams,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct IntervalParams {
    /// Start of interval (RFC 3339)
    start: String,
    /// End of interval (RFC 3339)
    end: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IntervalCheckParams {
    /// First time interval
    interval_a: IntervalParams,
    /// Second time interval
    interval_b: IntervalParams,
    /// Check mode: overlap, contains, or gap
    mode: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ZoneInfoParams {
    /// IANA timezone name. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
    /// Optional instant to query at (RFC 3339, defaults to now)
    #[serde(default)]
    at: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListZonesParams {
    /// Optional prefix filter (e.g. "America/")
    #[serde(default)]
    region_filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NowParams {
    /// Optional IANA timezone for local projection. Defaults to the user's detected timezone if omitted.
    #[serde(default)]
    zone: Option<String>,
}

/// Evaluate an engine request and return an MCP-compliant result.
///
/// * Engine success (`ok: true`) -> `Ok(pretty_json)` -> the rmcp framework
///   wraps this in `CallToolResult::success` with `is_error: false`.
/// * Engine error (`ok: false`) -> `Err(human_message)` -> the rmcp framework
///   wraps this in `CallToolResult::error` with `is_error: true`.
/// * Serialization failures are caught -- the server process never panics.
fn eval(request: serde_json::Value) -> Result<String, String> {
    let response = chronow_core::evaluate_request_value(request);
    if response.ok {
        let text = serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("{{\"ok\":true,\"_serialization_note\":\"{}\"}}", e));
        Ok(text)
    } else {
        let msg = match &response.error {
            Some(err) => format!("[{}] {}", err.code, err.message),
            None => "unknown engine error".to_string(),
        };
        Err(msg)
    }
}

/// Validate that a required string field is non-empty.
/// Returns `Err(message)` which the rmcp framework maps to `is_error: true`.
fn require_non_empty(field_name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!(
            "missing required field: `{field_name}` must be a non-empty string"
        ))
    } else {
        Ok(())
    }
}

#[tool_router]
impl ChronowServer {
    /// Parse an ISO 8601/RFC 3339 datetime string into a UTC instant with epoch timestamps.
    #[tool]
    fn parse_instant(
        &self,
        Parameters(p): Parameters<ParseInstantParams>,
    ) -> Result<String, String> {
        require_non_empty("input", &p.input)?;
        eval(json!({"op": "parse_instant", "input": p.input}))
    }

    /// Format a UTC instant in a specific timezone with the given format (extended/basic/date/time).
    #[tool]
    fn format_instant(
        &self,
        Parameters(p): Parameters<FormatInstantParams>,
    ) -> Result<String, String> {
        require_non_empty("instant", &p.instant)?;
        let zone = self.resolve_zone(p.zone);
        eval(
            json!({"op": "format_instant", "instant": p.instant, "zone": zone, "format": p.format}),
        )
    }

    /// Resolve a local datetime in a timezone to a UTC instant, handling DST ambiguity.
    #[tool]
    fn resolve_local(
        &self,
        Parameters(p): Parameters<ResolveLocalParams>,
    ) -> Result<String, String> {
        require_non_empty("local", &p.local)?;
        let zone = self.resolve_zone(p.zone);
        eval(
            json!({"op": "resolve_local", "local": p.local, "zone": zone, "disambiguation": p.disambiguation}),
        )
    }

    /// Add a duration to an instant using absolute or calendar arithmetic.
    #[tool]
    fn add_duration(&self, Parameters(p): Parameters<AddDurationParams>) -> Result<String, String> {
        require_non_empty("start", &p.start)?;
        let zone = self.resolve_zone(p.zone);
        eval(json!({
            "op": "add_duration",
            "start": p.start,
            "zone": zone,
            "duration": p.duration,
            "arithmetic": p.arithmetic,
            "disambiguation": p.disambiguation,
        }))
    }

    /// Generate a series of recurring datetime occurrences with optional business calendar rules.
    #[tool]
    fn recurrence_preview(
        &self,
        Parameters(p): Parameters<RecurrencePreviewParams>,
    ) -> Result<String, String> {
        require_non_empty("start_local", &p.start_local)?;
        let zone = self.resolve_zone(p.zone);
        eval(json!({
            "op": "recurrence_preview",
            "start_local": p.start_local,
            "zone": zone,
            "rule": p.rule,
            "business_calendar": p.business_calendar,
            "disambiguation": p.disambiguation,
        }))
    }

    /// Normalize a natural language temporal intent into a deterministic request.
    #[tool]
    fn normalize_intent(
        &self,
        Parameters(p): Parameters<NormalizeIntentParams>,
    ) -> Result<String, String> {
        require_non_empty("input", &p.input)?;
        require_non_empty("reference_local", &p.reference_local)?;
        let default_zone = self.resolve_zone(p.default_zone);
        eval(json!({
            "op": "normalize_intent",
            "input": p.input,
            "reference_local": p.reference_local,
            "default_zone": default_zone,
        }))
    }

    /// Compute the calendar difference between two instants (years, months, days, hours, minutes, seconds).
    #[tool]
    fn diff_instants(
        &self,
        Parameters(p): Parameters<DiffInstantsParams>,
    ) -> Result<String, String> {
        require_non_empty("start", &p.start)?;
        require_non_empty("end", &p.end)?;
        let zone = self.resolve_zone(p.zone);
        eval(json!({"op": "diff_instants", "start": p.start, "end": p.end, "zone": zone}))
    }

    /// Compare two instants, returning -1, 0, or 1.
    #[tool]
    fn compare_instants(
        &self,
        Parameters(p): Parameters<CompareInstantsParams>,
    ) -> Result<String, String> {
        require_non_empty("a", &p.a)?;
        require_non_empty("b", &p.b)?;
        eval(json!({"op": "compare_instants", "a": p.a, "b": p.b}))
    }

    /// Snap an instant to the start or end of a calendar unit (day/week/month/quarter/year/hour).
    #[tool]
    fn snap_to(&self, Parameters(p): Parameters<SnapToParams>) -> Result<String, String> {
        require_non_empty("instant", &p.instant)?;
        let zone = self.resolve_zone(p.zone);
        require_non_empty("unit", &p.unit)?;
        require_non_empty("edge", &p.edge)?;
        eval(json!({
            "op": "snap_to",
            "instant": p.instant,
            "zone": zone,
            "unit": p.unit,
            "edge": p.edge,
            "week_starts_on": p.week_starts_on,
        }))
    }

    /// Parse an ISO 8601 duration string (e.g. P1Y2M3DT4H5M6S) into components.
    #[tool]
    fn parse_duration(
        &self,
        Parameters(p): Parameters<ParseDurationParams>,
    ) -> Result<String, String> {
        require_non_empty("input", &p.input)?;
        eval(json!({"op": "parse_duration", "input": p.input}))
    }

    /// Format duration components into an ISO 8601 duration string.
    #[tool]
    fn format_duration(
        &self,
        Parameters(p): Parameters<FormatDurationParams>,
    ) -> Result<String, String> {
        eval(json!({"op": "format_duration", "duration": p.duration}))
    }

    /// Check two time intervals for overlap, containment, or gap.
    #[tool]
    fn interval_check(
        &self,
        Parameters(p): Parameters<IntervalCheckParams>,
    ) -> Result<String, String> {
        require_non_empty("interval_a.start", &p.interval_a.start)?;
        require_non_empty("interval_a.end", &p.interval_a.end)?;
        require_non_empty("interval_b.start", &p.interval_b.start)?;
        require_non_empty("interval_b.end", &p.interval_b.end)?;
        require_non_empty("mode", &p.mode)?;
        eval(json!({
            "op": "interval_check",
            "interval_a": p.interval_a,
            "interval_b": p.interval_b,
            "mode": p.mode,
        }))
    }

    /// Get timezone information: offset, DST status, abbreviation, and next transition.
    #[tool]
    fn zone_info(&self, Parameters(p): Parameters<ZoneInfoParams>) -> Result<String, String> {
        let zone = self.resolve_zone(p.zone);
        let mut req = json!({"op": "zone_info", "zone": zone});
        if let Some(ref at) = p.at {
            if at.trim().is_empty() {
                return Err("field `at` must be non-empty when provided".to_string());
            }
            req["at"] = json!(at);
        }
        eval(req)
    }

    /// List available IANA timezone names, optionally filtered by region prefix.
    #[tool]
    fn list_zones(&self, Parameters(p): Parameters<ListZonesParams>) -> Result<String, String> {
        let mut req = json!({"op": "list_zones"});
        if let Some(filter) = p.region_filter {
            req["region_filter"] = json!(filter);
        }
        eval(req)
    }

    /// Get the current time as a UTC instant, optionally projected into a timezone. Defaults to the user's detected timezone.
    #[tool]
    fn now(&self, Parameters(p): Parameters<NowParams>) -> Result<String, String> {
        let zone = self.resolve_zone(p.zone);
        let req = json!({"op": "now", "zone": zone});
        eval(req)
    }
}

/// Build the static list of MCP resources exposed by this server.
fn chronow_resources() -> Vec<Annotated<RawResource>> {
    vec![
        RawResource {
            uri: resources::DISAMBIGUATION_URI.into(),
            name: "disambiguation-policies".into(),
            title: Some("Disambiguation Policies".into()),
            description: Some(
                "Description of all disambiguation policies (compatible, earlier, later, reject) with when to use each."
                    .into(),
            ),
            mime_type: Some("text/plain".into()),
            size: None,
            icons: None,
            meta: None,
        }
        .no_annotation(),
        RawResource {
            uri: resources::TIMEZONE_CONVERSION_URI.into(),
            name: "workflow-timezone-conversion".into(),
            title: Some("Workflow: Timezone Conversion".into()),
            description: Some(
                "Prompt template for converting times across timezones using Chronow tools.".into(),
            ),
            mime_type: Some("text/plain".into()),
            size: None,
            icons: None,
            meta: None,
        }
        .no_annotation(),
        RawResource {
            uri: resources::SCHEDULE_MEETING_URI.into(),
            name: "workflow-schedule-meeting".into(),
            title: Some("Workflow: Schedule Meeting".into()),
            description: Some(
                "Prompt template for scheduling meetings across timezones using Chronow tools."
                    .into(),
            ),
            mime_type: Some("text/plain".into()),
            size: None,
            icons: None,
            meta: None,
        }
        .no_annotation(),
        RawResource {
            uri: resources::NEXT_BUSINESS_DAY_URI.into(),
            name: "workflow-next-business-day".into(),
            title: Some("Workflow: Next Business Day".into()),
            description: Some(
                "Prompt template for finding the next business day using Chronow tools.".into(),
            ),
            mime_type: Some("text/plain".into()),
            size: None,
            icons: None,
            meta: None,
        }
        .no_annotation(),
    ]
}

/// Look up the text content for a given resource URI.
fn resource_content_for_uri(uri: &str) -> Option<&'static str> {
    match uri {
        resources::DISAMBIGUATION_URI => Some(resources::DISAMBIGUATION_CONTENT),
        resources::TIMEZONE_CONVERSION_URI => Some(resources::TIMEZONE_CONVERSION_CONTENT),
        resources::SCHEDULE_MEETING_URI => Some(resources::SCHEDULE_MEETING_CONTENT),
        resources::NEXT_BUSINESS_DAY_URI => Some(resources::NEXT_BUSINESS_DAY_CONTENT),
        _ => None,
    }
}

#[tool_handler]
impl ServerHandler for ChronowServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(format!(
                "Chronow is a deterministic temporal engine. All operations are pure functions over \
                 UTC instants and IANA timezones. Results are byte-equivalent across Rust, TypeScript, \
                 and Python implementations.\n\n\
                 User's default timezone: {}. When tools accept a `zone` parameter and none is \
                 provided, this timezone is used automatically. You do not need to ask the user for \
                 their timezone.",
                self.default_zone
            )),
            server_info: Implementation {
                name: "chronow".into(),
                version: "0.2.0".into(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListResourcesResult {
            resources: chronow_resources(),
            next_cursor: None,
            meta: None,
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        let result = match resource_content_for_uri(&request.uri) {
            Some(text) => Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: request.uri,
                    mime_type: Some("text/plain".into()),
                    text: text.into(),
                    meta: None,
                }],
            }),
            None => Err(McpError::resource_not_found(
                format!("unknown resource URI: {}", request.uri),
                None,
            )),
        };
        std::future::ready(result)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let default_zone = detect_default_zone();
    eprintln!("[chronow-mcp] default timezone: {default_zone}");

    let server = ChronowServer {
        tool_router: ChronowServer::tool_router(),
        default_zone,
    };

    let transport = stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------ //
    // Helper: convert `Result<String, String>` into the same shape the
    // rmcp framework would produce (`CallToolResult`) so we can inspect
    // `is_error` in assertions.
    // ------------------------------------------------------------------ //
    use rmcp::handler::server::tool::IntoCallToolResult;

    fn to_result(r: Result<String, String>) -> rmcp::model::CallToolResult {
        r.into_call_tool_result()
            .expect("framework conversion should not fail")
    }

    fn is_error(r: &rmcp::model::CallToolResult) -> bool {
        r.is_error == Some(true)
    }

    fn first_text(r: &rmcp::model::CallToolResult) -> String {
        r.content
            .first()
            .and_then(|c| c.raw.as_text())
            .map(|t| t.text.clone())
            .unwrap_or_default()
    }

    fn test_server() -> ChronowServer {
        ChronowServer {
            tool_router: ChronowServer::tool_router(),
            default_zone: "America/Chicago".to_string(),
        }
    }

    // ------------------------------------------------------------------ //
    // 1. Invalid timezone  -- engine returns ok:false with code
    //    "invalid_zone".  Our `eval` wrapper must surface this as
    //    `is_error: true`.
    // ------------------------------------------------------------------ //
    #[test]
    fn error_invalid_timezone() {
        let result = to_result(eval(json!({
            "op": "zone_info",
            "zone": "Fake/Not_A_Zone"
        })));
        assert!(
            is_error(&result),
            "expected is_error: true for invalid timezone"
        );
        let text = first_text(&result);
        assert!(
            text.contains("invalid_zone"),
            "error text should contain error code; got: {text}"
        );
    }

    // ------------------------------------------------------------------ //
    // 2. Unparseable instant -- e.g. feeding garbage to `parse_instant`.
    // ------------------------------------------------------------------ //
    #[test]
    fn error_unparseable_instant() {
        let result = to_result(eval(json!({
            "op": "parse_instant",
            "input": "not-a-date"
        })));
        assert!(
            is_error(&result),
            "expected is_error: true for unparseable instant"
        );
        let text = first_text(&result);
        assert!(
            text.contains("invalid_datetime") || text.contains("invalid_request"),
            "error text should indicate a datetime/request error; got: {text}"
        );
    }

    // ------------------------------------------------------------------ //
    // 3. Missing required field -- `require_non_empty` catches empty
    //    strings before the engine is even called.
    // ------------------------------------------------------------------ //
    #[test]
    fn error_missing_required_field() {
        // Simulate what happens when a handler calls require_non_empty
        // with an empty string.
        let validation = require_non_empty("zone", "");
        assert!(validation.is_err(), "empty string should fail validation");
        let msg = validation.unwrap_err();
        assert!(
            msg.contains("missing required field") && msg.contains("zone"),
            "message should name the field; got: {msg}"
        );

        // Wrap it through the framework converter to confirm is_error semantics.
        let result = to_result(Err(msg));
        assert!(is_error(&result));
    }

    // ------------------------------------------------------------------ //
    // 4. Engine success path still works (sanity check).
    // ------------------------------------------------------------------ //
    #[test]
    fn success_parse_instant() {
        let result = to_result(eval(json!({
            "op": "parse_instant",
            "input": "2024-06-15T12:00:00Z"
        })));
        assert!(!is_error(&result), "valid input should not be an error");
        let text = first_text(&result);
        assert!(
            text.contains("\"ok\": true"),
            "response should contain ok:true; got: {text}"
        );
    }

    // ------------------------------------------------------------------ //
    // 5. Whitespace-only string is caught by require_non_empty.
    // ------------------------------------------------------------------ //
    #[test]
    fn error_whitespace_only_field() {
        let validation = require_non_empty("instant", "   ");
        assert!(validation.is_err());
        assert!(validation.unwrap_err().contains("missing required field"));
    }

    // ------------------------------------------------------------------ //
    // 6. Engine error for invalid format_instant zone propagates as
    //    is_error: true through the full eval pipeline.
    // ------------------------------------------------------------------ //
    #[test]
    fn error_format_instant_bad_zone() {
        let result = to_result(eval(json!({
            "op": "format_instant",
            "instant": "2024-06-15T12:00:00Z",
            "zone": "Mars/Olympus_Mons",
            "format": "extended"
        })));
        assert!(is_error(&result));
        let text = first_text(&result);
        assert!(text.contains("invalid_zone"), "got: {text}");
    }

    // ------------------------------------------------------------------ //
    // 7. Timezone detection fallback chain.
    // ------------------------------------------------------------------ //
    #[test]
    fn detect_zone_returns_nonempty() {
        let zone = detect_default_zone();
        assert!(!zone.is_empty(), "detected zone should not be empty");
    }

    // ------------------------------------------------------------------ //
    // 8. resolve_zone uses default when None is provided.
    // ------------------------------------------------------------------ //
    #[test]
    fn resolve_zone_uses_default() {
        let server = test_server();
        assert_eq!(server.resolve_zone(None), "America/Chicago");
        assert_eq!(server.resolve_zone(Some("".to_string())), "America/Chicago");
        assert_eq!(
            server.resolve_zone(Some("  ".to_string())),
            "America/Chicago"
        );
        assert_eq!(
            server.resolve_zone(Some("Asia/Tokyo".to_string())),
            "Asia/Tokyo"
        );
    }

    // ================================================================== //
    // Resource tests
    // ================================================================== //

    // ------------------------------------------------------------------ //
    // 9. list_resources returns all four resources.
    // ------------------------------------------------------------------ //
    #[test]
    fn list_resources_returns_all() {
        let resources = chronow_resources();
        assert_eq!(resources.len(), 4, "expected 4 resources");

        let uris: Vec<&str> = resources.iter().map(|r| r.raw.uri.as_str()).collect();
        assert!(uris.contains(&"chronow://policies/disambiguation"));
        assert!(uris.contains(&"chronow://workflows/timezone-conversion"));
        assert!(uris.contains(&"chronow://workflows/schedule-meeting"));
        assert!(uris.contains(&"chronow://workflows/next-business-day"));

        // All should have text/plain mime type
        for r in &resources {
            assert_eq!(
                r.raw.mime_type.as_deref(),
                Some("text/plain"),
                "resource {} should be text/plain",
                r.raw.uri
            );
        }
    }

    // ------------------------------------------------------------------ //
    // 10. read_resource returns content for each known URI.
    // ------------------------------------------------------------------ //
    #[test]
    fn read_resource_disambiguation() {
        let content = resource_content_for_uri("chronow://policies/disambiguation");
        assert!(content.is_some(), "disambiguation resource should exist");
        let text = content.unwrap();
        assert!(
            text.contains("compatible"),
            "should describe compatible policy"
        );
        assert!(text.contains("earlier"), "should describe earlier policy");
        assert!(text.contains("later"), "should describe later policy");
        assert!(text.contains("reject"), "should describe reject policy");
    }

    #[test]
    fn read_resource_timezone_conversion() {
        let content = resource_content_for_uri("chronow://workflows/timezone-conversion");
        assert!(
            content.is_some(),
            "timezone-conversion resource should exist"
        );
        let text = content.unwrap();
        assert!(
            text.contains("resolve_local"),
            "should reference resolve_local tool"
        );
        assert!(
            text.contains("format_instant"),
            "should reference format_instant tool"
        );
    }

    #[test]
    fn read_resource_schedule_meeting() {
        let content = resource_content_for_uri("chronow://workflows/schedule-meeting");
        assert!(content.is_some(), "schedule-meeting resource should exist");
        let text = content.unwrap();
        assert!(
            text.contains("resolve_local"),
            "should reference resolve_local tool"
        );
        assert!(
            text.contains("format_instant"),
            "should reference format_instant tool"
        );
        assert!(
            text.contains("diff_instants"),
            "should reference diff_instants tool"
        );
    }

    #[test]
    fn read_resource_next_business_day() {
        let content = resource_content_for_uri("chronow://workflows/next-business-day");
        assert!(content.is_some(), "next-business-day resource should exist");
        let text = content.unwrap();
        assert!(
            text.contains("recurrence_preview"),
            "should reference recurrence_preview tool"
        );
        assert!(
            text.contains("exclude_weekends"),
            "should mention weekend exclusion"
        );
        assert!(text.contains("holidays"), "should mention holidays");
    }

    // ------------------------------------------------------------------ //
    // 11. read_resource returns None for unknown URIs.
    // ------------------------------------------------------------------ //
    #[test]
    fn read_resource_unknown_uri() {
        let content = resource_content_for_uri("chronow://nonexistent/thing");
        assert!(content.is_none(), "unknown URI should return None");
    }

    // ------------------------------------------------------------------ //
    // 12. All resources have non-empty names, titles, and descriptions.
    // ------------------------------------------------------------------ //
    #[test]
    fn resources_have_metadata() {
        for r in chronow_resources() {
            assert!(!r.raw.name.is_empty(), "resource name should not be empty");
            assert!(
                r.raw.title.is_some() && !r.raw.title.as_ref().unwrap().is_empty(),
                "resource {} should have a non-empty title",
                r.raw.uri
            );
            assert!(
                r.raw.description.is_some() && !r.raw.description.as_ref().unwrap().is_empty(),
                "resource {} should have a non-empty description",
                r.raw.uri
            );
        }
    }
}
