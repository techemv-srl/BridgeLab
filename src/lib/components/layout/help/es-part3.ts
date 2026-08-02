import type { ManualSection } from '../helpContent';

export const schemaExportSection: ManualSection = {
	id: 'schema-export',
	heading: 'Exportación de esquema (XSD)',
	body: `
<p>¿Necesitas un XSD que describa un mensaje HL7 v2 para un pipeline
basado en XML, una integración contract-first o simplemente para
cargarlo en una herramienta de terceros? Abre <strong>Herramientas →
Exportar esquema del mensaje como XSD…</strong> — elige una versión de
HL7 y un tipo de mensaje, previsualiza el esquema generado y guárdalo
con un clic.</p>

<h3>Qué obtienes</h3>
<p>Un XSD autocontenido que sigue la convención estándar de codificación
HL7 v2.xml:</p>
<ul>
	<li>Un elemento raíz por mensaje (p. ej. <code>ADT_A01</code>) con un
		complex type inline que lista los segmentos y grupos de segmentos
		en orden.</li>
	<li>Cada segmento declarado como <code>xsd:complexType</code> de
		nivel superior (<code>MSH</code>, <code>PID</code>,
		<code>OBX</code>, …) con cada campo tipado según la referencia de
		tipos de dato HL7 (<code>XPN</code>, <code>CX</code>,
		<code>HD</code>, …).</li>
	<li>Tipos de dato compuestos expandidos en sus componentes; tipos de
		dato primitivos (<code>ST</code>, <code>ID</code>, <code>NM</code>, …)
		como restricciones <code>xsd:simpleType</code> sobre
		<code>xsd:string</code>.</li>
	<li>Cardinalidad preservada: <code>minOccurs="0"</code> para los
		campos opcionales, <code>maxOccurs="unbounded"</code> para los
		repetibles.</li>
	<li>Grupos como <code>ORM_O01.ORDER_DETAIL</code> representados con
		la convención de nombres <code>MESSAGE.GROUP</code>; los bloques
		de tipo choice definidos por HL7
		(<code>OBR | RQD | RQ1 | RXO | ODS | ODT</code>) se emiten como
		<code>xsd:choice</code>.</li>
	<li>Compilación garantizada bajo procesadores de esquema estrictos —
		las pocas estructuras HL7 cuya definición viola la regla Unique
		Particle Attribution de XSD se emiten como un choice relajado y
		anotado.</li>
</ul>

<h3>Acciones</h3>
<ul>
	<li><strong>Copiar</strong> — copia el XSD al portapapeles, práctico
		cuando quieres pegarlo en un editor o en un chat.</li>
	<li><strong>Guardar como…</strong> — abre el diálogo de archivos del
		sistema con <code>{MESSAGE}.xsd</code> como nombre por
		defecto.</li>
</ul>

<h3>Cobertura y niveles</h3>
<p>Se incluyen completas siete versiones de HL7: <strong>2.3, 2.3.1,
2.4, 2.5, 2.6, 2.7 y 2.7.1</strong> — 1.916 estructuras de mensaje en
total, seleccionables desde el desplegable de versiones.</p>
<p>El nivel gratuito exporta cuatro tipos de mensaje de alto uso en HL7
v2.5, de modo que el flujo típico de depuración MLLP queda totalmente
cubierto:</p>
<ul>
	<li><strong>ADT^A01</strong> — Admit / Visit Notification</li>
	<li><strong>ADT^A40</strong> — Merge Patient (Patient Identifier
		List)</li>
	<li><strong>ORM^O01</strong> — Order Message</li>
	<li><strong>ORU^R01</strong> — Unsolicited Observation Result</li>
</ul>
<p>Cualquier otro tipo de mensaje, o cualquier otra versión de HL7,
aparece marcado como <strong>(PRO)</strong> en el desplegable y requiere
una licencia Professional (o una prueba activa). Si intentas exportar
una entrada restringida, BridgeLab muestra un aviso de actualización que
apunta a <strong>Ayuda → Activación</strong>.</p>

<h3>Nota sobre licencias</h3>
<p>BridgeLab no redistribuye ningún archivo XSD con copyright de HL7.
Los metadatos de esquema se reconstruyen a partir de las
especificaciones públicas de HL7 v2; cada archivo generado lleva una
cabecera que reconoce a HL7® como estándar de origen y señala la salida
como obra derivada con fines de interoperabilidad.</p>

<div class="info">Destino ideal: Astraia y aplicaciones de integración
similares que aceptan definiciones XSD escritas a mano para tipos de
mensaje que el motor no conoce de forma nativa. Exporta una vez, cárgalo
en el motor y sigue adelante.</div>
`,
};

