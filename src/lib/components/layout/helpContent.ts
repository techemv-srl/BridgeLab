/**
 * Generates a standalone HTML document for the BridgeLab user manual.
 * Opened in a separate OS window via window.open() so the user can move,
 * resize, minimize it independently from the main application.
 */

import { enSections } from './help/en';
import { itSections } from './help/it';

export interface ManualSection {
	id: string;
	heading: string;
	body: string; // HTML string (already sanitized / authored by us)
}

const TITLES: Record<string, string> = {
	en: 'BridgeLab User Manual',
	it: 'Manuale Utente BridgeLab',
	fr: 'Manuel Utilisateur BridgeLab',
	es: 'Manual de Usuario BridgeLab',
	de: 'BridgeLab Benutzerhandbuch',
};

const CONTENTS_LABEL: Record<string, string> = {
	en: 'Contents',
	it: 'Indice',
	fr: 'Sommaire',
	es: 'Índice',
	de: 'Inhalt',
};

function getSections(locale: string): ManualSection[] {
	switch (locale) {
		case 'it': return itSections;
		default:   return enSections;
	}
}

export function generateManualHtml(locale: string): string {
	const title = TITLES[locale] ?? TITLES.en;
	const contents = CONTENTS_LABEL[locale] ?? CONTENTS_LABEL.en;
	const sections = getSections(locale);

	const toc = sections.map(s =>
		`<li><a href="#${s.id}">${s.heading}</a></li>`
	).join('');

	const body = sections.map(s =>
		`<section id="${s.id}"><h2>${s.heading}</h2>${s.body}</section>`
	).join('');

	return `<!doctype html>
<html lang="${locale}">
<head>
<meta charset="utf-8" />
<title>${title}</title>
<style>${STYLES}</style>
</head>
<body>
<div class="wrap">
	<aside class="toc">
		<h1>${title}</h1>
		<h3>${contents}</h3>
		<ul>${toc}</ul>
	</aside>
	<main class="content">${body}</main>
</div>
</body>
</html>`;
}

const STYLES = `
* { box-sizing: border-box; }
body {
	margin: 0;
	font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
	background: #1e1e2e;
	color: #cdd6f4;
	font-size: 14px;
	line-height: 1.6;
}
.wrap { display: flex; min-height: 100vh; }
.toc {
	width: 240px;
	flex-shrink: 0;
	padding: 20px 16px;
	background: #181825;
	border-right: 1px solid #313244;
	position: sticky;
	top: 0;
	height: 100vh;
	overflow-y: auto;
}
.toc h1 {
	font-size: 16px;
	margin: 0 0 20px;
	color: #89b4fa;
	letter-spacing: -0.01em;
}
.toc h3 {
	font-size: 10px;
	text-transform: uppercase;
	letter-spacing: 0.1em;
	color: #a6adc8;
	margin: 0 0 8px;
}
.toc ul { list-style: none; padding: 0; margin: 0; }
.toc li { margin: 2px 0; }
.toc a {
	display: block;
	padding: 4px 8px;
	color: #cdd6f4;
	text-decoration: none;
	font-size: 13px;
	border-radius: 3px;
	border-left: 2px solid transparent;
}
.toc a:hover { background: #313244; border-left-color: #89b4fa; }
.content {
	flex: 1;
	padding: 32px 48px;
	max-width: 820px;
}
h2 {
	color: #89b4fa;
	font-size: 22px;
	font-weight: 700;
	margin: 32px 0 12px;
	padding-bottom: 8px;
	border-bottom: 1px solid #313244;
	letter-spacing: -0.01em;
}
section:first-child h2 { margin-top: 0; }
h3 {
	color: #cba6f7;
	font-size: 15px;
	margin: 20px 0 8px;
	font-weight: 600;
}
p { margin: 8px 0; }
code {
	font-family: 'Consolas', 'Monaco', monospace;
	background: #313244;
	padding: 1px 6px;
	border-radius: 3px;
	font-size: 12px;
	color: #f9e2af;
}
kbd {
	display: inline-block;
	padding: 2px 8px;
	border: 1px solid #45475a;
	border-bottom-width: 2px;
	border-radius: 4px;
	background: #313244;
	font-family: monospace;
	font-size: 11px;
	color: #cdd6f4;
}
ul, ol { margin: 8px 0; padding-left: 22px; }
li { margin: 3px 0; }
strong { color: #f9e2af; }
table { width: 100%; border-collapse: collapse; margin: 12px 0; }
td, th { padding: 6px 10px; border-bottom: 1px solid #313244; text-align: left; }
th { background: #313244; font-size: 12px; color: #a6adc8; }
.mockup {
	display: block;
	margin: 16px 0;
	max-width: 100%;
	border-radius: 6px;
	border: 1px solid #45475a;
	background: #181825;
}
.note {
	padding: 10px 14px;
	background: #313244;
	border-left: 3px solid #89b4fa;
	border-radius: 4px;
	margin: 12px 0;
	font-size: 13px;
}
.warn {
	padding: 10px 14px;
	background: #4a2c2c;
	border-left: 3px solid #f38ba8;
	border-radius: 4px;
	margin: 12px 0;
	font-size: 13px;
}
a { color: #89b4fa; }
`;
