export type JsonValue = string | number | boolean | null | JsonObject | JsonValue[];

export interface JsonObject {
  [key: string]: JsonValue;
}

export interface ErrorPayload {
  code: string;
  message: string;
}

export interface EngineResponse {
  ok: boolean;
  value?: JsonValue;
  error?: ErrorPayload;
}

export interface CorpusCase {
  id: string;
  request: JsonObject;
  expected: EngineResponse;
  category: string;
  description: string;
  sources: JsonValue[];
}

export interface EvalCorpusResult {
  results: Array<{ id: string; response: EngineResponse }>;
  count: number;
}

export interface ChronowOptions {
  chronowBin?: string;
  forceCli?: boolean;
}

// --- New v0.2.0 types ---

export interface DurationSpec {
  years?: number;
  months?: number;
  weeks?: number;
  days?: number;
  hours?: number;
  minutes?: number;
  seconds?: number;
}

export interface TimeInterval {
  start: string;
  end: string;
}

export type SnapUnit = "day" | "week" | "month" | "quarter" | "year";
export type SnapEdge = "start" | "end";
export type IntervalCheckMode = "overlap" | "contains" | "gap";

export interface DiffInstantsRequest {
  op: "diff_instants";
  start: string;
  end: string;
  zone: string;
}

export interface CompareInstantsRequest {
  op: "compare_instants";
  a: string;
  b: string;
}

export interface SnapToRequest {
  op: "snap_to";
  instant: string;
  zone: string;
  unit: SnapUnit;
  edge: SnapEdge;
  week_starts_on?: string;
}

export interface ParseDurationRequest {
  op: "parse_duration";
  input: string;
}

export interface FormatDurationRequest {
  op: "format_duration";
  duration: DurationSpec;
}

export interface IntervalCheckRequest {
  op: "interval_check";
  interval_a: TimeInterval;
  interval_b: TimeInterval;
  mode: IntervalCheckMode;
}

export interface ZoneInfoRequest {
  op: "zone_info";
  zone: string;
  at?: string;
}

export interface ListZonesRequest {
  op: "list_zones";
  region_filter?: string;
}

export interface NowRequest {
  op: "now";
  zone?: string;
}
