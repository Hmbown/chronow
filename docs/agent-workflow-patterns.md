# Agent Workflow Patterns

## 1) Normalize then resolve
1. Call `normalize_intent`.
2. Execute returned normalized request (`resolve_local` or `recurrence_preview`).
3. Persist resulting `instant` + `zone` + `resolved_local`.

## 2) User timezone-safe edits
1. Read existing event in UTC instant form.
2. Convert to user zone via `format_instant`.
3. Apply edits with `add_duration` using `calendar` arithmetic.
4. Store result back as UTC instant.

## 3) Recurrence planning with business constraints
1. Build `recurrence_preview` with explicit `count`.
2. Set `business_calendar.exclude_weekends` and `holidays`.
3. Show preview before writing final schedule.

## 4) DST guardrails for agents
- Always set `disambiguation` explicitly for local-time operations.
- Use `reject` for compliance workflows.
- Use `compatible` for user-friendly scheduling where forward-shift behavior is acceptable.

## 5) Cross-language parity check in automation
Run on every release candidate:
```bash
python3 conformance/runner/run.py --matrix rust ts python --strict
```
