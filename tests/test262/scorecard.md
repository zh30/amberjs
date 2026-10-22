# Amber test262 scorecard

- Generated: 2026-10-22T02:40Z
- test262 pin: `d53ebc8b1931d9f1541e43d59650776e4b766248`
- Amber: `./target/release/amber`
- Filter: `language/types`
- Elapsed: 2.0s

| Status | Count |
|--------|------:|
| PASS | 202 |
| FAIL | 5 |
| SKIP | 4 |
| TOTAL | 211 |

**Gate: FAIL** (PASS=202 FAIL=5 SKIP=4; filtered path=language/types (not full-suite gate))

## Sample failures

| Test | Mode | Reason |
|------|------|--------|
| `language/types/reference/8.7.2-1-s.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: Expected a ReferenceError to be thrown but no exception was thrown at all |
| `language/types/reference/8.7.2-3-a-1gs.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: Expected a ReferenceError to be thrown but no exception was thrown at all |
| `language/types/reference/8.7.2-3-s.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: Expected a TypeError to be thrown but no exception was thrown at all |
| `language/types/reference/8.7.2-4-s.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: Expected a TypeError to be thrown but no exception was thrown at all |
| `language/types/reference/8.7.2-5-s.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: Expected a TypeError to be thrown but no exception was thrown at all |

## Sample skips

| Test | Mode | Reason |
|------|------|--------|
| `language/types/reference/get-value-prop-base-primitive-realm.js` | non-strict | unsupported-feature:cross-realm |
| `language/types/reference/get-value-prop-base-primitive-realm.js` | strict | unsupported-feature:cross-realm |
| `language/types/reference/put-value-prop-base-primitive-realm.js` | non-strict | unsupported-feature:cross-realm |
| `language/types/reference/put-value-prop-base-primitive-realm.js` | strict | unsupported-feature:cross-realm |
