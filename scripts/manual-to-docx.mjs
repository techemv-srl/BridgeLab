/**
 * Export the in-app English user manual to a Word document.
 *
 *   node scripts/manual-to-docx.mjs [output.docx]
 *
 * The manual lives in src/lib/components/layout/help/ as HTML strings, which
 * is also what the in-app Help window renders — this script is the single
 * source of truth for the Word edition, so regenerate it whenever those
 * files change. The UI mockups are inline SVG and are rasterized to PNG.
 *
 * The generated .docx is deliberately NOT committed (see .gitignore): the
 * public mirror only carries the sources, and the private copy is kept on
 * the internal notes branch.
 */
import { writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Resvg } from '@resvg/resvg-js';
import { parse as parseHtml } from 'node-html-parser';
import {
	AlignmentType, BorderStyle, Document, Footer, HeadingLevel, ImageRun, LevelFormat,
	Packer, PageBreak, PageNumber, Paragraph, ShadingType, Table, TableCell, TableRow,
	TextRun, WidthType,
} from 'docx';
import { createServer } from 'vite';

const ROOT = resolve(fileURLToPath(new URL('..', import.meta.url)));
const OUT = resolve(process.argv[2] ?? join(ROOT, 'docs', 'BridgeLab-User-Manual-EN.docx'));

// --- palette (mirrors the app's dark theme, on a white page) ----------------
const ACCENT = '1F5FA9';
const MUTED = '5A6472';
const CODE_BG = 'F2F3F5';
const NOTE_BG = 'EAF2FB';
const WARN_BG = 'FDF0E7';
const PAGE_WIDTH_DXA = 12240 - 2 * 1080; // Letter minus 0.75" margins

/** Load the manual sections through Vite so the TypeScript sources, their
 *  extensionless imports and the SVG mockups resolve exactly as in the app. */
async function loadSections() {
	const server = await createServer({
		root: ROOT,
		appType: 'custom',
		logLevel: 'silent',
		server: { middlewareMode: true, hmr: false, watch: null },
	});
	try {
		const mod = await server.ssrLoadModule('/src/lib/components/layout/help/en.ts');
		const meta = await server.ssrLoadModule('/src/lib/components/layout/helpContent.ts');
		return { sections: mod.enSections, title: meta.TITLES?.en ?? 'BridgeLab User Manual' };
	} finally {
		await server.close();
	}
}

const text = (node) => node.text.replace(/\s+/g, ' ').trim();

/** Inline formatting: <strong>, <em>, <code>, <kbd>, <a>. */
function runs(node, base = {}) {
	const out = [];
	for (const child of node.childNodes) {
		if (child.nodeType === 3) {
			const value = child.rawText.replace(/\s+/g, ' ');
			if (value) out.push(new TextRun({ text: decode(value), ...base }));
			continue;
		}
		const tag = child.rawTagName?.toLowerCase();
		if (tag === 'strong' || tag === 'b') out.push(...runs(child, { ...base, bold: true }));
		else if (tag === 'em' || tag === 'i') out.push(...runs(child, { ...base, italics: true }));
		else if (tag === 'code') out.push(...runs(child, { ...base, font: 'Consolas', shading: { type: ShadingType.CLEAR, fill: CODE_BG } }));
		else if (tag === 'kbd') out.push(...runs(child, { ...base, font: 'Consolas', bold: true, color: ACCENT }));
		else if (tag === 'a') out.push(...runs(child, { ...base, color: ACCENT, underline: {} }));
		else if (tag === 'br') out.push(new TextRun({ text: '', break: 1 }));
		else out.push(...runs(child, base));
	}
	return out;
}

function decode(s) {
	return s
		.replace(/&nbsp;/g, ' ').replace(/&amp;/g, '&').replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'")
		.replace(/&rarr;/g, '→').replace(/&hellip;/g, '…').replace(/&mdash;/g, '—');
}

