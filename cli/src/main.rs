use std::fs;
use std::path::PathBuf;

use chronow_core::{evaluate_request, evaluate_request_value, Disambiguation, Request};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Parser, Debug)]
#[command(name = "chronow")]
#[command(about = "Deterministic temporal engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Parse {
        #[arg(long)]
        input: String,
    },
    Convert {
        #[arg(long)]
        input: String,
        #[arg(long)]
        zone: String,
        #[arg(long, default_value = "extended")]
        format: String,
    },
    Resolve {
        #[arg(long)]
        local: String,
        #[arg(long)]
        zone: String,
        #[arg(long, default_value = "compatible")]
        disambiguation: String,
    },
    Recur {
        #[arg(long)]
        start_local: String,
        #[arg(long)]
        zone: String,
        #[arg(long)]
        freq: String,
        #[arg(long)]
        count: usize,
        #[arg(long, default_value_t = 1)]
        interval: u32,
        #[arg(long)]
        weekdays: Option<String>,
        #[arg(long, default_value_t = false)]
        exclude_weekends: bool,
        #[arg(long)]
        holidays: Option<String>,
        #[arg(long, default_value = "compatible")]
        disambiguation: String,
    },
    Diff {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        zone: String,
    },
    Compare {
        #[arg(long)]
        a: String,
        #[arg(long)]
        b: String,
    },
    Snap {
        #[arg(long)]
        instant: String,
        #[arg(long)]
        zone: String,
        #[arg(long)]
        unit: String,
        #[arg(long)]
        edge: String,
        #[arg(long, default_value = "monday")]
        week_starts_on: String,
    },
    ParseDuration {
        #[arg(long)]
        input: String,
    },
    FormatDuration {
        #[arg(long, default_value_t = 0)]
        years: i32,
        #[arg(long, default_value_t = 0)]
        months: i32,
        #[arg(long, default_value_t = 0)]
        weeks: i64,
        #[arg(long, default_value_t = 0)]
        days: i64,
        #[arg(long, default_value_t = 0)]
        hours: i64,
        #[arg(long, default_value_t = 0)]
        minutes: i64,
        #[arg(long, default_value_t = 0)]
        seconds: i64,
    },
    IntervalCheck {
        #[arg(long)]
        a_start: String,
        #[arg(long)]
        a_end: String,
        #[arg(long)]
        b_start: String,
        #[arg(long)]
        b_end: String,
        #[arg(long)]
        mode: String,
    },
    ZoneInfo {
        #[arg(long)]
        zone: String,
        #[arg(long)]
        at: Option<String>,
    },
    ListZones {
        #[arg(long)]
        region_filter: Option<String>,
    },
    Now {
        #[arg(long)]
        zone: Option<String>,
    },
    Eval {
        #[arg(long)]
        request: Option<String>,
        #[arg(long)]
        request_file: Option<PathBuf>,
    },
    EvalCorpus {
        #[arg(long)]
        cases_file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let output = match cli.command {
        Commands::Parse { input } => {
            let req = Request::ParseInstant { input };
            serde_json::to_value(evaluate_request(req)).expect("serialize parse response")
        }
        Commands::Convert {
            input,
            zone,
            format,
        } => {
            let req = Request::FormatInstant {
                instant: input,
                zone,
                format,
            };
            serde_json::to_value(evaluate_request(req)).expect("serialize format response")
        }
        Commands::Resolve {
            local,
            zone,
            disambiguation,
        } => {
            let req = Request::ResolveLocal {
                local,
                zone,
                disambiguation: parse_disambiguation(&disambiguation),
            };
            serde_json::to_value(evaluate_request(req)).expect("serialize resolve response")
        }
        Commands::Recur {
            start_local,
            zone,
            freq,
            count,
            interval,
            weekdays,
            exclude_weekends,
            holidays,
            disambiguation,
        } => {
            let by_weekdays = weekdays
                .map(|v| {
                    v.split(',')
                        .map(|item| item.trim().to_lowercase())
                        .filter(|item| !item.is_empty())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            let holiday_values = holidays
                .map(|v| {
                    v.split(',')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            let request = json!({
                "op": "recurrence_preview",
                "start_local": start_local,
                "zone": zone,
                "rule": {
                    "frequency": freq,
                    "interval": interval,
                    "count": count,
                    "by_weekdays": by_weekdays,
                },
                "business_calendar": {
                    "exclude_weekends": exclude_weekends,
                    "holidays": holiday_values,
                },
                "disambiguation": disambiguation,
            });

            serde_json::to_value(evaluate_request_value(request)).expect("serialize recur response")
        }
        Commands::Diff { start, end, zone } => {
            let request = json!({
                "op": "diff_instants",
                "start": start,
                "end": end,
                "zone": zone,
            });
            serde_json::to_value(evaluate_request_value(request)).expect("serialize diff response")
        }
        Commands::Compare { a, b } => {
            let request = json!({
                "op": "compare_instants",
                "a": a,
                "b": b,
            });
            serde_json::to_value(evaluate_request_value(request)).expect("serialize compare response")
        }
        Commands::Snap {
            instant,
            zone,
            unit,
            edge,
            week_starts_on,
        } => {
            let request = json!({
                "op": "snap_to",
                "instant": instant,
                "zone": zone,
                "unit": unit,
                "edge": edge,
                "week_starts_on": week_starts_on,
            });
            serde_json::to_value(evaluate_request_value(request)).expect("serialize snap response")
        }
        Commands::ParseDuration { input } => {
            let request = json!({
                "op": "parse_duration",
                "input": input,
            });
            serde_json::to_value(evaluate_request_value(request)).expect("serialize parse-duration response")
        }
        Commands::FormatDuration {
            years,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
        } => {
            let request = json!({
                "op": "format_duration",
                "duration": {
                    "years": years,
                    "months": months,
                    "weeks": weeks,
                    "days": days,
                    "hours": hours,
                    "minutes": minutes,
                    "seconds": seconds,
                },
            });
            serde_json::to_value(evaluate_request_value(request)).expect("serialize format-duration response")
        }
        Commands::IntervalCheck {
            a_start,
            a_end,
            b_start,
            b_end,
            mode,
        } => {
            let request = json!({
                "op": "interval_check",
                "interval_a": {"start": a_start, "end": a_end},
                "interval_b": {"start": b_start, "end": b_end},
                "mode": mode,
            });
            serde_json::to_value(evaluate_request_value(request)).expect("serialize interval-check response")
        }
        Commands::ZoneInfo { zone, at } => {
            let mut request = json!({
                "op": "zone_info",
                "zone": zone,
            });
            if let Some(at_val) = at {
                request["at"] = json!(at_val);
            }
            serde_json::to_value(evaluate_request_value(request)).expect("serialize zone-info response")
        }
        Commands::ListZones { region_filter } => {
            let mut request = json!({
                "op": "list_zones",
            });
            if let Some(filter) = region_filter {
                request["region_filter"] = json!(filter);
            }
            serde_json::to_value(evaluate_request_value(request)).expect("serialize list-zones response")
        }
        Commands::Now { zone } => {
            let mut request = json!({
                "op": "now",
            });
            if let Some(z) = zone {
                request["zone"] = json!(z);
            }
            serde_json::to_value(evaluate_request_value(request)).expect("serialize now response")
        }
        Commands::Eval {
            request,
            request_file,
        } => {
            let req_json = if let Some(raw) = request {
                serde_json::from_str::<Value>(&raw).expect("--request must be valid JSON")
            } else if let Some(path) = request_file {
                let content = fs::read_to_string(path).expect("read request file");
                serde_json::from_str::<Value>(&content).expect("request file must be valid JSON")
            } else {
                panic!("provide --request or --request-file");
            };

            serde_json::to_value(evaluate_request_value(req_json)).expect("serialize eval response")
        }
        Commands::EvalCorpus { cases_file } => {
            let content = fs::read_to_string(cases_file).expect("read cases file");
            let parsed: Value = serde_json::from_str(&content).expect("valid cases JSON");

            let cases = if parsed.is_array() {
                parsed.as_array().expect("cases array").clone()
            } else {
                parsed
                    .get("cases")
                    .and_then(Value::as_array)
                    .expect("expected array or object with cases field")
                    .clone()
            };

            let results: Vec<Value> = cases
                .iter()
                .map(|case| {
                    let id = case
                        .get("id")
                        .and_then(Value::as_str)
                        .expect("case id")
                        .to_string();
                    let request = case.get("request").expect("case request").clone();
                    let response = serde_json::to_value(evaluate_request_value(request))
                        .expect("serialize corpus response");
                    json!({
                        "id": id,
                        "response": response,
                    })
                })
                .collect();

            json!({
                "results": results,
                "count": results.len(),
            })
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("serialize final output")
    );
}

fn parse_disambiguation(value: &str) -> Disambiguation {
    match value.to_ascii_lowercase().as_str() {
        "compatible" => Disambiguation::Compatible,
        "earlier" => Disambiguation::Earlier,
        "later" => Disambiguation::Later,
        "reject" => Disambiguation::Reject,
        _ => Disambiguation::Compatible,
    }
}
