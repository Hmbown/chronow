use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{ServerCapabilities, ServerInfo, Implementation, ToolsCapability},
    schemars, tool, tool_handler, tool_router,
    transport::io::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
struct ChronowServer {
    tool_router: ToolRouter<Self>,
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
    /// IANA timezone name (e.g. "America/New_York")
    zone: String,
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
    /// IANA timezone name
    zone: String,
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
    /// IANA timezone name for calendar arithmetic
    zone: String,
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
    /// IANA timezone name
    zone: String,
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
    /// Default IANA timezone if not specified in input
    default_zone: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DiffInstantsParams {
    /// Start instant (RFC 3339)
    start: String,
    /// End instant (RFC 3339)
    end: String,
    /// IANA timezone for calendar diff computation
    zone: String,
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
    /// IANA timezone name
    zone: String,
    /// Unit to snap to: day, week, month, quarter, year
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
    /// IANA timezone name
    zone: String,
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
    /// Optional IANA timezone for local projection
    #[serde(default)]
    zone: Option<String>,
}

fn eval(request: serde_json::Value) -> String {
    let response = chronow_core::evaluate_request_value(request);
    serde_json::to_string_pretty(&response).unwrap_or_else(|_| {
        r#"{"ok":false,"error":{"code":"serialization_error","message":"failed to serialize"}}"#
            .to_string()
    })
}

#[tool_router]
impl ChronowServer {
    /// Parse an ISO 8601/RFC 3339 datetime string into a UTC instant with epoch timestamps.
    #[tool]
    fn parse_instant(&self, Parameters(p): Parameters<ParseInstantParams>) -> String {
        eval(json!({"op": "parse_instant", "input": p.input}))
    }

    /// Format a UTC instant in a specific timezone with the given format (extended/basic/date/time).
    #[tool]
    fn format_instant(&self, Parameters(p): Parameters<FormatInstantParams>) -> String {
        eval(json!({"op": "format_instant", "instant": p.instant, "zone": p.zone, "format": p.format}))
    }

    /// Resolve a local datetime in a timezone to a UTC instant, handling DST ambiguity.
    #[tool]
    fn resolve_local(&self, Parameters(p): Parameters<ResolveLocalParams>) -> String {
        eval(json!({"op": "resolve_local", "local": p.local, "zone": p.zone, "disambiguation": p.disambiguation}))
    }

    /// Add a duration to an instant using absolute or calendar arithmetic.
    #[tool]
    fn add_duration(&self, Parameters(p): Parameters<AddDurationParams>) -> String {
        eval(json!({
            "op": "add_duration",
            "start": p.start,
            "zone": p.zone,
            "duration": p.duration,
            "arithmetic": p.arithmetic,
            "disambiguation": p.disambiguation,
        }))
    }

    /// Generate a series of recurring datetime occurrences with optional business calendar rules.
    #[tool]
    fn recurrence_preview(&self, Parameters(p): Parameters<RecurrencePreviewParams>) -> String {
        eval(json!({
            "op": "recurrence_preview",
            "start_local": p.start_local,
            "zone": p.zone,
            "rule": p.rule,
            "business_calendar": p.business_calendar,
            "disambiguation": p.disambiguation,
        }))
    }

    /// Normalize a natural language temporal intent into a deterministic request.
    #[tool]
    fn normalize_intent(&self, Parameters(p): Parameters<NormalizeIntentParams>) -> String {
        eval(json!({
            "op": "normalize_intent",
            "input": p.input,
            "reference_local": p.reference_local,
            "default_zone": p.default_zone,
        }))
    }

    /// Compute the calendar difference between two instants (years, months, days, hours, minutes, seconds).
    #[tool]
    fn diff_instants(&self, Parameters(p): Parameters<DiffInstantsParams>) -> String {
        eval(json!({"op": "diff_instants", "start": p.start, "end": p.end, "zone": p.zone}))
    }

    /// Compare two instants, returning -1, 0, or 1.
    #[tool]
    fn compare_instants(&self, Parameters(p): Parameters<CompareInstantsParams>) -> String {
        eval(json!({"op": "compare_instants", "a": p.a, "b": p.b}))
    }

    /// Snap an instant to the start or end of a calendar unit (day/week/month/quarter/year).
    #[tool]
    fn snap_to(&self, Parameters(p): Parameters<SnapToParams>) -> String {
        eval(json!({
            "op": "snap_to",
            "instant": p.instant,
            "zone": p.zone,
            "unit": p.unit,
            "edge": p.edge,
            "week_starts_on": p.week_starts_on,
        }))
    }

    /// Parse an ISO 8601 duration string (e.g. P1Y2M3DT4H5M6S) into components.
    #[tool]
    fn parse_duration(&self, Parameters(p): Parameters<ParseDurationParams>) -> String {
        eval(json!({"op": "parse_duration", "input": p.input}))
    }

    /// Format duration components into an ISO 8601 duration string.
    #[tool]
    fn format_duration(&self, Parameters(p): Parameters<FormatDurationParams>) -> String {
        eval(json!({"op": "format_duration", "duration": p.duration}))
    }

    /// Check two time intervals for overlap, containment, or gap.
    #[tool]
    fn interval_check(&self, Parameters(p): Parameters<IntervalCheckParams>) -> String {
        eval(json!({
            "op": "interval_check",
            "interval_a": p.interval_a,
            "interval_b": p.interval_b,
            "mode": p.mode,
        }))
    }

    /// Get timezone information: offset, DST status, abbreviation, and next transition.
    #[tool]
    fn zone_info(&self, Parameters(p): Parameters<ZoneInfoParams>) -> String {
        let mut req = json!({"op": "zone_info", "zone": p.zone});
        if let Some(at) = p.at {
            req["at"] = json!(at);
        }
        eval(req)
    }

    /// List available IANA timezone names, optionally filtered by region prefix.
    #[tool]
    fn list_zones(&self, Parameters(p): Parameters<ListZonesParams>) -> String {
        let mut req = json!({"op": "list_zones"});
        if let Some(filter) = p.region_filter {
            req["region_filter"] = json!(filter);
        }
        eval(req)
    }

    /// Get the current time as a UTC instant, optionally projected into a timezone.
    #[tool]
    fn now(&self, Parameters(p): Parameters<NowParams>) -> String {
        let mut req = json!({"op": "now"});
        if let Some(z) = p.zone {
            req["zone"] = json!(z);
        }
        eval(req)
    }
}

#[tool_handler]
impl ServerHandler for ChronowServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Chronow is a deterministic temporal engine. All operations are pure functions over UTC instants and IANA timezones. Results are byte-equivalent across Rust, TypeScript, and Python implementations.".into()),
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
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = ChronowServer {
        tool_router: ChronowServer::tool_router(),
    };

    let transport = stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
