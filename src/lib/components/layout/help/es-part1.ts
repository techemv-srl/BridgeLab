import type { ManualSection } from '../helpContent';
import { mockupAppShellEs as mockupAppShell, mockupContextMenuEs as mockupContextMenu } from './mockups';

export const getStarted: ManualSection = {
	id: 'getting-started',
	heading: 'Primeros pasos',
	body: `
<p>BridgeLab es un editor moderno de mensajes HL7 v2.x y FHIR, diseñado
para ingenieros de integración sanitaria. Está construido sobre un
backend en Rust para un análisis rápido (un mensaje de 10 MB con un adjunto base64
se abre en unos 2 segundos) y un frontend en Svelte 5 con el editor Monaco.</p>

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
		después de la última pulsación de tecla (retraso, o desactivación,
		en <strong>Configuración → Analizador</strong>).</li>
	<li><strong>Qué se abre:</strong> líneas vacías, espacios, un BOM o el
		marco MLLP antes de <code>MSH</code>, los archivos UTF-16 (el
		«Unicode» del Bloc de notas) y los archivos por lotes FHS/BHS se
		leen tal cual. Un archivo que BridgeLab no puede analizar se abre
		igualmente, como texto para corregir, con una nota que explica por
		qué.</li>
	<li><strong>Archivo → Nuevo Mensaje desde Plantilla...</strong> (<kbd>Ctrl</kbd>+<kbd>N</kbd>) -
		plantillas ADT, ORM, ORU, SIU y más, ya rellenadas. Campos como
		MSH-7 y MSH-10 se completan con la fecha y hora actuales y un
		identificador de mensaje único.</li>
	<li><strong>Archivo → Mensajes de ejemplo</strong> (también desde la
		pantalla de inicio) - mensajes completos y realistas en lugar de
		esqueletos: ADT de ingreso, registro, actualización, alta y fusión,
		ORU con resultados (un hemograma de doce valores, un panel
		metabólico), ORM, SIU, MDM, DFT, VXU y un ACK, en las versiones 2.3,
		2.5 y 2.5.1. Filtra por versión, mira la vista previa y abre uno en
		una pestaña nueva. Cada ejemplo supera la validación; pacientes y
		datos son ficticios.</li>
</ul>

<div class="note">En el primer arranque obtienes una <strong>prueba Pro
de 14 días</strong> con todas las funcionalidades Pro habilitadas (SOAP y
el soporte prioritario son de Enterprise). Al
expirar, BridgeLab sigue funcionando con el conjunto de funciones
Community - nunca pierdes tus mensajes.</div>

<p>Las tarjetas <strong>Descubre BridgeLab</strong> de la pantalla de
bienvenida abren directamente las funciones que distinguen a BridgeLab —
generador de mensajes de prueba, anonimización PHI, listener MLLP y
exportación XSD — con insignias PRO para las funciones con licencia.</p>
`,
};