function svgToImage(svgMarkup) {
	const svg = svgMarkup.includes('xmlns=')
		? svgMarkup
		: svgMarkup.replace('<svg', '<svg xmlns="http://www.w3.org/2000/svg"');
	const resvg = new Resvg(svg, { fitTo: { mode: 'width', value: 1400 }, background: 'white' });
	const rendered = resvg.render();
	const png = rendered.asPng();
	// keep the aspect ratio, fit the text column (6.5in = 624pt)
	const width = 468;
	const height = Math.round((rendered.height / rendered.width) * width);
	return new Paragraph({
		alignment: AlignmentType.CENTER,
		spacing: { before: 160, after: 200 },
		children: [new ImageRun({ data: png, type: 'png', transformation: { width, height } })],
	});
}

function blockToParagraphs(node, listCounters) {
	const tag = node.rawTagName?.toLowerCase();

	if (tag === 'svg') return [svgToImage(node.outerHTML)];

	if (tag === 'h3') {
		return [new Paragraph({ heading: HeadingLevel.HEADING_2, spacing: { before: 260, after: 120 }, children: runs(node) })];
	}
	if (tag === 'h4') {
		return [new Paragraph({ heading: HeadingLevel.HEADING_3, spacing: { before: 200, after: 100 }, children: runs(node) })];
	}
	if (tag === 'p') {
		return [new Paragraph({ spacing: { after: 140 }, children: runs(node) })];
	}
	if (tag === 'pre') {
		// <pre> is a raw-text element for the parser, so its inner <code>
		// markup survives in .text — strip tags first, decode entities after
		// (the manual writes literal placeholders as &lt;config&gt;).
		const raw = node.text.replace(/<\/?[a-zA-Z][^>]*>/g, '');
		// each source line becomes its own paragraph: docx has no \n
		const lines = decode(raw).replace(/\t/g, '    ').split('\n');
		while (lines.length && !lines[0].trim()) lines.shift();
		while (lines.length && !lines[lines.length - 1].trim()) lines.pop();
		return lines.map((line, i) => new Paragraph({
			spacing: { before: i === 0 ? 100 : 0, after: i === lines.length - 1 ? 160 : 0 },
			shading: { type: ShadingType.CLEAR, fill: CODE_BG },
			children: [new TextRun({ text: line || ' ', font: 'Consolas', size: 18 })],
		}));
	}
	if (tag === 'ul' || tag === 'ol') {
		const ordered = tag === 'ol';
		return node.querySelectorAll(':scope > li').map((li) => new Paragraph({
			numbering: { reference: ordered ? 'manual-ordered' : 'manual-bullets', level: 0 },
			spacing: { after: 80 },
			children: runs(li),
		}));
	}
	if (tag === 'table') {
		return [tableToDocx(node), new Paragraph({ spacing: { after: 160 }, children: [] })];
	}
	if (tag === 'div') {
		const cls = node.getAttribute('class') ?? '';
		const fill = cls.includes('warn') ? WARN_BG : NOTE_BG;
		const label = cls.includes('warn') ? 'Warning' : cls.includes('info') ? 'Info' : 'Note';
		return [new Paragraph({
			spacing: { before: 160, after: 180 },
			shading: { type: ShadingType.CLEAR, fill },
			border: { left: { style: BorderStyle.SINGLE, size: 18, color: ACCENT, space: 8 } },
			children: [new TextRun({ text: `${label}: `, bold: true, color: ACCENT }), ...runs(node)],
		})];
	}
	// unknown wrapper: keep walking
	return node.childNodes.flatMap((c) => (c.nodeType === 1 ? blockToParagraphs(c, listCounters) : []));
}

function tableToDocx(table) {
	const rows = table.querySelectorAll('tr');
	const colCount = Math.max(...rows.map((r) => r.querySelectorAll('th,td').length));
	const colWidth = Math.floor(PAGE_WIDTH_DXA / colCount);
	const columnWidths = Array.from({ length: colCount }, (_, i) =>
		i === colCount - 1 ? PAGE_WIDTH_DXA - colWidth * (colCount - 1) : colWidth);

	return new Table({
		columnWidths,
		width: { size: PAGE_WIDTH_DXA, type: WidthType.DXA },
		rows: rows.map((tr) => {
			const cells = tr.querySelectorAll('th,td');
			const isHeader = cells.some((c) => c.rawTagName.toLowerCase() === 'th');
			return new TableRow({
				tableHeader: isHeader,
				children: cells.map((cell, i) => new TableCell({
					width: { size: columnWidths[i] ?? colWidth, type: WidthType.DXA },
					shading: isHeader ? { type: ShadingType.CLEAR, fill: CODE_BG } : undefined,
					margins: { top: 60, bottom: 60, left: 100, right: 100 },
					children: [new Paragraph({ children: runs(cell, isHeader ? { bold: true } : {}) })],
				})),
			});
		}),
	});
}

