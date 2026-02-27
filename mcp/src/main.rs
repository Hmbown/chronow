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
        let text = serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
            format!("{{\"ok\":true,\"_serialization_note\":\"{}\"}}", e)
        });
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
    fn parse_instant(&self, Parameters(p): Parameters<ParseInstantParams>) -> Result<String, String> {
        require_non_empty("input", &p.input)?;
        eval(json!({"op": "parse_instant", "input": p.input}))
    }

    /// Format a UTC instant in a specific timezone with the given format (extended/basic/date/time).
    #[tool]
    fn format_instant(&self, Parameters(p): Parameters<FormatInstantParams>) -> Result<String, String> {
        require_non_empty("instant", &p.instant)?;
        require_non_empty("zone", &p.zone)?;
        eval(json!({"op": "format_instant", "instant": p.instant, "zone": p.zone, "format": p.format}))
    }

    /// Resolve a local datetime in a timezone to a UTC instant, handling DST ambiguity.
    #[tool]
    fn resolve_local(&self, Parameters(p): Parameters<ResolveLocalParams>) -> Result<String, String> {
        require_non_empty("local", &p.local)?;
        require_non_empty("zone", &p.zone)?;
        eval(json!({"op": "resolve_local", "local": p.local, "zone": p.zone, "disambiguation": p.disambiguation}))
    }

    /// Add a duration to an instant using absolute or calendar arithmetic.
    #[tool]
    fn add_duration(&self, Parameters(p): Parameters<AddDurationParams>) -> Result<String, String> {
        require_non_empty("start", &p.start)?;
        require_non_empty("zone", &p.zone)?;
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
    fn recurrence_preview(&self, Parameters(p): Parameters<RecurrencePreviewParams>) -> Result<String, String> {
        require_non_empty("start_local", &p.start_local)?;
        require_non_empty("zone", &p.zone)?;
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
    fn normalize_intent(&self, Parameters(p): Parameters<NormalizeIntentParams>) -> Result<String, String> {
        require_non_empty("input", &p.input)?;
        require_non_empty("reference_local", &p.reference_local)?;
        require_non_empty("default_zone", &p.default_zone)?;
        eval(json!({
            "op": "normalize_intent",
            "input": p.input,
            "reference_local": p.reference_local,
            "default_zone": p.default_zone,
        }))
    }

    /// Compute the calendar difference between two instants (years, months, days, hours, minutes, seconds).
    #[tool]
    fn diff_instants(&self, Parameters(p): Parameters<DiffInstantsParams>) -> Result<String, String> {
        require_non_empty("start", &p.start)?;
        require_non_empty("end", &p.end)?;
        require_non_empty("zone", &p.zone)?;
        eval(json!({"op": "diff_instants", "start": p.start, "end": p.end, "zone": p.zone}))
    }

    /// Compare two instants, returning -1, 0, or 1.
    #[tool]
    fn compare_instants(&self, Parameters(p): Parameters<CompareInstantsParams>) -> Result<String, String> {
        require_non_empty("a", &p.a)?;
        require_non_empty("b", &p.b)?;
        eval(json!({"op": "compare_instants", "a": p.a, "b": p.b}))
    }

    /// Snap an instant to the start or end of a calendar unit (day/week/month/quarter/year).
    #[tool]
    fn snap_to(&self, Parameters(p): Parameters<SnapToParams>) -> Result<String, String> {
        require_non_empty("instant", &p.instant)?;
        require_non_empty("zone", &p.zone)?;
        require_non_empty("unit", &p.unit)?;
        require_non_empty("edge", &p.edge)?;
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
    fn parse_duration(&self, Parameters(p): Parameters<ParseDurationParams>) -> Result<String, String> {
        require_non_empty("input", &p.input)?;
        eval(json!({"op": "parse_duration", "input": p.input}))
    }

    /// Format duration components into an ISO 8601 duration string.
    #[tool]
    fn format_duration(&self, Parameters(p): Parameters<FormatDurationParams>) -> Result<String, String> {
        eval(json!({"op": "format_duration", "duration": p.duration}))
    }

    /// Check two time intervals for overlap, containment, or gap.
    #[tool]
    fn interval_check(&self, Parameters(p): Parameters<IntervalCheckParams>) -> Result<String, String> {
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
        require_non_empty("zone", &p.zone)?;
        let mut req = json!({"op": "zone_info", "zone": p.zone});
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

    /// Get the current time as a UTC instant, optionally projected into a timezone.
    #[tool]
    fn now(&self, Parameters(p): Parameters<NowParams>) -> Result<String, String> {
        let mut req = json!({"op": "now"});
        if let Some(ref z) = p.zone {
            if z.trim().is_empty() {
                return Err("field `zone` must be non-empty when provided".to_string());
            }
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
        r.into_call_tool_result().expect("framework conversion should not fail")
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
        assert!(is_error(&result), "expected is_error: true for invalid timezone");
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
        assert!(is_error(&result), "expected is_error: true for unparseable instant");
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
        assert!(text.contains("\"ok\": true"), "response should contain ok:true; got: {text}");
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
}
