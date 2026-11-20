import { baseName } from '$lib/paths';
import { t } from '$lib/i18n';

/** The kind of file a tab's text is saved as. */
export interface SaveFormat {
	/** The file's extension, without the dot. */
	ext: 'hl7' | 'json' | 'xml';
	/** The save dialog's filter for it: a name the same in every language
	 *  ("FHIR JSON"), or the key of a translated one. */
	filter: string;
}

const HL7: SaveFormat = { ext: 'hl7', filter: 'filter.hl7' };
const FHIR_JSON: SaveFormat = { ext: 'json', filter: 'FHIR JSON' };
const FHIR_XML: SaveFormat = { ext: 'xml', filter: 'FHIR XML' };

/**
 * What a tab's text is saved as: the format it parsed as (`format`, from
 * the parse result: "HL7v2", "FHIR JSON", "FHIR XML"), or, for text that
 * has not parsed, what it starts with: JSON, XML, or else an HL7 message.
 */
export function saveFormatOf(format: string | null | undefined, content: string): SaveFormat {
	if (format?.startsWith('FHIR JSON')) return FHIR_JSON;
	if (format?.startsWith('FHIR XML')) return FHIR_XML;
	if (format) return HL7;
	const head = content.replace(/^﻿/, '').trimStart();
	if (head.startsWith('{') || head.startsWith('[')) return FHIR_JSON;
	if (head.startsWith('<')) return FHIR_XML;
	return HL7;
}

/**
 * Whether the file name at the end of `path` has an extension: a last dot
 * followed by up to ten letters and digits, at least one of them a letter.
 * A version or a date at the end of a name ("ADT^A01 v2.5", "batch
 * 2024.11") is part of the name, not a file type.
 */
export function hasExtension(path: string): boolean {
	const m = /\.([A-Za-z0-9]{1,10})$/.exec(baseName(path));
	return !!m && m.index > 0 && /[A-Za-z]/.test(m[1]);
}

/** A tab label as a file name: characters no file name may hold on
 *  Windows (a path separator anywhere) become `_`. */
function fileNameOf(label: string): string {
	return label.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '_').replace(/[. ]+$/, '').trim();
}

/** What the tab's Save As needs, from the parts of a tab it reads. */
export interface SaveAsTab {
	filePath: string | null;
	label: string;
	content: string;
	parseResult: { format: string } | null;
	parseStale?: string | null;
}

/**
 * The save dialog's options for Save As of `tab`, and its format.
 *
 * The format's filter comes first: Windows adds its extension to a name
 * typed without one, and macOS offers it first. "All Files" stays, for a
 * name with another extension. The default is the tab's file, or for a
 * new tab its label with the format's extension.
 */
export function saveAsDialog(tab: SaveAsTab): {
	format: SaveFormat;
	defaultPath: string;
	filters: { name: string; extensions: string[] }[];
} {
	// A parse result for an earlier text says nothing about this one.
	const format = saveFormatOf(tab.parseStale ? null : tab.parseResult?.format, tab.content);
	let defaultPath = tab.filePath;
	if (!defaultPath) {
		const name = fileNameOf(tab.label) || fileNameOf(t('tab.untitled'));
		defaultPath = hasExtension(name) ? name : `${name}.${format.ext}`;
	}
	return {
		format,
		defaultPath,
		// In the app's language, as the rest of the dialog (the system's).
		filters: [
			{ name: t(format.filter), extensions: [format.ext] },
			{ name: t('filter.all'), extensions: ['*'] },
		],
	};
}
