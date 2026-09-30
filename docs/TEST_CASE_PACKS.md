# Test case packs

A test case pack is how BridgeLab shares test cases: *Export…* and
*Import…* in the Test Case Library (Ctrl+L) write and read one JSON
file, by convention named `*.bltests.json`. Send it to a colleague, put
it on a shared drive, or commit it to a Git repository next to the
interface it tests.

## Format

```json
{
  "format": "bridgelab-test-cases",
  "format_version": 1,
  "app_version": "2.0.0",
  "exported_at": "2026-09-29T10:00:00Z",
  "test_cases": [
    {
      "id": "0b6f0c1e-3f5e-4c1e-9d0a-2f1c0f3a9e11",
      "name": "ADT^A01 admission",
      "description": "Inpatient admission, ward 3",
      "category": "admission",
      "tags": "adt, regression",
      "content": "MSH|^~\\&|APP|FAC|RCV|RFAC|20260101120000||ADT^A01|MSG1|P|2.5\rPID|1||...",
      "expected_message_type": "ADT^A01",
      "expected_validation_result": "valid",
      "created_at": "2026-09-01T08:00:00Z",
      "updated_at": "2026-09-20T15:30:00Z"
    }
  ]
}
```

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | Always `bridgelab-test-cases` |
| `format_version` | yes | `1`. A newer version is refused with a message asking to update BridgeLab |
| `app_version`, `exported_at` | no | Shown in the import preview |
| `test_cases[].name`, `content` | yes | Content is the raw HL7 v2 message (segments separated by `\r`) or a FHIR resource as JSON or XML |
| `id` | no | Keeps a case recognisable across imports; a case without one always imports as new |
| `category` | no | Default `general` |
| `tags` | no | Comma-separated |
| `expected_message_type` | no | Only the components given are compared, case-insensitively: `ADT` matches any ADT event, `ADT^A01` matches `ADT^A01` and `ADT^A01^ADT_A01` (not `ADT^A04`), `ADT^A01^ADT_A01` needs all three; for FHIR, the `resourceType` |
| `expected_validation_result` | no | `valid` (default) or `invalid`, any case; any other value is refused when the pack is read |
| `created_at`, `updated_at` | no | RFC 3339 |

A hand-written pack needs only `format`, `format_version` and, per case,
`name` and `content`. An `id` may appear only once in a pack; a pack
that repeats one is refused.

## Importing

Nothing is written until you confirm the preview. For each case:

- **New**: no case with that id in the library; it is added.
- **Already in the library**: same id and same content, name, category,
  tags, description and expectations; skipped.
- **Differs**: same id, something changed. You choose *Keep mine*
  (default), *Replace with the imported one* (the original creation date
  is kept) or *Keep both* (the imported one gets a new id and
  "(imported)" after its name).

The import is written in one transaction. If the file changes between
the preview and *Import* (a colleague saves a new version on the shared
drive), the import is refused and asks you to open it again, so your
choices never land on cases you have not seen. In Community it may not take
the library past 10 test cases; if it would, nothing is written.

## Personal data

Test cases often start from real messages. Before exporting, BridgeLab
lists the HL7 v2 cases whose PHI fields (the built-in catalogue plus
active plugin packs) hold a value, and every FHIR case, which is not
checked field by field. With Pro, *Mask personal data* anonymizes the
HL7 v2 messages in the exported file only; the library keeps the
originals. FHIR content is never changed on export: review it before
sharing.

Masking changes values. Dates keep a valid shape and all-digit values
become zeros of the same length; other text is replaced (`REDACTED`, or
its first character and `***`, by sensitivity). A plugin rule that checks the format of a PHI
field (a social security number pattern on PID-19, say) can therefore
pass in the library and fail on the masked pack. Run the exported pack
once with `bridgelab-cli test` before relying on it in CI, or export
that case unmasked if its data may be shared.