export const editorSection: ManualSection = {
	id: 'editor',
	heading: 'Editor',
	body: `
<p>El área del editor es una instancia de <strong>Monaco</strong> con
una gramática específica para HL7. Los códigos de segmento se colorean
en morado, los separadores de campo en gris, y las cargas ED/base64 y otros valores
largos se muestran plegados, para mantener el editor rápido con mensajes
grandes (ver más abajo).</p>

<h3>Autocompletado y hover</h3>
<p>Empieza a escribir <code>P</code> en una línea nueva - Monaco sugiere
<code>PID</code>, <code>PV1</code>, <code>PV2</code>, etc. Una vez
dentro de un segmento, el autocompletado tras cada pipe propone valores
de campo (códigos de sexo, códigos ACK, clase de paciente...). Al pasar
el cursor sobre cualquier campo se muestran su nombre, su tipo de dato y
su marca de obligatoriedad, extraídos del estándar HL7.</p>

<h3>Campos largos plegados</h3>
<p>Los campos más largos que el umbral de plegado (100 caracteres por
defecto, en <strong>Configuración → Analizador</strong>) se muestran
<em>plegados</em>: un adjunto base64 en OBX-5, una nota larga o una cadena
JSON <code>data</code> aparece como una etiqueta compacta, por ejemplo
<code>⟨Base64 · 5.1 KB⟩</code>, mientras los separadores y los demás
componentes siguen visibles. El plegado es solo una vista: el mensaje
siempre está completo, y guardar, la sesión, la validación, el árbol y
copiar usan el texto entero.</p>
<ul>
	<li><strong>Expandir:</strong> clic en la etiqueta, o cursor al lado y
		<kbd>Alt</kbd>+<kbd>Intro</kbd>. Al pasar el ratón se ven los
		primeros caracteres.</li>
	<li><strong>Volver a plegar:</strong> clic derecho → <em>Plegar este
		campo</em> sobre un valor largo, o <em>Plegar todos los campos
		largos</em>.</li>
	<li><strong>Todo a la vez:</strong> la insignia <em>N plegados</em> de
		la barra de estado lo expande todo; el menú contextual tiene ambos
		comandos.</li>
	<li>Una etiqueta se mueve como una unidad: las flechas la saltan y
		Retroceso/Supr la seleccionan primero, así nunca se borra a medias.
		Copiar una selección copia el contenido completo.</li>
	<li><strong>Buscar y Reemplazar</strong> (<kbd>Ctrl</kbd>+<kbd>F</kbd>,
		<kbd>Ctrl</kbd>+<kbd>H</kbd>) trabajan sobre el texto completo: un
		campo plegado que contiene una coincidencia se despliega, y esta se
		cuenta, se muestra y se reemplaza como las demás.</li>
</ul>

<h3>Configuración del editor</h3>
<p><strong>Editar → Configuración → Editor</strong> (Ctrl+,) cambia el aspecto y el
comportamiento del editor: fuente y tamaño, ancho de tabulación, ajuste de
línea, espacios visibles, minimapa, números de línea, desplazamiento suave,
colores de paréntesis, resaltado de las demás apariciones de la palabra bajo
el cursor, enlaces clicables, cabecera fija al desplazarse (sticky scroll) y
origen de las sugerencias de palabras (este mensaje, todos los mensajes
abiertos o ninguno; las sugerencias de campos y valores HL7 funcionan en
todos los modos). Los cambios se aplican al instante. Cada pestaña conserva
su propio historial de deshacer y rehacer (Ctrl+Z, Ctrl+Y) al pasar de una
pestaña a otra.</p>

<h3>Menú contextual (clic derecho)</h3>
${mockupContextMenu}
<p>El menú agrupa las acciones en tres secciones:</p>
<ul>
	<li><strong>Navegación:</strong> Mostrar segmento en el árbol
		(<kbd>Alt</kbd>+<kbd>T</kbd>) - abre el árbol y resalta el campo
		exacto bajo el cursor; Expandir / Plegar para valores
		largos.</li>
	<li><strong>Portapapeles:</strong> Copiar segmento
		(<kbd>Alt</kbd>+<kbd>C</kbd>), Copiar mensaje completo (con los
		campos expandidos), Copiar mensaje truncado (campos largos
		acortados, para caber en un email; los datos del paciente se copian
		tal cual: anonimiza primero).</li>
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
<strong>componentes</strong>; un campo repetido lista cada repetición
(<code>PID-3(1)</code>, <code>PID-3(2)</code>) con sus componentes. Muéstralo u ocúltalo con
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

<p>Las líneas vacías entre segmentos no desvían los saltos. En el
árbol, <kbd>↑</kbd>/<kbd>↓</kbd> mueven la selección, <kbd>→</kbd>
expande un nodo y <kbd>←</kbd> lo contrae o sube al padre;
<kbd>Inicio</kbd>/<kbd>Fin</kbd> y <kbd>RePág</kbd>/<kbd>AvPág</kbd>
saltan. Mientras editas, el árbol conserva lo que has expandido y
seleccionado, y el Inspector de Campo muestra el valor actual.</p>

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
		para los campos largos que el editor muestra plegados</li>
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

<h3>Cuadrícula de segmentos</h3>
<p>Un mensaje de resultados puede traer decenas de OBX, y el árbol los
muestra de uno en uno. <strong>Ver → Cuadrícula de segmentos</strong>
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>), o <em>Mostrar todos los
OBX en una tabla</em> desde el menú contextual de un segmento, abre un
panel inferior con todas las apariciones de un tipo de segmento en forma
de tabla: una fila por aparición y una columna por cada campo que tenga
valor en al menos una de ellas, con la posición y el nombre del campo en
la versión HL7 del mensaje. Los valores codificados muestran su
significado bajo el código, como en el árbol; los valores largos se
recortan con puntos suspensivos. Elige otro segmento de la lista (cada
uno indica cuántas veces aparece), escribe en <em>Filtrar filas</em> para
quedarte con las filas que contienen un texto y haz clic en una celda
para seleccionar ese campo en el editor. La cuadrícula es de solo lectura
y sigue al mensaje mientras lo editas.</p>

<h3>Comparar dos mensajes</h3>
<p><strong>Herramientas → Comparar mensajes…</strong> abre un diff lado
a lado de dos pestañas abiertas cualesquiera con resaltado de sintaxis
HL7. Elige izquierda/derecha en los desplegables, usa el botón ⇆ para
intercambiar los lados y pulsa <kbd>Esc</kbd> para cerrar. Debe haber al
menos dos pestañas abiertas.</p>

<h3>Campos codificados: qué significa un código</h3>
<p>BridgeLab incluye las tablas de valores HL7 — 394 tablas, unos 5.000
códigos — y sabe de qué tabla toma sus valores cada campo y componente
codificado, por versión. Un valor codificado se explica allí donde lo
encuentres: el árbol muestra el significado junto al valor
(<code>M — Male</code>, <code>ADT — ADT message</code>,
<code>F — Final results</code>), el hover sobre el campo en el editor
lo repite, y el autocompletado en un campo codificado ofrece todos los
valores de su tabla. Los componentes también están cubiertos: MSH-9.2 se
explica con la tabla de eventos, PID-3.5 con la de tipos de
identificador.</p>
<p>El Inspector de Campo lista la tabla completa del campo o componente
seleccionado y resalta el valor actual. Que un valor fuera de la tabla
sea un problema depende del tipo de dato, y el inspector indica en qué
caso estás: un campo <code>ID</code> toma sus valores de una tabla
definida por HL7 (<em>Valores permitidos</em>) y un valor no listado es
no estándar — aparece una advertencia; un campo <code>IS</code> los toma
de una tabla definida por el usuario (<em>Valores sugeridos</em>), donde
cada centro añade sus propios códigos y la ausencia no significa nada.
Algunas tablas de usuario no tienen ningún valor estándar (IN1-2
Insurance Plan ID): esos campos no muestran lista.</p>
<p class="note">Qué tabla usa un campo sigue la versión HL7 declarada; el
contenido de las tablas es un único conjunto para todas las versiones,
tal como lo distribuye la fuente original. Un código añadido en una
versión posterior se acepta, por tanto, también en una anterior.</p>

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

<h3>Estructura estándar completa</h3>
<p>Con <strong>Ver → Mostrar campos del estándar</strong> activado, el árbol
muestra también los segmentos que el estándar define para el tipo de mensaje
pero ausentes del mensaje — filas en gris en su posición estándar, anotadas
con grupo, cardinalidad y estado de elección. Expándelas para recorrer la
lista completa de campos hasta los componentes de los tipos compuestos (p.
ej. OBX-16 → componentes XCN). <strong>Clic derecho en un segmento gris →
Insertar segmento</strong> para añadir su esqueleto al mensaje en la posición
estándar, con separadores hasta el último campo obligatorio. El esqueleto usa los separadores
del mensaje, y <kbd>Ctrl</kbd>+<kbd>Z</kbd> lo quita de nuevo.</p>
`,
};
