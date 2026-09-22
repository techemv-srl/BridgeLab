import type * as MonacoTypes from 'monaco-editor';
import { getSegmentInfo, getHl7Table, type FieldDef, type ValueTable } from '$lib/ipc/tables';

/** Common HL7 segment types for quick suggestion */
const COMMON_SEGMENTS = [
	{ code: 'MSH', desc: 'Message Header' },
	{ code: 'EVN', desc: 'Event Type' },
	{ code: 'PID', desc: 'Patient Identification' },
	{ code: 'PD1', desc: 'Patient Additional Demographics' },
	{ code: 'NK1', desc: 'Next of Kin' },
	{ code: 'PV1', desc: 'Patient Visit' },
	{ code: 'PV2', desc: 'Patient Visit - Additional' },
	{ code: 'DG1', desc: 'Diagnosis' },
	{ code: 'AL1', desc: 'Patient Allergy Information' },
	{ code: 'OBR', desc: 'Observation Request' },
	{ code: 'OBX', desc: 'Observation/Result' },
	{ code: 'ORC', desc: 'Common Order' },
	{ code: 'RXA', desc: 'Pharmacy Administration' },
	{ code: 'RXE', desc: 'Pharmacy Encoded Order' },
	{ code: 'IN1', desc: 'Insurance' },
	{ code: 'GT1', desc: 'Guarantor' },
	{ code: 'MSA', desc: 'Message Acknowledgment' },
	{ code: 'ERR', desc: 'Error' },
	{ code: 'TXA', desc: 'Transcription Document Header' },
	{ code: 'SCH', desc: 'Scheduling Activity Information' },
	{ code: 'AIS', desc: 'Appointment Information - Service' },
	{ code: 'AIL', desc: 'Appointment Information - Location' },
	{ code: 'AIP', desc: 'Appointment Information - Personnel' },
];

/** Common message types */
const MESSAGE_TYPES = [
	'ADT^A01', 'ADT^A02', 'ADT^A03', 'ADT^A04', 'ADT^A05', 'ADT^A06', 'ADT^A07', 'ADT^A08',
	'ORU^R01', 'ORU^R30', 'ORM^O01', 'OMG^O19', 'OML^O21', 'OMI^O23',
	'SIU^S12', 'SIU^S13', 'SIU^S14', 'SIU^S15',
	'MDM^T01', 'MDM^T02', 'MDM^T04', 'MDM^T06', 'MDM^T08', 'MDM^T11',
	'ACK', 'DFT^P03', 'QBP^Q11', 'RSP^K11',
];

/** Versions the backend ships definitions for (see Hl7Version::ALL). */
const KNOWN_VERSIONS = ['2.1', '2.2', '2.3', '2.3.1', '2.4', '2.5', '2.5.1', '2.6', '2.7', '2.7.1'];

/** Used when the message carries no usable MSH-12. */
const DEFAULT_VERSION = '2.5';

/**
 * HL7 version of the message currently in the editor, read from MSH-12.
 *
 * The completion and hover providers only get a Monaco model, not the parse
 * result, so the version is recovered from the text itself — otherwise every
 * message would be described against v2.5 tables regardless of what it
 * declares. MSH-1 is the field separator and MSH-2 the encoding characters,
 * so splitting the header on '|' puts MSH-n at index n-1 for n >= 2; MSH-12
 * may carry components (`2.5.1^...`), of which only the first matters.
 */
export function versionFromModel(model: MonacoTypes.editor.ITextModel): string {
	// Scan a bounded prefix: MSH is the first segment of a well-formed
	// message, and batch files put it just after FHS/BHS.
	const maxLines = Math.min(model.getLineCount(), 5);
	for (let line = 1; line <= maxLines; line++) {
		const content = model.getLineContent(line);
		if (!content.startsWith('MSH') || content.length < 4) continue;
		// MSH-1 is the field separator itself, so the header splits on
		// whatever character follows "MSH"; the version number is the
		// leading digits-and-dots of MSH-12, whatever the VID's own
		// component separator.
		const declared = content.split(content[3])[11]?.match(/^\s*[\d.]+/)?.[0]?.trim();
		return declared && KNOWN_VERSIONS.includes(declared) ? declared : DEFAULT_VERSION;
	}
	return DEFAULT_VERSION;
}

