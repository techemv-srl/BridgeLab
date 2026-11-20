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
A reload is also triggered at every app startup. Packs load in the
background, so the window opens without waiting for them; a validation or
an anonymization started before they finish waits for them rather than run
without your rules.

## Manage plugins from the UI

`Settings → Plugins`:

- Lists every pack found, grouped by kind (`validation` / `fhir` /
  `anonymization`),
  with author, version, rule count, and the full on-disk path.
- Toggle individual packs on/off. The choice is saved in the plugins folder
  itself (`.state.json`), so it survives restarts and the
  [CLI](../tools/bridgelab-cli/README.md) honours it too. The switch belongs
  to one pack: a validation pack and an anonymization pack that happen to
  share an id are switched separately. If `.state.json` cannot be read (a
  hand edit gone wrong), Settings → Plugins and the CLI say so, each pack
  follows its own `"enabled"` until the file is fixed, and the file is not
  written over: the next switch saves new choices and keeps the old file
  as `.state.json.broken` (`.state.json.broken.1`, `.2`… when an earlier
  backup is there).
- Files that fail to load are surfaced with a red error block &ndash; the
  rest of the registry stays loaded. A second pack in the same folder with
  an id already in use is one of them: it is listed with the error, and none
  of its rules run until it gets its own id. Values the engine does not recognise (a
  misspelt severity or sensitivity, a pattern that does not compile or is
  slow to compile, FHIRPath that does not parse, a PHI field that cannot be
  masked, a segment name in lower case or not three letters or digits,
  field 0 or component 0, an empty `id` or `name`) load but are listed under
  the pack as warnings. The [CLI](../tools/bridgelab-cli/README.md) prints
  the same list on standard error.

## Tiers

Plugin packs are not a paid feature: every pack kind and every check type
runs in every tier, and so do reload and the per-pack toggles. The one
difference is the number of packs that can be **active at the same time**
&ndash; up to **3** in Community, unlimited in Pro and Enterprise. Enabling
a fourth pack in Community is refused with a message naming the limit; packs
that were enabled beyond the cap (during a trial, say) are neither locked nor
deleted &ndash; they show an "inactive" badge and contribute rules again the
moment another pack is disabled or the license is upgraded. Which packs
stay active is decided by the order they first appeared in the folder, so a
newly added file never displaces a pack already in use (the CLI, which
records nothing, goes by the files' creation times, as the app does for
packs it has not seen yet). A pack that holds no rules checks nothing and
takes no slot. Validation, FHIR and anonymization results each carry a
note when packs that would have added to them are left out. The in-app
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
| `max_length` | `max` | value length in characters `<= max` |
| `min_length` | `min` | value length in characters `>= min` |
| `one_of` | `values[]` | value exactly equals one of the listed values |
| `contains` | `value` | value contains the given substring |

Checks see a value as it reads, with its HL7 escapes resolved: `O\T\BRIEN`
is checked as `O&BRIEN`. A repeating field is checked one repetition at a
time. When the message has no segment of the rule's type, a `not_empty`
rule fires once (the field is missing too); every other check has no value
to disagree with and passes.

### Severities

`error`, `warning`, `info` &ndash; same semantics as the built-in validator.
Issue counts in the Validation panel reflect the merged report. Any other
value is reported as a warning and listed under the pack in Settings.

A `regex` rule whose pattern does not compile is not skipped: every
validation reports it, so a broken rule never looks like a passing one.

Each pattern is compiled once, when its pack loads, and reused by every
validation and by a Reload that leaves it unchanged; rules that spell the
same pattern share it. Most patterns compile in well under a millisecond,
but a Unicode class repeated many times does not: `^[\p{L} .-]{1,100}$`
compiles to about 4 MB, and `\w{300}` goes over the 10 MB a pattern may
take and does not compile at all. A pattern over 2 MB is listed as a
warning, since it slows every load of the pack and takes as much memory; a
shorter repetition, or an ASCII class such as `[A-Za-z0-9_]` in place of
`\w`, is far smaller.

### Segments and fields

`segment` is matched in any case and without surrounding spaces, on the
rule and in the message alike, as the anonymizer matches PHI fields: a rule
on `"pid"` checks PID. HL7 writes segment names as three capital letters or
digits, so a name written otherwise is listed as a warning. `field` counts
from 1; field 0 would be the segment name, and a rule on it checks an empty
value (and is listed).

### Component-level checks

Set `component` (1-based) to narrow the check from the full field to a
single component, e.g. component 1 of `PID-5` (family name). Components are
split with the message's own component separator (MSH-2), usually `^`.
Component 0 does not exist: it is read as component 1 and listed as a
warning.

### Repeating fields

A field with repetitions (`111^^^H^MR~222^^^H^PI`) is checked one repetition
at a time, and the rule passes only when every repetition does.

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

Other keys, in the pack or in a rule (`$schema`, an owner, a `comment`, a
ticket reference), are ignored by the validator and kept when the in-app
builder saves `user-rules.json`. The builder also refuses to save when the
file changed on disk after it was opened, so a rule added by hand or by a
`git pull` in the meantime is not overwritten.

### Supported `check.type` (FHIR)

