use std::fs;
use std::path::PathBuf;

use chronow_core::{Disambiguation, Request, evaluate_request, evaluate_request_value};
use clap::{CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};

/// Print a JSON error to stderr and exit with code 1.
fn exit_error(code: &str, message: &str) -> ! {
    let err = serde_json::json!({
        "error": { "code": code, "message": message }
    });
    eprintln!("{}", err);
    std::process::exit(1);
}

/// Serialize a value to `serde_json::Value`, exiting with a JSON error on failure.
fn to_json_value<T: serde::Serialize>(val: T) -> Value {
    serde_json::to_value(val).unwrap_or_else(|e| exit_error("serialization_error", &e.to_string()))
}

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
    /// Add a duration to an instant
    AddDuration {
        #[arg(long)]
        instant: String,
        #[arg(long)]
        duration: String,
        #[arg(long, default_value = "calendar")]
        mode: String,
        #[arg(long)]
        zone: Option<String>,
        #[arg(long, default_value = "compatible")]
        disambiguation: String,
    },
    /// Parse natural language temporal intent
    NormalizeIntent {
        #[arg(long)]
        text: String,
        #[arg(long)]
        ref_now: String,
        #[arg(long)]
        ref_zone: String,
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
    /// Generate shell completion scripts
    Completions {
        /// Target shell (bash, zsh, fish, powershell)
        #[arg(long)]
        shell: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let output = match cli.command {
        Commands::Parse { input } => {
            let req = Request::ParseInstant { input };
            to_json_value(evaluate_request(req))
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
            to_json_value(evaluate_request(req))
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
            to_json_value(evaluate_request(req))
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

            to_json_value(evaluate_request_value(request))
        }
        Commands::Diff { start, end, zone } => {
            let request = json!({
                "op": "diff_instants",
                "start": start,
                "end": end,
                "zone": zone,
            });
            to_json_value(evaluate_request_value(request))
        }
        Commands::Compare { a, b } => {
            let request = json!({
                "op": "compare_instants",
                "a": a,
                "b": b,
            });
            to_json_value(evaluate_request_value(request))
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
            to_json_value(evaluate_request_value(request))
        }
        Commands::ParseDuration { input } => {
            let request = json!({
                "op": "parse_duration",
                "input": input,
            });
            to_json_value(evaluate_request_value(request))
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
            to_json_value(evaluate_request_value(request))
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
            to_json_value(evaluate_request_value(request))
        }
        Commands::ZoneInfo { zone, at } => {
            let mut request = json!({
                "op": "zone_info",
                "zone": zone,
            });
            if let Some(at_val) = at {
                request["at"] = json!(at_val);
            }
            to_json_value(evaluate_request_value(request))
        }
        Commands::ListZones { region_filter } => {
            let mut request = json!({
                "op": "list_zones",
            });
            if let Some(filter) = region_filter {
                request["region_filter"] = json!(filter);
            }
            to_json_value(evaluate_request_value(request))
        }
        Commands::Now { zone } => {
            let mut request = json!({
                "op": "now",
            });
            if let Some(z) = zone {
                request["zone"] = json!(z);
            }
            to_json_value(evaluate_request_value(request))
        }
        Commands::AddDuration {
            instant,
            duration,
            mode,
            zone,
            disambiguation,
        } => {
            // First, parse the ISO duration string into component fields
            let parse_resp = evaluate_request_value(json!({
                "op": "parse_duration",
                "input": duration,
            }));
            if !parse_resp.ok {
                // Duration parsing failed; surface the error directly
                return println!(
                    "{}",
                    serde_json::to_string_pretty(&to_json_value(parse_resp))
                        .unwrap_or_else(|e| exit_error("serialization_error", &e.to_string()))
                );
            }
            let dur = parse_resp.value.unwrap_or_else(|| {
                exit_error("internal_error", "parse_duration returned ok but no value")
            });

            let zone_str = zone.unwrap_or_else(|| {
                if mode == "calendar" {
                    exit_error(
                        "missing_argument",
                        "--zone is required for calendar mode arithmetic",
                    )
                } else {
                    "UTC".to_string()
                }
            });

            let request = json!({
                "op": "add_duration",
                "start": instant,
                "zone": zone_str,
                "duration": {
                    "years": dur["years"],
                    "months": dur["months"],
                    "weeks": dur["weeks"],
                    "days": dur["days"],
                    "hours": dur["hours"],
                    "minutes": dur["minutes"],
                    "seconds": dur["seconds"],
                },
                "arithmetic": mode,
                "disambiguation": disambiguation,
            });
            to_json_value(evaluate_request_value(request))
        }
        Commands::NormalizeIntent {
            text,
            ref_now,
            ref_zone,
        } => {
            let request = json!({
                "op": "normalize_intent",
                "input": text,
                "reference_local": ref_now,
                "default_zone": ref_zone,
            });
            to_json_value(evaluate_request_value(request))
        }
        Commands::Eval {
            request,
            request_file,
        } => {
            let req_json = if let Some(raw) = request {
                serde_json::from_str::<Value>(&raw).unwrap_or_else(|e| {
                    exit_error(
                        "invalid_json",
                        &format!("--request is not valid JSON: {}", e),
                    )
                })
            } else if let Some(path) = request_file {
                let content = fs::read_to_string(&path).unwrap_or_else(|e| {
                    exit_error(
                        "io_error",
                        &format!("failed to read request file '{}': {}", path.display(), e),
                    )
                });
                serde_json::from_str::<Value>(&content).unwrap_or_else(|e| {
                    exit_error(
                        "invalid_json",
                        &format!("request file is not valid JSON: {}", e),
                    )
                })
            } else {
                exit_error("missing_argument", "provide --request or --request-file");
            };

            to_json_value(evaluate_request_value(req_json))
        }
        Commands::Completions { shell } => {
            use clap_complete::{Shell, generate};
            let shell = shell
                .parse::<Shell>()
                .unwrap_or_else(|_| exit_error("invalid_argument", "unsupported shell"));
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "chronow", &mut std::io::stdout());
            return; // Don't go through the normal JSON output path
        }
        Commands::EvalCorpus { cases_file } => {
            let content = fs::read_to_string(&cases_file).unwrap_or_else(|e| {
                exit_error(
                    "io_error",
                    &format!(
                        "failed to read cases file '{}': {}",
                        cases_file.display(),
                        e
                    ),
                )
            });
            let parsed: Value = serde_json::from_str(&content).unwrap_or_else(|e| {
                exit_error(
                    "invalid_json",
                    &format!("cases file is not valid JSON: {}", e),
                )
            });

            let cases = if parsed.is_array() {
                parsed.as_array().unwrap().clone()
            } else {
                parsed
                    .get("cases")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_else(|| {
                        exit_error(
                            "invalid_format",
                            "cases file must be a JSON array or an object with a \"cases\" field",
                        )
                    })
            };

            let results: Vec<Value> = cases
                .iter()
                .enumerate()
                .map(|(i, case)| {
                    let id = case
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_else(|| {
                            exit_error(
                                "invalid_format",
                                &format!("case at index {} is missing a string \"id\" field", i),
                            )
                        })
                        .to_string();
                    let request = case.get("request").cloned().unwrap_or_else(|| {
                        exit_error(
                            "invalid_format",
                            &format!("case '{}' (index {}) is missing a \"request\" field", id, i),
                        )
                    });
                    let response = to_json_value(evaluate_request_value(request));
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

    let output_str = serde_json::to_string_pretty(&output)
        .unwrap_or_else(|e| exit_error("serialization_error", &e.to_string()));
    println!("{}", output_str);
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
