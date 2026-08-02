import type { ManualSection } from '../helpContent';
import { mockupAppShell, mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Primeros pasos',
	body: `
<p>BridgeLab es un editor moderno de mensajes HL7 v2.x y FHIR, diseñado
para ingenieros de integración sanitaria. Está construido sobre un
backend en Rust para un análisis rápido (procesa mensajes de 10 MB en
menos de 2 segundos) y un frontend en Svelte 5 con el editor Monaco.</p>

<p>La ventana principal se divide en cuatro regiones:</p>
${mockupAppShell}

<ol>
	<li><strong>Barra de menús y banner de prueba</strong> en la parte
		superior - los menús Archivo, Editar, Ver, Herramientas y Ayuda,
		más un banner amarillo/rojo que recuerda el estado de la prueba
		Pro.</li>
	<li><strong>Panel de árbol</strong> a la izquierda - la estructura del
		mensaje analizado con flechas para expandir/contraer, y un
		Inspector de Campo en la parte inferior que muestra la información
		del esquema HL7 del nodo seleccionado.</li>
	<li><strong>Editor y pestañas</strong> en el centro - editor Monaco
		con resaltado de sintaxis HL7; barra de pestañas múltiples para
		mantener varios mensajes abiertos a la vez.</li>
	<li><strong>Barra de estado</strong> en la parte inferior - tipo de
		mensaje, versión, número de segmentos, posición del cursor.</li>
</ol>

<h3>Abrir un mensaje</h3>
<ul>
	<li><strong>Archivo → Abrir archivo</strong> (<kbd>Ctrl</kbd>+<kbd>O</kbd>) -
		selector de archivos nativo para <code>.hl7</code>, <code>.txt</code>,
		<code>.msg</code>, <code>.json</code>, <code>.xml</code>.</li>
	<li><strong>Arrastrar y soltar</strong> - suelta un archivo sobre el
		área del editor.</li>
	<li><strong>Pegar</strong> - haz clic en el editor y pega
		(<kbd>Ctrl</kbd>+<kbd>V</kbd>). El auto-análisis se ejecuta 500 ms
		después de la última pulsación de tecla.</li>
	<li><strong>Archivo → Nuevo desde plantilla</strong> (<kbd>Ctrl</kbd>+<kbd>N</kbd>) -
		plantillas ADT, ORM, ORU, SIU y más, ya rellenadas. Campos como
		MSH-7 y MSH-10 se completan con la fecha y hora actuales y un
		GUID nuevo.</li>
</ul>

<div class="note">En el primer arranque obtienes una <strong>prueba Pro
de 7 días</strong> con todas las funcionalidades habilitadas. Al
expirar, BridgeLab sigue funcionando con el conjunto de funciones
Community - nunca pierdes tus mensajes.</div>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Editor',
	body: `
<p>El área del editor es una instancia de <strong>Monaco</strong> con
una gramática específica para HL7. Los códigos de segmento se colorean
en morado, los separadores de campo en gris, y las cargas ED/base64 se
truncan automáticamente para mantener el editor rápido con mensajes
grandes.</p>

<h3>Autocompletado y hover</h3>
<p>Empieza a escribir <code>P</code> en una línea nueva - Monaco sugiere
<code>PID</code>, <code>PV1</code>, <code>PV2</code>, etc. Una vez
dentro de un segmento, el autocompletado tras cada pipe propone valores
de campo (códigos de sexo, códigos ACK, clase de paciente...). Al pasar
el cursor sobre cualquier campo se muestran su nombre, su tipo de dato y
su marca de obligatoriedad, extraídos del estándar HL7.</p>

<h3>Truncamiento de campos grandes</h3>
<p>Los campos que superan el umbral de truncamiento (100 bytes por
defecto, ajustable en <strong>Configuración → Analizador</strong>)
aparecen como <code>{...N bytes}</code>. El contenido completo nunca se
pierde - expándelo bajo demanda desde el menú contextual o el
<em>Inspector de Campo</em>.</p>

<h3>Menú contextual (clic derecho)</h3>
${mockupContextMenu}
<p>El menú agrupa las acciones en tres secciones:</p>
<ul>
	<li><strong>Navegación:</strong> Mostrar segmento en el árbol
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - abre el árbol y resalta el campo
		exacto bajo el cursor; Expandir / Contraer para valores
		truncados.</li>
	<li><strong>Portapapeles:</strong> Copiar segmento
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copiar mensaje completo (con los
		campos expandidos), Copiar mensaje truncado (seguro para
		email).</li>
</ul>

<div class="note">Los atajos nativos de Monaco (<kbd>Ctrl</kbd>+<kbd>F</kbd>
buscar, <kbd>Ctrl</kbd>+<kbd>H</kbd> reemplazar, <kbd>Ctrl</kbd>+<kbd>Z</kbd>
deshacer, <kbd>Ctrl</kbd>+<kbd>D</kbd> multicursor) funcionan todos como
es de esperar dentro del editor.</div>
`,
};

export const treeSection: ManualSection = {
	id: 'tree-view',
	heading: 'Vista de árbol e Inspector de Campo',
	body: `
<p>El árbol de la izquierda refleja la jerarquía del mensaje HL7:
<strong>segmentos</strong> → <strong>campos</strong> →
<strong>componentes</strong>. Muéstralo u ocúltalo con
<kbd>Ctrl</kbd>+<kbd>B</kbd> o <strong>Ver → Estructura del
mensaje</strong>.</p>

<h3>Navegar entre árbol y editor</h3>
<ul>
	<li><strong>Editor → árbol:</strong> haz clic derecho sobre un campo
		en Monaco y elige <em>Mostrar segmento en el árbol</em>. El árbol
		expande el segmento, selecciona el campo exacto (hasta el nivel
		de componente) y lo desplaza hasta hacerlo visible.</li>
	<li><strong>Árbol → editor:</strong> haz clic derecho sobre un nodo
		del árbol y elige <em>Mostrar en el editor</em>. Monaco salta a
		la línea, coloca el cursor en la columna correcta y selecciona el
		rango del campo.</li>
</ul>

<h3>Panel Inspector de Campo</h3>
<p>Haz clic en el icono <strong>ⓘ</strong> de la cabecera del panel de
árbol (o <strong>Ver → Inspector de Campo</strong>) para mostrar los
metadatos derivados del esquema para el nodo seleccionado:</p>
<ul>
	<li>Posición HL7 (p. ej. <code>PID-5</code>) y nombre canónico
		(Patient Name)</li>
	<li>Tipo de dato (XPN, CX, ST, ...), longitud máxima, marcas de
		requerido/repetible, descripción</li>
	<li>Valor actual y longitud; un botón <em>Ver valor completo</em>
		para los campos truncados</li>
</ul>
<p>Los segmentos desconocidos (Z-segments o códigos personalizados fuera
del estándar) muestran <em>Fuera del estándar HL7</em> pero siguen
siendo totalmente editables.</p>

<h3>Buscar en el árbol</h3>
<p>La casilla de búsqueda en la parte superior del árbol busca por
<strong>tipo de segmento</strong> (<code>PID</code>), <strong>nombre de
campo del esquema</strong> (<code>Patient Name</code>) y <strong>valor
del campo</strong> — incluidos los campos de segmentos que aún no has
expandido. Pulsa <kbd>Enter</kbd>/<kbd>Shift</kbd>+<kbd>Enter</kbd> para
recorrer las coincidencias, <kbd>Esc</kbd> para limpiar, y
<kbd>Ctrl</kbd>+<kbd>F</kbd> con el foco en el árbol para saltar a la
casilla. Al hacer clic en un resultado, el segmento se expande, el campo
se selecciona y se desplaza hasta quedar visible.</p>
<p class="note">La búsqueda del árbol funciona con mensajes HL7 v2. Para
recursos FHIR usa el filtro propio del visualizador de Bundle o el
<kbd>Ctrl</kbd>+<kbd>F</kbd> del editor.</p>

<h3>Comparar dos mensajes</h3>
<p><strong>Herramientas → Comparar mensajes…</strong> abre un diff lado
a lado de dos pestañas abiertas cualesquiera con resaltado de sintaxis
HL7. Elige izquierda/derecha en los desplegables, usa el botón ⇆ para
intercambiar los lados y pulsa <kbd>Esc</kbd> para cerrar. Debe haber al
menos dos pestañas abiertas.</p>

<h3>Valores permitidos en campos codificados</h3>
<p>Cuando el campo seleccionado está respaldado por una tabla de valores
HL7 (PID-8 Administrative Sex, PV1-2 Patient Class, MSA-1 Acknowledgment
Code, ORC-1 Order Control, OBX-11 Result Status, …), el inspector lista
los <strong>valores permitidos</strong> con su significado y resalta el
que está presente en el mensaje. Si el valor actual no figura en la
tabla, aparece una advertencia — una forma rápida de detectar códigos no
estándar antes de que el sistema receptor los rechace.</p>

<h3>Árbol consciente del esquema</h3>
<p><strong>Ver → Mostrar campos del estándar</strong> inserta filas
placeholder para cada campo definido por el estándar HL7 que está
<em>ausente</em> del mensaje. Los placeholders se muestran atenuados y
en cursiva - permiten ver fácilmente qué campos <em>podrías</em>
añadir, pero no se puede navegar hasta ellos en el editor (todavía no
tienen una posición física).</p>

<h3>Redimensionar los paneles</h3>
<p>Arrastra el separador vertical entre el árbol y el editor para
cambiar el ancho; arrastra el separador horizontal sobre el Inspector de
Campo para cambiar su altura. Ambas dimensiones se conservan entre
reinicios.</p>
`,
};