export const fhirSection: ManualSection = {
	id: 'fhir',
	heading: 'Soporte FHIR',
	body: `
<p>BridgeLab detecta automáticamente los recursos FHIR cuando pegas o
abres un archivo cuyo primer carácter no blanco es <code>{</code> y que
contiene <code>"resourceType"</code>. El árbol cambia a una vista
específica de FHIR que muestra la jerarquía del recurso como rutas
JSON.</p>

<h3>Formatos soportados</h3>
<ul>
	<li><strong>JSON</strong> - Patient, Observation, Bundle, DiagnosticReport,
		MedicationRequest y cualquier otro recurso FHIR R4/R5.</li>
	<li><strong>XML</strong> - los mismos recursos en codificación XML
		(<code>&lt;Patient xmlns="http://hl7.org/fhir"&gt;</code>).</li>
</ul>

<h3>Visualizador de Bundle (Pro)</h3>
<p><strong>Herramientas → Visualizador de Bundle FHIR</strong> abre una
vista de tres paneles cuando el mensaje activo es un Bundle:</p>
<ul>
	<li><strong>Panel izquierdo:</strong> lista de entradas con tipo de
		recurso, nombre visible (p. ej. nombre del Patient, código de la
		Observation) y un contador de referencias entrantes.</li>
	<li><strong>Panel central:</strong> referencias salientes de la
		entrada seleccionada - cada campo <code>reference</code> se
		convierte en un enlace clicable que navega hasta la entrada de
		destino.</li>
	<li><strong>Panel derecho:</strong> el JSON en bruto del recurso
		seleccionado, con resaltado de sintaxis.</li>
</ul>
<p>Las <strong>referencias colgantes</strong> (que apuntan a entradas no
presentes en el Bundle) se señalan con una insignia roja.</p>
<p>El conmutador <strong>Lista / Grafo</strong> cambia a un grafo de
referencias: cada entrada es un nodo (coloreado por tipo de recurso) y
cada <code>reference</code> una flecha dirigida. Haz clic en un nodo
para seleccionarlo — el panel de detalle lo sigue. Disponible hasta 150
entradas; los bundles más grandes usan la lista.</p>

<h3>Evaluador FHIRPath (Pro)</h3>
<p><kbd>Ctrl</kbd>+<kbd>P</kbd> o <strong>Herramientas → Evaluador FHIRPath</strong>
abre una consola interactiva donde escribes expresiones FHIRPath contra
el recurso actual. Entre los operadores soportados:</p>
<ul>
	<li><strong>Navegación:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code></li>
	<li><strong>Indexación:</strong> <code>Patient.name[0].given</code></li>
	<li><strong>Filtros:</strong>
		<code>Bundle.entry.where(resource.resourceType = 'Patient')</code></li>
	<li><strong>Agregados:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>,
		<code>distinct()</code></li>
	<li><strong>Proyección:</strong>
		<code>Bundle.entry.select(resource.id)</code></li>
</ul>
<p>Las expresiones recientes se conservan en un desplegable de historial
para reutilizarlas rápidamente.</p>

<h3>Validación FHIR</h3>
<p>F6 también funciona con los recursos FHIR. Los errores resaltan
campos obligatorios ausentes (p. ej. <code>Patient.identifier</code>),
tipos de dato no válidos (gender fuera del value set) y problemas
estructurales. Las URL canónicas declaradas en <code>meta.profile</code>
se listan como hallazgos informativos (la conformidad con el perfil en
sí no se comprueba); las entradas malformadas se señalan como
advertencias.</p>

<h3>Plantillas FHIR</h3>
<p><strong>Archivo → Nuevo desde plantilla</strong> incluye una
categoría FHIR: un Patient mínimo, una Observation de presión arterial
con componentes y un Bundle de tipo transaction cuyas entradas se
referencian entre sí mediante <code>urn:uuid</code> — ábrelo y prueba la
vista de grafo del visualizador de Bundle.</p>
`,
};

