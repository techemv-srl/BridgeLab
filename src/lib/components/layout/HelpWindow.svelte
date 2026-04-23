<script lang="ts">
	import { t, subscribeLocale, getLocale } from '$lib/i18n';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') {
		subscribeLocale(() => { localeVersion++; });
	}

	let windowEl = $state<HTMLDivElement | undefined>(undefined);
	let x = $state(120);
	let y = $state(80);
	let w = $state(640);
	let h = $state(520);
	let dragging = $state(false);
	let resizing = $state(false);
	let dragOffset = { x: 0, y: 0 };

	function startDrag(e: MouseEvent) {
		if ((e.target as HTMLElement).closest('.help-close')) return;
		dragging = true;
		dragOffset = { x: e.clientX - x, y: e.clientY - y };
		e.preventDefault();
	}

	function startResize(e: MouseEvent) {
		resizing = true;
		e.preventDefault();
		e.stopPropagation();
	}

	function onMouseMove(e: MouseEvent) {
		if (dragging) {
			x = Math.max(0, e.clientX - dragOffset.x);
			y = Math.max(0, e.clientY - dragOffset.y);
		} else if (resizing) {
			w = Math.max(400, e.clientX - x);
			h = Math.max(300, e.clientY - y);
		}
	}

	function onMouseUp() {
		dragging = false;
		resizing = false;
	}

	// Manual content by locale
	const manuals: Record<string, { title: string; sections: { heading: string; body: string }[] }> = {
		en: {
			title: 'BridgeLab User Manual',
			sections: [
				{ heading: 'Getting Started',
				  body: `<p>BridgeLab is a modern HL7 v2.x and FHIR message editor for healthcare integration professionals.</p>
<p><strong>Open a file:</strong> File → Open (Ctrl+O), or drag & drop an .hl7 / .json file into the editor.</p>
<p><strong>Paste a message:</strong> Click in the editor area and paste (Ctrl+V). Auto-parse runs after 500ms.</p>
<p><strong>New from template:</strong> File → New from Template (Ctrl+N) to create a pre-filled ADT, ORM, ORU, etc.</p>` },
				{ heading: 'Editor',
				  body: `<p>The Monaco-based editor provides syntax highlighting, auto-complete for HL7 segments and fields, word wrap, minimap, and find/replace (Ctrl+F).</p>
<p><strong>Right-click context menu:</strong> Show in Tree, Expand/Collapse truncated fields, Copy Segment, Copy Full/Truncated Message.</p>
<p><strong>Large messages:</strong> Fields over the truncation threshold appear as <code>{...N bytes}</code>. Right-click → Expand to see the full content.</p>` },
				{ heading: 'Tree View',
				  body: `<p>The tree panel (Ctrl+B to toggle) shows the message structure: segments → fields → components.</p>
<p><strong>Field Inspector:</strong> Click the ⓘ button or View → Field Inspector to see HL7 schema info (name, type, required, max length) for the selected node.</p>
<p><strong>Schema-aware mode:</strong> View → Show Schema Fields injects placeholder rows for standard fields not present in the message.</p>
<p><strong>Navigate to Editor:</strong> Right-click a tree node → Show in Editor to jump to that field with selection.</p>` },
				{ heading: 'Validation',
				  body: `<p>Press <strong>F6</strong> or Tools → Validate to run structural, field-level, and data-type validation.</p>
<p>Issues appear in the bottom Validation panel, filterable by severity (Error / Warning / Info) and segment.</p>
<p><strong>Plugin rules:</strong> Drop JSON rule packs in the plugins folder to add custom checks (see Settings → Plugins).</p>` },
				{ heading: 'Communication (MLLP / HTTP)',
				  body: `<p>Open the Communication panel with <strong>Ctrl+K</strong> or Tools → Communication Panel.</p>
<p><strong>MLLP Send:</strong> Enter host + port, click Send. ACK/NACK is displayed with response time.</p>
<p><strong>MLLP Listen:</strong> Click Listen to start a server on a port and receive incoming messages (Pro feature).</p>
<p><strong>HTTP:</strong> GET requests are available in Community. POST/PUT/DELETE and auth headers require Pro.</p>
<p>Connection profiles and request history are saved automatically.</p>` },
				{ heading: 'Anonymization',
				  body: `<p>Tools → Anonymize detects PHI fields (PID, NK1, IN1, GT1) and masks them by sensitivity level.</p>
<p><strong>High:</strong> REDACTED or 000... &nbsp; <strong>Medium:</strong> J*** &nbsp; <strong>Low:</strong> Joh...</p>
<p>Anonymized output opens in a new tab or can be copied to clipboard. Requires Pro license.</p>` },
				{ heading: 'FHIR Support',
				  body: `<p>Paste or open a FHIR JSON resource (Patient, Observation, Bundle). BridgeLab auto-detects the format.</p>
<p><strong>Bundle Visualizer:</strong> Tools → FHIR Bundle Visualizer shows entries, cross-references, and dangling refs (Pro).</p>
<p><strong>FHIRPath:</strong> Ctrl+P opens the FHIRPath evaluator panel. Supports count(), first(), last(), where(), select(), distinct() (Pro).</p>` },
				{ heading: 'Plugins',
				  body: `<p>Drop JSON files in <code>&lt;config&gt;/BridgeLab/plugins/validation/</code> or <code>anonymization/</code>.</p>
<p>Supported checks: not_empty, regex, one_of, max_length, min_length, contains. Component-level targeting via the "component" field.</p>
<p>Manage in Settings → Plugins: enable/disable per pack, Reload, Open folder.</p>` },
				{ heading: 'Licensing',
				  body: `<p><strong>Community (Free):</strong> Core HL7 editor, parser, validation, basic MLLP send + HTTP GET, PHI detection.</p>
<p><strong>Trial (30 days):</strong> Full Pro access. After expiry, falls back to Community.</p>
<p><strong>Professional:</strong> MLLP listener, full HTTP, anonymization, export, FHIRPath, Bundle Visualizer, unlimited plugins.</p>
<p><strong>Enterprise:</strong> All Pro + SOAP, priority support.</p>
<p>Activate: Settings → License → paste your key. Contact info@techemv.it to purchase.</p>` },
				{ heading: 'Keyboard Shortcuts',
				  body: `<table>
<tr><td><strong>Ctrl+O</strong></td><td>Open file</td></tr>
<tr><td><strong>Ctrl+N</strong></td><td>New from template</td></tr>
<tr><td><strong>Ctrl+S</strong></td><td>Save</td></tr>
<tr><td><strong>Ctrl+W</strong></td><td>Close tab</td></tr>
<tr><td><strong>Ctrl+B</strong></td><td>Toggle tree</td></tr>
<tr><td><strong>F5</strong></td><td>Re-parse</td></tr>
<tr><td><strong>F6</strong></td><td>Validate</td></tr>
<tr><td><strong>Ctrl+K</strong></td><td>Communication panel</td></tr>
<tr><td><strong>Ctrl+P</strong></td><td>FHIRPath panel</td></tr>
<tr><td><strong>Ctrl+,</strong></td><td>Settings</td></tr>
<tr><td><strong>F1</strong></td><td>This help</td></tr>
</table>
<p>Customize in Settings → Shortcuts.</p>` },
			]
		},
		it: {
			title: 'Manuale Utente BridgeLab',
			sections: [
				{ heading: 'Per Iniziare',
				  body: `<p>BridgeLab è un editor moderno per messaggi HL7 v2.x e FHIR per professionisti dell'integrazione sanitaria.</p>
<p><strong>Aprire un file:</strong> File → Apri (Ctrl+O), oppure trascina un file .hl7 / .json nell'editor.</p>
<p><strong>Incollare un messaggio:</strong> Clicca nell'editor e incolla (Ctrl+V). L'analisi automatica parte dopo 500ms.</p>
<p><strong>Nuovo da modello:</strong> File → Nuovo da Modello (Ctrl+N) per creare un messaggio ADT, ORM, ORU, ecc.</p>` },
				{ heading: 'Editor',
				  body: `<p>L'editor basato su Monaco offre evidenziazione della sintassi, auto-completamento per segmenti e campi HL7, word wrap, minimap e trova/sostituisci (Ctrl+F).</p>
<p><strong>Menu contestuale (tasto destro):</strong> Mostra nel Tree, Espandi/Comprimi campi troncati, Copia Segmento, Copia Messaggio Completo/Troncato.</p>
<p><strong>Messaggi grandi:</strong> I campi oltre la soglia di troncamento appaiono come <code>{...N bytes}</code>. Tasto destro → Espandi per vedere il contenuto completo.</p>` },
				{ heading: 'Vista ad Albero',
				  body: `<p>Il pannello tree (Ctrl+B per mostrare/nascondere) mostra la struttura del messaggio: segmenti → campi → componenti.</p>
<p><strong>Ispettore Campo:</strong> Clicca il bottone ⓘ o Visualizza → Ispettore Campo per vedere le info dello schema HL7 (nome, tipo, obbligatorio, lunghezza max) per il nodo selezionato.</p>
<p><strong>Modalità schema:</strong> Visualizza → Mostra campi dello standard inserisce righe placeholder per i campi standard non presenti nel messaggio.</p>
<p><strong>Naviga all'Editor:</strong> Tasto destro su un nodo del tree → Mostra nell'Editor per saltare a quel campo con selezione.</p>` },
				{ heading: 'Validazione',
				  body: `<p>Premi <strong>F6</strong> o Strumenti → Valida per eseguire la validazione strutturale, a livello di campo e tipo dati.</p>
<p>I problemi appaiono nel pannello Validazione in basso, filtrabile per gravità (Errore / Avviso / Info) e segmento.</p>
<p><strong>Regole plugin:</strong> Inserisci file JSON nella cartella plugin per aggiungere controlli personalizzati (vedi Impostazioni → Plugin).</p>` },
				{ heading: 'Comunicazione (MLLP / HTTP)',
				  body: `<p>Apri il pannello Comunicazione con <strong>Ctrl+K</strong> o Strumenti → Pannello Comunicazione.</p>
<p><strong>Invio MLLP:</strong> Inserisci host + porta, clicca Invia. ACK/NACK mostrato con tempo di risposta.</p>
<p><strong>Ascolto MLLP:</strong> Clicca Ascolta per avviare un server su una porta e ricevere messaggi in arrivo (funzione Pro).</p>
<p><strong>HTTP:</strong> Le richieste GET sono disponibili nella versione Community. POST/PUT/DELETE e header di autenticazione richiedono Pro.</p>` },
				{ heading: 'Anonimizzazione',
				  body: `<p>Strumenti → Anonimizza rileva i campi PHI (PID, NK1, IN1, GT1) e li maschera per livello di sensibilità.</p>
<p><strong>Alta:</strong> REDACTED o 000... &nbsp; <strong>Media:</strong> J*** &nbsp; <strong>Bassa:</strong> Joh...</p>
<p>L'output anonimizzato si apre in un nuovo tab o può essere copiato negli appunti. Richiede licenza Pro.</p>` },
				{ heading: 'Supporto FHIR',
				  body: `<p>Incolla o apri una risorsa FHIR JSON (Patient, Observation, Bundle). BridgeLab rileva automaticamente il formato.</p>
<p><strong>Visualizzatore Bundle:</strong> Strumenti → Visualizzatore Bundle FHIR mostra le entry, i riferimenti incrociati e i riferimenti pendenti (Pro).</p>
<p><strong>FHIRPath:</strong> Ctrl+P apre il pannello FHIRPath. Supporta count(), first(), last(), where(), select(), distinct() (Pro).</p>` },
				{ heading: 'Plugin',
				  body: `<p>Inserisci file JSON in <code>&lt;config&gt;/BridgeLab/plugins/validation/</code> o <code>anonymization/</code>.</p>
<p>Controlli supportati: not_empty, regex, one_of, max_length, min_length, contains. Targeting a livello di componente tramite il campo "component".</p>
<p>Gestisci in Impostazioni → Plugin: attiva/disattiva per pack, Ricarica, Apri cartella.</p>` },
				{ heading: 'Licenza',
				  body: `<p><strong>Community (Gratuita):</strong> Editor HL7, parser, validazione, invio MLLP + HTTP GET, rilevamento PHI.</p>
<p><strong>Trial (30 giorni):</strong> Accesso Pro completo. Dopo la scadenza, ritorna a Community.</p>
<p><strong>Professional:</strong> Listener MLLP, HTTP completo, anonimizzazione, export, FHIRPath, Visualizzatore Bundle, plugin illimitati.</p>
<p><strong>Enterprise:</strong> Tutto Pro + SOAP, supporto prioritario.</p>
<p>Attiva: Impostazioni → Licenza → incolla la tua chiave. Contatta info@techemv.it per l'acquisto.</p>` },
				{ heading: 'Scorciatoie da Tastiera',
				  body: `<table>
<tr><td><strong>Ctrl+O</strong></td><td>Apri file</td></tr>
<tr><td><strong>Ctrl+N</strong></td><td>Nuovo da modello</td></tr>
<tr><td><strong>Ctrl+S</strong></td><td>Salva</td></tr>
<tr><td><strong>Ctrl+W</strong></td><td>Chiudi tab</td></tr>
<tr><td><strong>Ctrl+B</strong></td><td>Mostra/nascondi tree</td></tr>
<tr><td><strong>F5</strong></td><td>Ri-analizza</td></tr>
<tr><td><strong>F6</strong></td><td>Valida</td></tr>
<tr><td><strong>Ctrl+K</strong></td><td>Pannello comunicazione</td></tr>
<tr><td><strong>Ctrl+P</strong></td><td>Pannello FHIRPath</td></tr>
<tr><td><strong>Ctrl+,</strong></td><td>Impostazioni</td></tr>
<tr><td><strong>F1</strong></td><td>Questo aiuto</td></tr>
</table>
<p>Personalizza in Impostazioni → Scorciatoie.</p>` },
			]
		},
		fr: { title: 'Manuel Utilisateur BridgeLab', sections: [] },
		es: { title: 'Manual de Usuario BridgeLab', sections: [] },
		de: { title: 'BridgeLab Benutzerhandbuch', sections: [] },
	};

	// Fallback: if no sections for current locale, use English
	let currentManual = $derived.by(() => {
		void localeVersion;
		const locale = getLocale();
		const m = manuals[locale];
		if (m && m.sections.length > 0) return m;
		return manuals['en'];
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="help-window"
	bind:this={windowEl}
	style="left:{x}px;top:{y}px;width:{w}px;height:{h}px"
>
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="help-titlebar" onmousedown={startDrag}>
		<span class="help-title">{currentManual.title}</span>
		<button class="help-close" onclick={onClose}>&times;</button>
	</div>

	<div class="help-body">
		<nav class="help-toc">
			{#each currentManual.sections as sec, i}
				<button class="toc-item" onclick={() => {
					document.getElementById(`help-sec-${i}`)?.scrollIntoView({ behavior: 'smooth' });
				}}>{sec.heading}</button>
			{/each}
		</nav>

		<div class="help-content">
			{#each currentManual.sections as sec, i}
				<section id="help-sec-{i}">
					<h2>{sec.heading}</h2>
					{@html sec.body}
				</section>
			{/each}
		</div>
	</div>

	<!-- Resize handle -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="help-resize" onmousedown={startResize}></div>
</div>

<style>
	.help-window {
		position: fixed;
		z-index: 2000;
		display: flex;
		flex-direction: column;
		background: var(--color-bg-primary);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		box-shadow: 0 12px 40px rgba(0,0,0,0.5);
		overflow: hidden;
	}

	.help-titlebar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 14px;
		background: var(--color-bg-tertiary);
		cursor: grab;
		user-select: none;
		flex-shrink: 0;
	}
	.help-titlebar:active { cursor: grabbing; }
	.help-title { font-weight: 700; font-size: 13px; }
	.help-close {
		background: none; border: none; color: var(--color-text-secondary);
		font-size: 20px; cursor: pointer; line-height: 1;
	}
	.help-close:hover { color: var(--color-error); }

	.help-body {
		flex: 1;
		display: flex;
		overflow: hidden;
		min-height: 0;
	}

	.help-toc {
		width: 180px;
		flex-shrink: 0;
		overflow-y: auto;
		padding: 10px 0;
		border-right: 1px solid var(--color-border);
		background: var(--color-bg-secondary);
	}
	.toc-item {
		display: block;
		padding: 6px 14px;
		font-size: 12px;
		color: var(--color-text-secondary);
		text-decoration: none;
		border-left: 2px solid transparent;
	}
	.toc-item:hover {
		color: var(--color-text-primary);
		background: var(--color-bg-tertiary);
		border-left-color: var(--color-accent);
	}

	.help-content {
		flex: 1;
		overflow-y: auto;
		padding: 16px 24px;
		font-size: 13px;
		line-height: 1.6;
	}
	.help-content h2 {
		font-size: 16px;
		font-weight: 700;
		color: var(--color-accent);
		margin: 24px 0 8px;
		padding-bottom: 4px;
		border-bottom: 1px solid var(--color-border);
	}
	.help-content h2:first-child { margin-top: 0; }
	.help-content p { margin: 6px 0; color: var(--color-text-primary); }
	.help-content code {
		font-family: 'JetBrains Mono', monospace;
		font-size: 12px;
		background: var(--color-bg-tertiary);
		padding: 1px 5px;
		border-radius: 3px;
	}
	.help-content table { width: 100%; border-collapse: collapse; margin: 8px 0; }
	.help-content td {
		padding: 4px 10px;
		border-bottom: 1px solid var(--color-border);
		font-size: 12px;
	}
	.help-content td:first-child { width: 120px; white-space: nowrap; }

	.help-resize {
		position: absolute;
		bottom: 0;
		right: 0;
		width: 16px;
		height: 16px;
		cursor: nwse-resize;
	}
	.help-resize::after {
		content: '';
		position: absolute;
		bottom: 4px;
		right: 4px;
		width: 8px;
		height: 8px;
		border-right: 2px solid var(--color-text-secondary);
		border-bottom: 2px solid var(--color-text-secondary);
		opacity: 0.5;
	}
</style>