| Type | Extra fields | Passes when |
|---|---|---|
| `not_empty` | &ndash; | at least one value, none of them blank |
| `cardinality` | `min`, `max` (either may be omitted) | the number of selected values is in range |
| `regex` | `pattern` | every value matches |
| `one_of` | `values` | every value is in the list |
| `contains` | `value` | every value contains the substring |
| `min_length` / `max_length` | `min` / `max` | every value is within the length bound |

Things worth knowing:

- **A Bundle is walked entry by entry.** A rule scoped to `Patient` fires for
  the Patients inside a transaction Bundle as well as for a standalone one,
  and the reported path is prefixed with `entry[n].resource.`.
- **A rule that fails to evaluate is reported, not skipped.** A typo in a
  FHIRPath expression surfaces as a finding that says so, because a rule that
  silently never runs is worse than one that complains.
- **Dates stay between the years 1 and 9999.** A date or date-time moved
  past them (`birthDate + 100000000 days`) is empty, as an integer that
  overflows is, so a comparison with it is empty too and an invariant
  built on it fails.
- **An expression has size limits**, far above what a rule needs, so that
  a runaway one cannot take the app down. It nests at most 64 levels deep
  and joins at most 4096 terms in one chain of operators. One string it
  builds (`replace()`, `replaceMatches()`, `join()`, `&`, `+`, `encode()`,
  `escape()`, `upper()`, `lower()`) holds at most 16 MB. All the values one
  evaluation builds (strings, and what `select()`, `repeat()`,
  `aggregate()`, `combine()`, `union()`, `descendants()`, `resolve()`,
  `split()`, `toChars()` and `trace()` return) come to at most about
  512 MB; `descendants()` of a
  10 MB Bundle of Observations counts about 115 MB. One evaluation does at
  most 50 million steps of work and runs at most 60 seconds. A step is one
  part of the expression evaluated, one value a path reads, one item a
  `where()`, `select()`, `all()` or `exists()` checks, one comparison made
  by `distinct()`, `isDistinct()`, `|`, `in` or `subsetOf()`, or a quarter
  KB of the resource copied whole (`%resource` on its own, `$this` outside
  a criteria). A real rule takes a few hundred steps; `isDistinct()` over
  5000 values takes about 12 million, and criteria nested in criteria
  multiply: `%resource.name.where(%resource.name.where(…).exists())` over
  ten names is ten times the work at each level. Past a limit the rule
  is reported as failing to evaluate, with the limit in the message.
- **One validation's rules share a budget** of twice that: 100 million
  steps and 120 seconds, for every rule on the resource and, in a Bundle,
  on every entry. Once it is spent the remaining rules are not run, and one
  error says how many checks were left out, so the result never looks
  complete when it is not. `bridgelab-cli validate` applies the same
  limits to each file.

Every check except `not_empty` and `cardinality` is satisfied by a selector
that returns nothing &ndash; there is no value to disagree with. Pair the two
when a field must both exist and look right.

### Building rules in the app

**Tools → FHIR validation rules…** opens an editor that writes
`plugins/fhir/user-rules.json`. It validates the FHIRPath before saving and
can run a rule against the resource currently open, showing which values the
selector picked up; if the budget runs out before every value is listed, the
test says so and does not pass. Hand-written packs in the same folder load
alongside it.

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
		{ "segment": "ZPI", "field": 2,  "sensitivity": "high",   "name": "National ID" },
		{ "segment": "ZPI", "field": 3,  "sensitivity": "medium", "name": "ACME internal ID" }
	]
}
```

### Sensitivity levels

| Level | Replacement strategy |
|---|---|
| `high` | text → `REDACTED`, numeric → `000…` of the same length, date → `19000101…` of the same precision; also any value that is not one of the three levels |
| `medium` | first char kept, rest masked (e.g. `J***`) |
| `low` | first three chars kept, rest replaced with `...` (three full stops, plain ASCII, e.g. `Zoë...`); a value of three characters or fewer is kept whole |

Plugin PHI rules merge with the built-in catalogue. A field the built-in
list already covers is skipped (a rule on OBX-5 replaces the built-in
free-text one and masks every OBX, whatever its value type), and a field named by more than one pack is
masked once, at the strongest level any of them asks for, so no value is
ever masked twice. Every repetition, component and subcomponent is masked on
its own with the message's delimiters, so the output keeps the structure of
the input. Field 0 (the segment name) and MSH-1/MSH-2 (the delimiters) are
not data and are ignored (and listed as warnings). As for validation rules,
`segment` is matched in any case: `"zpi"` masks ZPI, and is listed as a
warning so the pack can be corrected.

PHI fields count wherever the pack sits: `phi_rules` in a pack under
`validation/` are masked too, not dropped.

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
  validator / anonymizer. A UTF-8 byte-order mark is accepted, and so is a
  `.JSON` extension in capitals. Anything that is not a regular file (a named
  pipe, a device) or is larger than 4 MB is reported instead of read. Packs
  load off the main thread, at startup and on Reload, so even a pack of
  slow patterns cannot freeze the window. Two packs in the same folder with
  the same `id` are refused: the second is reported.
- **User-scoped.** Plugins live under the user's config dir, so installing
  BridgeLab for another user on the same machine does not share them.

## Roadmap

1. ✅ Declarative HL7 v2, FHIR and anonymization packs (this doc)
2. Sandboxed JS plugins (QuickJS) for transformations and computed validation
3. WASM plugins with a stable ABI for marketplace distribution