/**
 * The HL7 value table behind a field, when it has one. Tables are small
 * and few, so a per-session cache keeps completion and hover free of
 * repeated IPC round-trips for the same field.
 */
const tableCache = new Map<string, Promise<ValueTable | null>>();
function tableFor(field: FieldDef): Promise<ValueTable | null> {
	if (!field.table_id) return Promise.resolve(null);
	// The table is closed or open by the type of the element it belongs
	// to: MSH-9 is MSG, its 0076 is MSG.1's, an ID.
	const type = field.table_data_type ?? field.data_type;
	const key = `${field.table_id}|${type}`;
	let p = tableCache.get(key);
	if (!p) {
		p = getHl7Table(field.table_id, type).catch(() => null);
		tableCache.set(key, p);
	}
	return p;
}

/** Occurrences of the field separator in `text` — the field position the
 *  cursor sits in, for a segment that is not MSH. */
function countFieldSeparators(text: string, sep: string): number {
	let n = 0;
	for (const c of text) if (c === sep) n++;
	return n;
}

/** The message's delimiters, read from MSH-1/MSH-2 of the model: `|^~\&`
 *  by default, but a message may declare its own and then `^` is an
 *  ordinary character. Same bounded scan as `versionFromModel`. */
export function delimitersFromModel(model: MonacoTypes.editor.ITextModel): { field: string; component: string; repetition: string; subcomponent: string } {
	const std = { field: '|', component: '^', repetition: '~', subcomponent: '&' };
	const maxLines = Math.min(model.getLineCount(), 5);
	for (let line = 1; line <= maxLines; line++) {
		const content = model.getLineContent(line);
		if (!content.startsWith('MSH') || content.length < 8) continue;
		// MSH|^~\&|: field separator at index 3, then component, repetition,
		// escape, subcomponent.
		return { field: content[3], component: content[4], repetition: content[5], subcomponent: content[7] };
	}
	return std;
}

/** The code a field value is looked up by: its first component, first repetition. */
function leadingCode(value: string, d: ReturnType<typeof delimitersFromModel>): string {
	const cut = [d.component, d.repetition, d.subcomponent]
		.map((sep) => value.indexOf(sep))
		.filter((i) => i >= 0);
	return (cut.length ? value.slice(0, Math.min(...cut)) : value).trim();
}

/**
 * Register a completion provider for HL7 v2 language.
 */
