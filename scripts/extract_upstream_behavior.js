#!/usr/bin/env node
/**
 * Upstream behavior extraction for dayjs and luxon.
 *
 * Runs a standard set of temporal operations through each JS library and
 * captures their output for comparison with chronow's conformance corpus.
 *
 * Usage:
 *   npm install dayjs luxon   # one-time
 *   node scripts/extract_upstream_behavior.js
 *
 * Output: conformance/upstream/dayjs.json, conformance/upstream/luxon.json
 */

const fs = require("fs");
const path = require("path");

const OUTPUT_DIR = path.join(__dirname, "..", "conformance", "upstream");

// ── Scenario definitions ────────────────────────────────────────────────
// Each scenario mirrors a chronow operation so we can compare behavior.

const ISO_PARSE_INPUTS = [
  "2024-01-01T00:00:00Z",
  "2024-02-29T23:59:59Z",
  "2019-03-03T19:00:52+14:00",
  "2019-03-03T19:00:52-11:00",
  "2024-09-17",
  "2024-09-17T10:15:30",
  "2024-13-01T00:00:00Z",
  "not-a-date",
];

const FORMAT_SCENARIOS = [
  { instant: "2024-04-10T00:00:00Z", zone: "America/New_York" },
  { instant: "2024-09-17T10:00:00Z", zone: "Europe/London" },
  { instant: "2024-06-15T12:00:00Z", zone: "Asia/Singapore" },
  { instant: "2025-03-30T00:30:00Z", zone: "Europe/London" },
];

const DST_RESOLVE_SCENARIOS = [
  // Spring-forward gap (America/New_York 2024-03-10 02:30 does not exist)
  { local: "2024-03-10T02:30:00", zone: "America/New_York", label: "spring_gap" },
  // Fall-back fold (America/New_York 2024-11-03 01:30 is ambiguous)
  { local: "2024-11-03T01:30:00", zone: "America/New_York", label: "fall_fold" },
  // Europe/London spring gap 2024-03-31 01:30
  { local: "2024-03-31T01:30:00", zone: "Europe/London", label: "uk_spring_gap" },
  // Normal (no ambiguity)
  { local: "2024-06-15T12:00:00", zone: "America/New_York", label: "normal" },
];

const DURATION_ADD_SCENARIOS = [
  { instant: "2024-01-31T12:00:00Z", zone: "UTC", add: { months: 1 }, label: "jan31+1m" },
  { instant: "2024-02-29T12:00:00Z", zone: "UTC", add: { years: 1 }, label: "leap+1y" },
  { instant: "2024-03-10T06:00:00Z", zone: "America/New_York", add: { days: 1 }, label: "dst_spring+1d" },
  { instant: "2024-11-03T05:00:00Z", zone: "America/New_York", add: { days: 1 }, label: "dst_fall+1d" },
  { instant: "2024-03-10T06:00:00Z", zone: "America/New_York", add: { hours: 24 }, label: "dst_spring+24h" },
];

const DIFF_SCENARIOS = [
  { start: "2024-01-01T00:00:00Z", end: "2024-12-31T23:59:59Z", label: "full_year" },
  { start: "2024-01-31T00:00:00Z", end: "2024-02-29T00:00:00Z", label: "month_clamp_leap" },
  { start: "2023-01-31T00:00:00Z", end: "2023-02-28T00:00:00Z", label: "month_clamp_nonleap" },
  { start: "2020-02-29T00:00:00Z", end: "2021-02-28T00:00:00Z", label: "leap_to_nonleap" },
];

const DURATION_PARSE_INPUTS = [
  "P1Y", "P1M", "P1W", "P1D", "PT1H", "PT1M", "PT1S",
  "P1Y2M3DT4H5M6S", "P0Y0M0DT0H0M0S", "-P1D",
];

// ── dayjs extraction ────────────────────────────────────────────────────
function extractDayjs() {
  let dayjs, utc, tz, duration, customParseFormat;
  try {
    dayjs = require("dayjs");
    utc = require("dayjs/plugin/utc");
    tz = require("dayjs/plugin/timezone");
    duration = require("dayjs/plugin/duration");
    customParseFormat = require("dayjs/plugin/customParseFormat");
  } catch {
    console.error("dayjs not installed. Run: npm install dayjs");
    return null;
  }

  dayjs.extend(utc);
  dayjs.extend(tz);
  dayjs.extend(duration);
  dayjs.extend(customParseFormat);

  const results = { library: "dayjs", version: dayjs.version || "unknown", scenarios: {} };

  // Parse
  results.scenarios.parse = ISO_PARSE_INPUTS.map((input) => {
    const d = dayjs(input);
    return {
      input,
      valid: d.isValid(),
      utc_iso: d.isValid() ? d.utc().toISOString() : null,
      epoch_ms: d.isValid() ? d.valueOf() : null,
    };
  });

  // Format in timezone
  results.scenarios.format = FORMAT_SCENARIOS.map(({ instant, zone }) => {
    const d = dayjs(instant).tz(zone);
    return {
      instant,
      zone,
      formatted: d.isValid() ? d.format("YYYY-MM-DDTHH:mm:ssZ") : null,
      offset: d.isValid() ? d.format("Z") : null,
    };
  });

  // DST resolution
  results.scenarios.dst_resolve = DST_RESOLVE_SCENARIOS.map(({ local, zone, label }) => {
    const d = dayjs.tz(local, zone);
    return {
      label,
      local,
      zone,
      valid: d.isValid(),
      utc_iso: d.isValid() ? d.utc().toISOString() : null,
      offset: d.isValid() ? d.format("Z") : null,
      note: "dayjs uses implicit resolution (no disambiguation parameter)",
    };
  });

  // Duration addition
  results.scenarios.add_duration = DURATION_ADD_SCENARIOS.map(({ instant, zone, add, label }) => {
    const d = dayjs(instant).tz(zone);
    let result = d;
    for (const [unit, value] of Object.entries(add)) {
      result = result.add(value, unit);
    }
    return {
      label,
      instant,
      zone,
      add,
      result_utc: result.utc().toISOString(),
      result_local: result.format("YYYY-MM-DDTHH:mm:ss"),
    };
  });

  // Diff
  results.scenarios.diff = DIFF_SCENARIOS.map(({ start, end, label }) => {
    const s = dayjs(start);
    const e = dayjs(end);
    return {
      label,
      start,
      end,
      diff_days: e.diff(s, "day"),
      diff_months: e.diff(s, "month"),
      diff_years: e.diff(s, "year"),
      diff_ms: e.diff(s),
    };
  });

  // Duration parsing
  results.scenarios.duration_parse = DURATION_PARSE_INPUTS.map((input) => {
    try {
      const d = dayjs.duration(input);
      return {
        input,
        valid: true,
        iso: d.toISOString(),
        total_seconds: d.asSeconds(),
      };
    } catch {
      return { input, valid: false, iso: null, total_seconds: null };
    }
  });

  return results;
}