export const pluginsSection: ManualSection = {
	id: 'plugins',
	heading: 'Packs de plugins',
	body: `
<p>Los packs de plugins te permiten ampliar el validador y el
anonimizador de BridgeLab <strong>sin escribir código</strong> y sin
permitir ninguna ejecución de código. Cada pack es un archivo JSON que
se coloca en una carpeta de usuario.</p>

<h3>Dónde viven los plugins</h3>
<p>Haz clic en <strong>Configuración → Plugins → Abrir carpeta de
plugins</strong> para mostrar el directorio en tu gestor de archivos. La
estructura es:</p>
<pre><code>&lt;config&gt;/BridgeLab/plugins/
├── validation/
│   ├── hospital-adt-rules.json
│   └── z-segment-checks.json
└── anonymization/
    └── eu-national-id.json</code></pre>

<p>En Windows la raíz es <code>%APPDATA%\\BridgeLab\\plugins</code>, en
macOS <code>~/Library/Application Support/BridgeLab/plugins</code>, en
Linux <code>~/.config/BridgeLab/plugins</code>.</p>

<h3>Pack de reglas de validación</h3>
<pre><code>{
  "id": "acme-adt-01",
  "name": "ACME ADT specific rules",
  "description": "Hospital-specific required fields",
  "version": "1.0",
  "enabled": true,
  "validation_rules": [
    {
      "rule_id": "ACME-PID-001",
      "severity": "error",
      "segment": "PID",
      "field": 3,
      "check": { "type": "not_empty" },
      "message": "PID-3 (Patient ID) is required"
    }
  ]
}</code></pre>

<h3>Tipos de comprobación soportados</h3>
<table>
	<tr><th>Comprobación</th><th>Parámetros</th><th>Ejemplo de uso</th></tr>
	<tr><td><code>not_empty</code></td><td>—</td>
		<td>El campo debe estar relleno.</td></tr>
	<tr><td><code>regex</code></td><td><code>pattern</code></td>
		<td>El apellido debe empezar por mayúscula.</td></tr>
	<tr><td><code>one_of</code></td><td><code>values[]</code></td>
		<td>La clase de paciente debe ser I, O, E.</td></tr>
	<tr><td><code>max_length</code></td><td><code>max</code></td>
		<td>MRN ≤ 16 caracteres.</td></tr>
	<tr><td><code>min_length</code></td><td><code>min</code></td>
		<td>SSN ≥ 9 dígitos.</td></tr>
	<tr><td><code>contains</code></td><td><code>value</code></td>
		<td>El número de visita debe contener un guion.</td></tr>
</table>
<p>Añade <code>"component": 1</code> para restringir una regla a un
componente concreto (p. ej. el apellido dentro de PID-5.1).</p>

<h3>Pack de reglas de anonimización</h3>
<pre><code>{
  "id": "eu-extra-phi",
  "name": "EU extra PHI fields",
  "enabled": true,
  "phi_rules": [
    { "segment": "PID", "field": 25, "sensitivity": "high",
      "name": "EU National ID" }
  ]
}</code></pre>

<h3>Gestión de los packs</h3>
<p><strong>Configuración → Plugins</strong> lista cada pack con su
autor, versión, número de reglas y ruta. Activa o desactiva packs
individuales (la elección se conserva), haz clic en <em>Recargar</em>
tras editar un archivo, o en <em>Abrir carpeta de plugins</em> para
editarlos en tu IDE favorito.</p>

<div class="note">Los archivos que no se pueden analizar aparecen con un
banner de error rojo, pero no rompen el registro - el resto de tus packs
sigue funcionando.</div>

<p class="note">En el nivel Community puede haber hasta <strong>3
packs</strong> activos a la vez: los packs habilitados de más muestran
una insignia de "inactivo" y no aportan reglas hasta que se libera un
hueco (desactiva otro pack o actualiza tu licencia).</p>
`,
};