function sectionToParagraphs(section, index) {
	const body = parseHtml(section.body);
	const blocks = body.childNodes
		.filter((n) => n.nodeType === 1)
		.flatMap((n) => blockToParagraphs(n));
	return [
		new Paragraph({
			heading: HeadingLevel.HEADING_1,
			pageBreakBefore: index > 0,
			spacing: { after: 200 },
			children: [new TextRun({ text: decode(section.heading), color: ACCENT })],
		}),
		...blocks,
	];
}

const { sections, title } = await loadSections();
const version = JSON.parse(await (await import('node:fs/promises')).readFile(join(ROOT, 'package.json'), 'utf8')).version;
const today = new Date().toISOString().slice(0, 10);

const cover = [
	new Paragraph({ spacing: { before: 2600, after: 0 }, alignment: AlignmentType.CENTER,
		children: [new TextRun({ text: 'BridgeLab', bold: true, size: 72, color: ACCENT })] }),
	new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 320 },
		children: [new TextRun({ text: 'HL7 made simple', size: 28, color: MUTED })] }),
	new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 120 },
		children: [new TextRun({ text: title, bold: true, size: 36 })] }),
	new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 60 },
		children: [new TextRun({ text: `Version ${version}`, size: 24, color: MUTED })] }),
	new Paragraph({ alignment: AlignmentType.CENTER,
		children: [new TextRun({ text: `Generated ${today} · TECHEMV SRL`, size: 20, color: MUTED })] }),
	new Paragraph({ children: [new PageBreak()] }),
	new Paragraph({ heading: HeadingLevel.HEADING_1, spacing: { after: 200 },
		children: [new TextRun({ text: 'Contents', color: ACCENT })] }),
	...sections.map((s, i) => new Paragraph({
		spacing: { after: 60 },
		children: [new TextRun({ text: `${i + 1}.  ${decode(s.heading)}` })],
	})),
];

const doc = new Document({
	creator: 'TECHEMV SRL',
	title,
	description: `BridgeLab ${version} — English user manual`,
	numbering: {
		config: [
			{ reference: 'manual-bullets', levels: [{ level: 0, format: LevelFormat.BULLET, text: '•', alignment: AlignmentType.LEFT,
				style: { paragraph: { indent: { left: 420, hanging: 220 } } } }] },
			{ reference: 'manual-ordered', levels: [{ level: 0, format: LevelFormat.DECIMAL, text: '%1.', alignment: AlignmentType.LEFT,
				style: { paragraph: { indent: { left: 420, hanging: 220 } } } }] },
		],
	},
	styles: {
		default: {
			document: { run: { font: 'Calibri', size: 21 }, paragraph: { spacing: { line: 276 } } },
			heading1: { run: { font: 'Calibri', size: 34, bold: true, color: ACCENT } },
			heading2: { run: { font: 'Calibri', size: 26, bold: true, color: '1A1A1A' } },
			heading3: { run: { font: 'Calibri', size: 23, bold: true, color: '1A1A1A' } },
		},
	},
	sections: [{
		properties: { page: { size: { width: 12240, height: 15840 }, margin: { top: 1080, bottom: 1080, left: 1080, right: 1080 } } },
		footers: {
			default: new Footer({
				children: [new Paragraph({
					alignment: AlignmentType.CENTER,
					children: [new TextRun({ text: `BridgeLab ${version} — User Manual   ·   `, size: 16, color: MUTED }),
						new TextRun({ children: [PageNumber.CURRENT], size: 16, color: MUTED })],
				})],
			}),
		},
		children: [...cover, ...sections.flatMap(sectionToParagraphs)],
	}],
});

const buffer = await Packer.toBuffer(doc);
writeFileSync(OUT, buffer);
console.log(`${OUT}  (${sections.length} sections, ${(buffer.length / 1024).toFixed(0)} KB)`);
