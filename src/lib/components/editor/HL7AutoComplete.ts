import type * as MonacoTypes from 'monaco-editor';
import { getSegmentInfo } from '$lib/ipc/tables';

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

/** Common ACK codes */
const ACK_CODES = [
	{ code: 'AA', desc: 'Application Accept' },
	{ code: 'AE', desc: 'Application Error' },
	{ code: 'AR', desc: 'Application Reject' },
	{ code: 'CA', desc: 'Commit Accept' },
	{ code: 'CE', desc: 'Commit Error' },
	{ code: 'CR', desc: 'Commit Reject' },
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
		if (!content.startsWith('MSH')) continue;
		const declared = content.split('|')[11]?.split('^')[0]?.trim();
		return declared && KNOWN_VERSIONS.includes(declared) ? declared : DEFAULT_VERSION;
	}
	return DEFAULT_VERSION;
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
			if (textBefore.length <= 3 && !textBefore.includes('|')) {
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

			// Count pipes to determine field position
			const pipeCount = (textBefore.match(/\|/g) || []).length;
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
							// Message Type field
							for (const mt of MESSAGE_TYPES) {
								suggestions.push({
									label: mt,
									kind: monaco.languages.CompletionItemKind.Enum,
									detail: 'HL7 Message Type',
									insertText: mt,
									range,
								});
							}
						} else if (segmentType === 'MSA' && fieldPosition === 1) {
							// ACK code
							for (const ack of ACK_CODES) {
								suggestions.push({
									label: ack.code,
									kind: monaco.languages.CompletionItemKind.EnumMember,
									detail: ack.desc,
									insertText: ack.code,
									range,
								});
							}
						} else if (segmentType === 'PID' && fieldPosition === 8) {
							// Gender
							for (const g of [
								{ v: 'M', d: 'Male' },
								{ v: 'F', d: 'Female' },
								{ v: 'O', d: 'Other' },
								{ v: 'U', d: 'Unknown' },
								{ v: 'A', d: 'Ambiguous' },
							]) {
								suggestions.push({
									label: g.v,
									kind: monaco.languages.CompletionItemKind.EnumMember,
									detail: g.d,
									insertText: g.v,
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
			const pipeCount = (textBefore.match(/\|/g) || []).length;
			const isMsh = segmentType === 'MSH';
			const fieldPosition = isMsh ? pipeCount + 1 : pipeCount;

			if (fieldPosition < 1) return null;

			try {
				const info = await getSegmentInfo(segmentType, versionFromModel(model));
				if (info) {
					const field = info.fields.find(f => f.position === fieldPosition);
					if (field) {
						return {
							contents: [
								{ value: `**${segmentType}-${fieldPosition}**: ${field.name}` },
								{ value: `Type: \`${field.data_type}\`${field.required ? ' · **required**' : ''}${field.max_length ? ' · max ' + field.max_length : ''}` },
								{ value: field.description },
							],
						};
					}
				}
			} catch {
				// ignore
			}
			return null;
		},
	});
}