export const licensingSection: ManualSection = {
	id: 'licensing',
	heading: 'Licencias',
	body: `
<p>BridgeLab se distribuye con tres niveles. El reparto de
funcionalidades está pensado para que los usuarios Community puedan
hacer trabajo HL7 real del día a día para siempre, mientras que Pro y
Enterprise desbloquean las funciones que necesitan los equipos de
integración y los hospitales.</p>

<table>
	<tr><th>Funcionalidad</th><th>Community</th><th>Pro</th><th>Enterprise</th></tr>
	<tr><td>Editor HL7 v2.x, analizador, validación</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Análisis FHIR + vista de árbol</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Envío MLLP, HTTP GET</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Detección de PHI (solo visualización)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Packs de plugins (básico)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Listener MLLP</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE + autenticación</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Enmascaramiento de anonimización</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Exportación JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Evaluador FHIRPath + Visualizador de Bundle</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Plugins y casos de prueba ilimitados</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>SOAP + soporte prioritario</td>
		<td>—</td><td>—</td><td>✓</td></tr>
</table>

<p class="note">Community conserva hasta <strong>3 packs de plugins
activos</strong> y <strong>10 casos de prueba guardados</strong>. Nada
se bloquea ni se elimina nunca: los elementos guardados por encima del
límite (p. ej. durante una prueba) siguen visibles, editables y
ejecutables — solo los guardados y activaciones nuevos por encima del
límite piden una actualización, y liberar un hueco los reactiva de
inmediato.</p>

<h3>Prueba</h3>
<p>El primer arranque inicia una <strong>prueba Pro de 7 días</strong>
con todas las funciones Pro habilitadas. El banner de prueba (amarillo)
se puede descartar; cuando quedan 3 días se vuelve rojo y permanece
visible como recordatorio.</p>

<p>Cuando la prueba expira, BridgeLab <strong>no deja de
funcionar</strong> - vuelve al nivel Community y el banner te invita a
actualizar. Tus mensajes, tu configuración, tus plugins y tus casos de
prueba permanecen intactos.</p>

<h3>Activación</h3>
<p>Abre el diálogo de activación desde:</p>
<ul>
	<li><strong>Configuración → Licencia → Activar</strong></li>
	<li><strong>Ayuda → Activar Licencia</strong></li>
	<li>El botón <em>Actualizar</em> del banner de prueba</li>
</ul>

<p>Para obtener una clave de licencia, escribe a
<a href="mailto:info@techemv.it">info@techemv.it</a> con tu <strong>ID
de Hardware</strong> (se muestra en el diálogo de activación y también
en Configuración → Licencia). TECHEMV SRL genera una licencia firmada
vinculada a tu máquina y te la devuelve por correo. Pégala en el campo
de la clave; el diálogo muestra el nombre del titular y las funciones
incluidas antes de la activación.</p>

<h3>Verificación offline</h3>
<p>Tras la primera activación, la verificación de la licencia es
puramente local - no se necesita ninguna llamada de red. La clave lleva
una firma Ed25519 que la aplicación verifica contra una clave pública
integrada.</p>
`,
};

export const shortcutsSection: ManualSection = {
	id: 'shortcuts',
	heading: 'Atajos de teclado',
	body: `
<p>Los atajos de BridgeLab son configurables por el usuario en
<strong>Configuración → Atajos de Teclado</strong>. Haz clic en
cualquier asignación, pulsa una combinación de teclas nueva y confirma
con OK.</p>

<h3>Valores predeterminados</h3>
<table>
	<tr><td><kbd>Ctrl</kbd>+<kbd>O</kbd></td><td>Abrir archivo</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>N</kbd></td><td>Nuevo desde plantilla</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>L</kbd></td><td>Biblioteca de casos de prueba</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>S</kbd></td><td>Guardar</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd></td><td>Guardar como</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>W</kbd></td><td>Cerrar pestaña</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>B</kbd></td><td>Mostrar/ocultar árbol</td></tr>
	<tr><td><kbd>F5</kbd></td><td>Re-analizar mensaje</td></tr>
	<tr><td><kbd>F6</kbd></td><td>Validar</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>K</kbd></td><td>Panel de comunicación</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>P</kbd></td><td>Panel FHIRPath</td></tr>
	<tr><td><kbd>Ctrl</kbd>+<kbd>,</kbd></td><td>Configuración</td></tr>
	<tr><td><kbd>F1</kbd></td><td>Este manual de usuario</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>T</kbd></td><td>Mostrar segmento en el árbol (menú contextual del editor)</td></tr>
	<tr><td><kbd>Alt</kbd>+<kbd>C</kbd></td><td>Copiar segmento (menú contextual del editor)</td></tr>
</table>

<h3>Detección de conflictos</h3>
<p>Si eliges una combinación de teclas ya asignada a otra acción, el
editor te avisa - confirma para transferir la asignación o elige una
tecla diferente. Los atajos propios de Monaco
(<kbd>Ctrl</kbd>+<kbd>F</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd>, ...) tienen
prioridad cuando el editor tiene el foco.</p>

<h3>Restablecer</h3>
<p>Haz clic en <em>Restablecer todo</em> para devolver cada atajo a su
valor predeterminado, o en el pequeño botón ↺ junto a cada entrada para
restablecer solo esa.</p>
`,
};
