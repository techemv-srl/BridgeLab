# BridgeLab Plugin Packs

BridgeLab supports **declarative plugin packs** &ndash; JSON files that extend
the built-in validation and anonymization rules **without writing code** and
without giving plugins any ability to execute arbitrary logic. This is the
safe baseline of the plugin roadmap; scripted plugins (JS/WASM) will layer on
top later.

## Where do plugins live?

```
<config_dir>/BridgeLab/plugins/
├── validation/
│   └── *.json     <- extra validation rules
└── anonymization/
    └── *.json     <- extra PHI fields
```

| Platform | `<config_dir>` |
|---|---|
| Windows | `%APPDATA%` (e.g. `C:\Users\<you>\AppData\Roaming`) |
| macOS   | `~/Library/Application Support` |
| Linux   | `~/.config` |

The easiest way to reach the folder is **Settings → Plugins → Open plugins folder**.

After adding / editing / removing files, hit **Reload** in the Plugins panel.
A reload is also triggered at every app startup.

## Manage plugins from the UI

`Settings → Plugins`:

- Lists every pack found, grouped by kind (`validation` / `anonymization`),
  with author, version, rule count, and the full on-disk path.
- Toggle individual packs on/off; the preference is persisted so the choice
  survives restarts.
- Files that fail to parse are surfaced with a red error block &ndash; the
  rest of the registry stays loaded.

## Validation pack schema

```json
{
	"id": "acme-adt-rules",
	"name": "ACME ADT custom rules",
	"description": "Hospital-specific rules on top of HL7 v2 standard.",
	"author": "ACME Hospital",
	"version": "1.0",
	"enabled": true,
	"validation_rules": [
		{
			"rule_id": "ACME-PID-001",
			"severity": "error",
			"segment": "PID",
			"field": 3,
			"check": { "type": "not_empty" },
			"message": "PID-3 (Patient ID) is required"
		},
		{
			"rule_id": "ACME-PID-002",
			"severity": "warning",
			"segment": "PID",
			"field": 5,
			"component": 1,
			"check": { "type": "regex", "pattern": "^[A-Z][A-Z -]*$" },
			"message": "PID-5.1 (family name) must be uppercase letters"
		},
		{
			"rule_id": "ACME-PV1-001",
			"severity": "error",
			"segment": "PV1",
			"field": 2,
			"check": { "type": "one_of", "values": ["I", "O", "E"] },
			"message": "PV1-2 must be I, O or E"
		},
		{
			"rule_id": "ACME-PID-003",
			"severity": "warning",
			"segment": "PID",
			"field": 19,
			"check": { "type": "max_length", "max": 16 },
			"message": "PID-19 (SSN) is longer than 16 chars"
		}
	]
}
```

### Supported `check.type`

| Type | Params | Passes when |
|---|---|---|
| `not_empty` | &ndash; | field (or component) is not blank |
| `regex` | `pattern` | the regex matches the value |
| `max_length` | `max` | value bytes `<= max` |
| `min_length` | `min` | value bytes `>= min` |
| `one_of` | `values[]` | value exactly equals one of the listed values |
| `contains` | `value` | value contains the given substring |

### Severities

`error`, `warning`, `info` &ndash; same semantics as the built-in validator.
Issue counts in the Validation panel reflect the merged report.

### Component-level checks

Set `component` (1-based, `^`-separated) to narrow the check from the full
field to a single component, e.g. component 1 of `PID-5` (family name).

## Anonymization pack schema

```json
{
	"id": "eu-extra-phi",
	"name": "EU-specific PHI extensions",
	"description": "Additional PHI fields for EU deployments.",
	"author": "BridgeLab Community",
	"version": "1.0",
	"enabled": true,
	"phi_rules": [
		{ "segment": "PID", "field": 25, "sensitivity": "high",   "name": "EU National ID" },
		{ "segment": "ZPI", "field": 3,  "sensitivity": "medium", "name": "ACME internal ID" }
	]
}
```

### Sensitivity levels

| Level | Replacement strategy |
|---|---|
| `high` | text → `REDACTED`, numeric → `000…` of same length |
| `medium` | first char kept, rest masked (e.g. `J***`) |
| `low` | first three chars kept, rest replaced with `…` |

Plugin PHI rules merge with the built-in catalogue. Duplicates (same segment
+ field already known to the built-in list) are silently skipped, so you
never double-mask a value.

## Security notes

- **No code execution.** Plugin packs are pure data parsed with `serde_json`.
- **No network access.** The loader only reads files from the plugins folder.
- **Best-effort parsing.** A malformed file cannot break the registry &ndash;
  it surfaces as an `error` entry in the Plugins panel and is ignored by the
  validator / anonymizer.
- **User-scoped.** Plugins live under the user's config dir, so installing
  BridgeLab for another user on the same machine does not share them.

## Roadmap

1. ✅ Declarative validation + anonymization packs (this doc)
2. Sandboxed JS plugins (QuickJS) for transformations and computed validation
3. WASM plugins with a stable ABI for marketplace distribution
