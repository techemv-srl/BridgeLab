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
│   └── *.json     <- extra HL7 v2 validation rules
├── fhir/
│   └── *.json     <- extra FHIR validation rules
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

- Lists every pack found, grouped by kind (`validation` / `fhir` /
  `anonymization`),
  with author, version, rule count, and the full on-disk path.
- Toggle individual packs on/off; the preference is persisted so the choice
  survives restarts.
- Files that fail to parse are surfaced with a red error block &ndash; the
  rest of the registry stays loaded.

## Tiers

Plugin packs are not a paid feature: every pack kind and every check type
runs in every tier, and so do reload and the per-pack toggles. The one
difference is the number of packs that can be **active at the same time**
&ndash; up to **3** in Community, unlimited in Pro and Enterprise. Enabling
a fourth pack in Community is refused with an upgrade prompt; packs that
were enabled beyond the cap (during a trial, say) are neither locked nor
deleted &ndash; they show an "inactive" badge and contribute rules again the
moment another pack is disabled or the license is upgraded. The in-app
editor for FHIR packs (below) is the only plugin-related feature that is
Pro by itself.

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

## FHIR pack schema

Files in `fhir/` carry `fhir_rules`. Each rule takes one of two shapes.

**Invariant** &ndash; a FHIRPath expression that must evaluate to `true`, the
way FHIR writes its own constraints:

```json
{
	"id": "acme-fhir-rules",
	"name": "ACME FHIR rules",
	"version": "1.0",
	"enabled": true,
	"fhir_rules": [
		{
			"rule_id": "patient-has-identifier",
			"severity": "error",
			"resource": "Patient",
			"expression": "identifier.exists()",
			"message": "Patient must carry at least one identifier"
		}
	]
}
```

**Selector plus check** &ndash; a FHIRPath expression picking the values, and
a check applied to each of them. This is what the in-app builder writes:

```json
{
	"rule_id": "patient-phone-format",
	"severity": "warning",
	"resource": "Patient",
	"path": "telecom.where(system = 'phone').value",
	"check": { "type": "regex", "pattern": "^[+0-9 ()./-]{6,}$" },
	"message": "Phone number contains unexpected characters"
}
```

| Field | Meaning |
|---|---|
| `rule_id` | Identifier shown in the validation panel. Required. |
| `severity` | `error`, `warning` or `info`. Default `warning`. |
| `resource` | Resource type the rule applies to. Omit to apply it to every resource. |
| `expression` | Invariant form. Mutually exclusive with `path`. |
| `path` | Selector form. Requires `check`. |
| `check` | What to assert about each selected value. |
| `message` | Text emitted when the rule fires. Required. |

### Supported `check.type` (FHIR)

| Type | Extra fields | Passes when |
|---|---|---|
| `not_empty` | &ndash; | at least one value, none of them blank |
| `cardinality` | `min`, `max` (either may be omitted) | the number of selected values is in range |
| `regex` | `pattern` | every value matches |
| `one_of` | `values` | every value is in the list |
| `contains` | `value` | every value contains the substring |
| `min_length` / `max_length` | `min` / `max` | every value is within the length bound |

Two things worth knowing:

- **A Bundle is walked entry by entry.** A rule scoped to `Patient` fires for
  the Patients inside a transaction Bundle as well as for a standalone one,
  and the reported path is prefixed with `entry[n].resource.`.
- **A rule that fails to evaluate is reported, not skipped.** A typo in a
  FHIRPath expression surfaces as a finding that says so, because a rule that
  silently never runs is worse than one that complains.

Every check except `not_empty` and `cardinality` is satisfied by a selector
that returns nothing &ndash; there is no value to disagree with. Pair the two
when a field must both exist and look right.

### Building rules in the app

**Tools → FHIR validation rules…** opens an editor that writes
`plugins/fhir/user-rules.json`. It validates the FHIRPath before saving and
can run a rule against the resource currently open, showing which values the
selector picked up. Hand-written packs in the same folder load alongside it.

The editor requires a Professional license; rules themselves run in every
tier under the same plugin-pack cap as HL7 v2 packs.

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

## Not the same thing: FHIR profile packages

Plugin packs are **your** rules, written as JSON, living under
`<config_dir>/BridgeLab/plugins/`.

FHIR **profile packages** are a different mechanism: published FHIR NPM
packages (`hl7.fhir.r4.core`, a national IG, your site's own profiles)
that BridgeLab validates resources against structurally &ndash;
cardinality, element types, choice elements, fixed values, unknown
elements. They live under `<config_dir>/BridgeLab/fhir-packages/` and are
installed from **Tools &rarr; FHIR profile packages&hellip;**.

Use a profile package when the rule you want is already written down in a
StructureDefinition; use a `fhir/` plugin pack when it is your own
convention and nobody has published it.

## Security notes

- **No code execution.** Plugin packs are pure data parsed with `serde_json`.
- **No network access.** The loader only reads files from the plugins folder.
- **Best-effort parsing.** A malformed file cannot break the registry &ndash;
  it surfaces as an `error` entry in the Plugins panel and is ignored by the
  validator / anonymizer.
- **User-scoped.** Plugins live under the user's config dir, so installing
  BridgeLab for another user on the same machine does not share them.

## Roadmap

1. ✅ Declarative HL7 v2, FHIR and anonymization packs (this doc)
2. Sandboxed JS plugins (QuickJS) for transformations and computed validation
3. WASM plugins with a stable ABI for marketplace distribution