export function registerHL7AutoComplete(monaco: typeof MonacoTypes) {
	monaco.languages.registerCompletionItemProvider('hl7v2', {
		triggerCharacters: ['|', '^', '&', '~', '\n', '\r'],
		provideCompletionItems: async (model, position) => {
			const lineContent = model.getLineContent(position.lineNumber);
			const column = position.column;
			const textBefore = lineContent.substring(0, column - 1);

			const range = {
				startLineNumber: position.lineNumber,
				endLineNumber: position.lineNumber,
				startColumn: column,
				endColumn: column,
			};

			const suggestions: MonacoTypes.languages.CompletionItem[] = [];

			// If at the start of a line, suggest segment types
			if (textBefore.length <= 3 && !textBefore.includes(delimitersFromModel(model).field)) {
				for (const seg of COMMON_SEGMENTS) {
					suggestions.push({
						label: seg.code,
						kind: monaco.languages.CompletionItemKind.Class,
						detail: seg.desc,
						documentation: `HL7 segment: ${seg.desc}`,
						insertText: seg.code + '|',
						range,
						sortText: '0' + seg.code,
					});
				}
				return { suggestions };
			}

			// Detect segment type from start of line
			const segMatch = lineContent.match(/^([A-Z][A-Z0-9]{2})/);
			if (!segMatch) return { suggestions: [] };
			const segmentType = segMatch[1];

			// Count field separators to determine the field position — the
			// message's own separator, which is "|" unless MSH-1 says otherwise.
			const delims = delimitersFromModel(model);
			const pipeCount = countFieldSeparators(textBefore, delims.field);
			const isMsh = segmentType === 'MSH';
			const fieldPosition = isMsh ? pipeCount + 1 : pipeCount;

			// Load segment info from backend
			try {
				const info = await getSegmentInfo(segmentType, versionFromModel(model));
				if (info) {
					// Suggest completion based on field position
					const field = info.fields.find(f => f.position === fieldPosition);
					if (field) {
						// Add field-specific suggestions
						if (segmentType === 'MSH' && fieldPosition === 9) {
							// Message Type field: the common type^event pairs, more
							// useful here than the 128 bare codes of table 0076.
							for (const mt of MESSAGE_TYPES) {
								suggestions.push({
									label: mt,
									kind: monaco.languages.CompletionItemKind.Enum,
									detail: 'HL7 Message Type',
									insertText: mt,
									range,
								});
							}
						} else {
							// Any coded field: every value of its HL7 table (MSA-1
							// acknowledgment codes, PID-8 sex, OBX-11 result status…).
							const table = await tableFor(field);
							for (const tv of table?.values ?? []) {
								suggestions.push({
									label: tv.code,
									kind: monaco.languages.CompletionItemKind.EnumMember,
									detail: tv.description,
									documentation: `HL7 table ${table!.id} — ${table!.name}`,
									insertText: tv.code,
									range,
								});
							}
						}

						// Always show field info as a snippet hint
						suggestions.push({
							label: `[${segmentType}-${fieldPosition}] ${field.name}`,
							kind: monaco.languages.CompletionItemKind.Field,
							detail: `${field.data_type}${field.required ? ' (required)' : ''}${field.max_length ? ' max ' + field.max_length : ''}`,
							documentation: field.description,
							insertText: '',
							range,
							sortText: '9',
						});
					}
				}
			} catch {
				// Running in web mode or backend not available
			}

			return { suggestions };
		},
	});

	// Hover provider - show field info when hovering
	monaco.languages.registerHoverProvider('hl7v2', {
		provideHover: async (model, position) => {
			const lineContent = model.getLineContent(position.lineNumber);
			const segMatch = lineContent.match(/^([A-Z][A-Z0-9]{2})/);
			if (!segMatch) return null;

			const segmentType = segMatch[1];
			const column = position.column;
			const textBefore = lineContent.substring(0, column - 1);
			const delims = delimitersFromModel(model);
			const pipeCount = countFieldSeparators(textBefore, delims.field);
			const isMsh = segmentType === 'MSH';
			const fieldPosition = isMsh ? pipeCount + 1 : pipeCount;

			if (fieldPosition < 1) return null;

			try {
				const info = await getSegmentInfo(segmentType, versionFromModel(model));
				if (info) {
					const field = info.fields.find(f => f.position === fieldPosition);
					if (field) {
						const contents = [
							{ value: `**${segmentType}-${fieldPosition}**: ${field.name}` },
							{ value: `Type: \`${field.data_type}\`${field.required ? ' · **required**' : ''}${field.max_length ? ' · max ' + field.max_length : ''}` },
							{ value: field.description },
						];
						// A coded value is explained from its HL7 table; one the
						// closed table does not list is called out.
						const table = await tableFor(field);
						if (table) {
							// MSH-1 is the separator itself; for n >= 2 the value sits at index n-1.
							const raw = lineContent.split(delims.field)[isMsh ? fieldPosition - 1 : fieldPosition] ?? '';
							const code = leadingCode(raw, delims);
							const hit = code ? table.values.find((tv) => tv.code === code) : undefined;
							if (hit) {
								contents.push({ value: `\`${hit.code}\` — ${hit.description} *(HL7 table ${table.id})*` });
							} else if (code && table.exhaustive) {
								contents.push({ value: `\`${code}\` is not in HL7 table ${table.id} (${table.name})` });
							}
						}
						return { contents };
					}
				}
			} catch {
				// ignore
			}
			return null;
		},
	});
}
