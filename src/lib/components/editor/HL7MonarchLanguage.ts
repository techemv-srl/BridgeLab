import type * as Monaco from 'monaco-editor';

/** Register HL7 v2.x language definition for Monaco Editor */
export function registerHL7Language(monaco: typeof Monaco) {
	// Register the language
	monaco.languages.register({ id: 'hl7v2' });

	// Define tokenizer rules
	monaco.languages.setMonarchTokensProvider('hl7v2', {
		tokenizer: {
			root: [
				// Segment types at start of line (MSH, PID, OBX, etc.)
				[/^[A-Z][A-Z0-9]{2}/, 'hl7-segment'],

				// Z-segments (custom segments)
				[/^Z[A-Z0-9]{2}/, 'hl7-zsegment'],

				// Field separator
				[/\|/, 'hl7-delimiter-field'],

				// Component separator
				[/\^/, 'hl7-delimiter-component'],

				// Subcomponent separator
				[/&/, 'hl7-delimiter-subcomponent'],

				// Repetition separator
				[/~/, 'hl7-delimiter-repetition'],

				// Escape sequences
				[/\\[FSETRL\\]\\/, 'hl7-escape'],
				[/\\.br\\/, 'hl7-escape'],

				// Truncation marker
				[/\{\.\.\.[\d]+ bytes\}/, 'hl7-truncated'],

				// Dates (common HL7 format)
				[/\d{8}(\d{4,6})?/, 'hl7-date'],

				// Numbers
				[/\d+(\.\d+)?/, 'hl7-number'],

				// Regular text
				[/[^\|^&~\\{}\r\n]+/, 'hl7-text'],
			],
		},
	});

	// Define theme rules for dark theme
	monaco.editor.defineTheme('bridgelab-dark', {
		base: 'vs-dark',
		inherit: true,
		rules: [
			{ token: 'hl7-segment', foreground: 'cba6f7', fontStyle: 'bold' },
			{ token: 'hl7-zsegment', foreground: 'f5c2e7', fontStyle: 'bold' },
			{ token: 'hl7-delimiter-field', foreground: '6c7086' },
			{ token: 'hl7-delimiter-component', foreground: '585b70' },
			{ token: 'hl7-delimiter-subcomponent', foreground: '45475a' },
			{ token: 'hl7-delimiter-repetition', foreground: 'f9e2af' },
			{ token: 'hl7-escape', foreground: 'fab387' },
			{ token: 'hl7-truncated', foreground: 'f38ba8', fontStyle: 'italic' },
			{ token: 'hl7-date', foreground: '89b4fa' },
			{ token: 'hl7-number', foreground: 'a6e3a1' },
			{ token: 'hl7-text', foreground: 'cdd6f4' },
		],
		colors: {
			'editor.background': '#1e1e2e',
			'editor.foreground': '#cdd6f4',
			'editor.lineHighlightBackground': '#313244',
			'editor.selectionBackground': '#585b7066',
			'editorCursor.foreground': '#f5e0dc',
			'editorLineNumber.foreground': '#6c7086',
			'editorLineNumber.activeForeground': '#cdd6f4',
		},
	});

	// Define theme rules for light theme
	monaco.editor.defineTheme('bridgelab-light', {
		base: 'vs',
		inherit: true,
		rules: [
			{ token: 'hl7-segment', foreground: '8839ef', fontStyle: 'bold' },
			{ token: 'hl7-zsegment', foreground: 'ea76cb', fontStyle: 'bold' },
			{ token: 'hl7-delimiter-field', foreground: '9ca0b0' },
			{ token: 'hl7-delimiter-component', foreground: '8c8fa1' },
			{ token: 'hl7-delimiter-subcomponent', foreground: '7c7f93' },
			{ token: 'hl7-delimiter-repetition', foreground: 'df8e1d' },
			{ token: 'hl7-escape', foreground: 'fe640b' },
			{ token: 'hl7-truncated', foreground: 'd20f39', fontStyle: 'italic' },
			{ token: 'hl7-date', foreground: '1e66f5' },
			{ token: 'hl7-number', foreground: '40a02b' },
			{ token: 'hl7-text', foreground: '4c4f69' },
		],
		colors: {
			'editor.background': '#eff1f5',
			'editor.foreground': '#4c4f69',
			'editor.lineHighlightBackground': '#e6e9ef',
			'editor.selectionBackground': '#9ca0b066',
			'editorCursor.foreground': '#dc8a78',
			'editorLineNumber.foreground': '#9ca0b0',
			'editorLineNumber.activeForeground': '#4c4f69',
		},
	});
}