// ── luxon extraction ────────────────────────────────────────────────────
function extractLuxon() {
  let luxon;
  try {
    luxon = require("luxon");
  } catch {
    console.error("luxon not installed. Run: npm install luxon");
    return null;
  }

  const { DateTime, Duration, Interval } = luxon;
  const results = { library: "luxon", version: luxon.VERSION || "unknown", scenarios: {} };

  // Parse
  results.scenarios.parse = ISO_PARSE_INPUTS.map((input) => {
    const d = DateTime.fromISO(input, { zone: "utc" });
    return {
      input,
      valid: d.isValid,
      utc_iso: d.isValid ? d.toUTC().toISO() : null,
      epoch_ms: d.isValid ? d.toMillis() : null,
      invalid_reason: d.isValid ? null : d.invalidReason,
    };
  });

  // Format in timezone
  results.scenarios.format = FORMAT_SCENARIOS.map(({ instant, zone }) => {
    const d = DateTime.fromISO(instant, { zone });
    return {
      instant,
      zone,
      formatted: d.isValid ? d.toISO() : null,
      offset: d.isValid ? d.toFormat("ZZ") : null,
    };
  });

  // DST resolution
  results.scenarios.dst_resolve = DST_RESOLVE_SCENARIOS.map(({ local, zone, label }) => {
    const d = DateTime.fromISO(local, { zone });
    return {
      label,
      local,
      zone,
      valid: d.isValid,
      utc_iso: d.isValid ? d.toUTC().toISO() : null,
      offset: d.isValid ? d.toFormat("ZZ") : null,
      note: "luxon resolves gaps by shifting forward and folds by picking the first occurrence",
    };
  });

  // Duration addition
  results.scenarios.add_duration = DURATION_ADD_SCENARIOS.map(({ instant, zone, add, label }) => {
    const d = DateTime.fromISO(instant, { zone });
    const result = d.plus(add);
    return {
      label,
      instant,
      zone,
      add,
      result_utc: result.toUTC().toISO(),
      result_local: result.toFormat("yyyy-MM-dd'T'HH:mm:ss"),
    };
  });

  // Diff
  results.scenarios.diff = DIFF_SCENARIOS.map(({ start, end, label }) => {
    const s = DateTime.fromISO(start);
    const e = DateTime.fromISO(end);
    const diff = e.diff(s, ["years", "months", "days", "hours", "minutes", "seconds"]);
    return {
      label,
      start,
      end,
      diff_days: Math.floor(e.diff(s, "days").days),
      diff_months: Math.floor(e.diff(s, "months").months),
      diff_years: Math.floor(e.diff(s, "years").years),
      diff_ms: e.diff(s).milliseconds,
      diff_breakdown: diff.toObject(),
    };
  });

  // Duration parsing
  results.scenarios.duration_parse = DURATION_PARSE_INPUTS.map((input) => {
    const d = Duration.fromISO(input);
    return {
      input,
      valid: d.isValid,
      iso: d.isValid ? d.toISO() : null,
      total_seconds: d.isValid ? d.as("seconds") : null,
      invalid_reason: d.isValid ? null : d.invalidReason,
    };
  });

  return results;
}

// ── Main ────────────────────────────────────────────────────────────────
function main() {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });

  const dayjs = extractDayjs();
  if (dayjs) {
    const out = path.join(OUTPUT_DIR, "dayjs.json");
    fs.writeFileSync(out, JSON.stringify(dayjs, null, 2) + "\n");
    console.log(`wrote ${out} (${Object.keys(dayjs.scenarios).length} scenario groups)`);
  }

  const luxon = extractLuxon();
  if (luxon) {
    const out = path.join(OUTPUT_DIR, "luxon.json");
    fs.writeFileSync(out, JSON.stringify(luxon, null, 2) + "\n");
    console.log(`wrote ${out} (${Object.keys(luxon.scenarios).length} scenario groups)`);
  }
}

main();
