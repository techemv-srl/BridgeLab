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
<p>Se incluyen completas diez versiones de HL7: <strong>2.1, 2.2, 2.3,
2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7 y 2.7.1</strong> — 2.320 estructuras
de mensaje seleccionables desde el desplegable de versiones.</p>
<p>HL7 v2.7.1 es una versión de corrección técnica de la 2.7 y usa las
mismas definiciones de mensaje, por lo que aparece marcada
<strong>(= v2.7)</strong> en el desplegable y exporta desde el catálogo
2.7.</p>
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
apunta a <strong>Ayuda → Comprar una licencia</strong> (la página de precios) o
<strong>Ayuda → Activar Licencia</strong>.</p>

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
el recurso actual.</p>
<p>El evaluador implementa el lenguaje FHIRPath 2.0: el conjunto
completo de operadores con la precedencia y la lógica de tres valores de
la especificación, y unas setenta funciones.</p>
<ul>
	<li><strong>Navegación:</strong> <code>Patient.name.family</code>,
		<code>Bundle.entry.resource</code>; los elementos choice se
		alcanzan por su nombre base — <code>Observation.value</code>
		encuentra <code>valueQuantity</code></li>
	<li><strong>Indexación:</strong> <code>Patient.name[0].given</code></li>
	<li><strong>Filtros y proyección:</strong> <code>where()</code>,
		<code>select()</code>, <code>repeat()</code>,
		<code>ofType()</code>, con <code>$this</code> e
		<code>$index</code> disponibles dentro</li>
	<li><strong>Colecciones:</strong> <code>count()</code>,
		<code>first()</code>, <code>last()</code>, <code>tail()</code>,
		<code>skip()</code>, <code>take()</code>, <code>distinct()</code>,
		<code>sort()</code>, <code>union()</code>, <code>combine()</code>,
		<code>intersect()</code>, <code>exclude()</code>,
		<code>aggregate()</code></li>
	<li><strong>Lógica:</strong> <code>and</code>, <code>or</code>,
		<code>xor</code>, <code>implies</code>, <code>not()</code>,
		<code>exists()</code>, <code>all()</code>, <code>iif()</code> —
		con la colección vacía como valor «desconocido»</li>
	<li><strong>Cadenas:</strong> <code>substring()</code>,
		<code>matches()</code>, <code>replace()</code>,
		<code>split()</code>, <code>join()</code>, <code>encode()</code>,
		<code>escape()</code> y afines</li>
	<li><strong>Fechas y cantidades:</strong> literales de precisión
		parcial (<code>@2015</code>,
		<code>@2015-02-04T14:34:28+10:00</code>), aritmética con
		duraciones (<code>Patient.birthDate + 18 years</code>) y
		conversión de unidades dentro de una dimensión
		(<code>4 'g' = 4000 'mg'</code>)</li>
	<li><strong>Extras FHIR:</strong> <code>extension(url)</code>,
		<code>hasValue()</code> y <code>resolve()</code>, que sigue una
		Reference hacia un recurso contained o del Bundle</li>
	<li><strong>Depuración:</strong> <code>trace('etiqueta')</code> deja
		pasar su entrada y muestra los valores bajo el resultado, para ver
		qué produce una ruta larga a mitad de camino</li>
</ul>
<p>Comparar valores de distinta precisión devuelve la colección vacía en
lugar de adivinar: <code>@2015-02-04 = @2015-02</code> no es ni
verdadero ni falso, porque el segundo valor podría ser ese día u otro
del mismo mes.</p>
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

<h3>Validación de perfiles (Pro)</h3>
<p>Por defecto un recurso FHIR se comprueba estructuralmente: si hay
<code>resourceType</code> y si se cumplen las reglas propias del recurso.
Contrastarlo con una <strong>StructureDefinition</strong> — la definición
real de lo que puede contener un Patient — requiere esas definiciones, que
se distribuyen como paquetes FHIR NPM.</p>
<p>El núcleo FHIR R4 (<code>hl7.fhir.r4.core</code> 4.0.1) está
<strong>integrado</strong>: las definiciones base están siempre — sin
conexión, en todas las ediciones, nada que descargar. <strong>Herramientas
→ Paquetes de perfiles FHIR…</strong> instala encima, desde un
<code>.tgz</code>, las guías de implementación nacionales o propias de tu
centro; un paquete instalado con las mismas definiciones sustituye a las
integradas, una versión más reciente las supera.</p>
<p>Cada validación FHIR comprueba, contra el núcleo integrado y lo que esté
instalado encima:</p>
<ul>
	<li><strong>Cardinalidad</strong> — un elemento obligatorio ausente, o un
		elemento <code>0..1</code> que se repite.</li>
	<li><strong>Tipos de elementos</strong> — un booleano escrito como
		cadena, un número donde corresponde un objeto.</li>
	<li><strong>Elementos choice</strong> — <code>value[x]</code> debe
		aparecer exactamente como una de <code>valueQuantity</code>,
		<code>valueString</code>, etc. Un <code>value</code> pelado, o dos
		formas a la vez, se informan.</li>
	<li><strong>Valores fijos y patrones</strong> — lo que el perfil
		fija.</li>
	<li><strong>Elementos desconocidos</strong> — un nombre que el perfil no
		define. Es la comprobación que caza una errata como
		<code>genderr</code> o un elemento que pertenece a otro tipo de
		recurso.</li>
</ul>
<p>Los perfiles declarados en <code>meta.profile</code> se aplican
automáticamente si el paquete que los define está instalado. Si no lo está,
el validador lo dice en lugar de informar en silencio un resultado limpio —
y cuando los perfiles sí se aplicaron, también lo dice, porque «sin
hallazgos» significa cosas muy distintas en cada caso.</p>
<p><strong>La terminología queda fuera del alcance.</strong> Un binding
<code>required</code> solo puede comprobarse expandiendo el ValueSet, lo que
implica los paquetes de terminología o un servidor. BridgeLab deja los
bindings sin comprobar en lugar de comprobarlos a medias.</p>
<p>Instalar un paquete requiere una licencia Professional. Los paquetes ya
instalados siguen validando en todos los niveles: una prueba caducada nunca
pone en rojo recursos que antes estaban limpios.</p>

<h3>Reglas FHIR personalizadas (Pro)</h3>
<p><strong>Herramientas → Reglas de validación FHIR…</strong> abre un editor
para tus propias comprobaciones. Se ejecutan junto a las integradas en cada
validación y se guardan como un plugin pack normal
(<code>plugins/fhir/user-rules.json</code>) que puedes copiar entre equipos o
versionar.</p>
<p>Una regla tiene una de dos formas:</p>
<ul>
	<li><strong>La expresión debe ser verdadera</strong> — un invariante
		FHIRPath, como FHIR escribe sus propias restricciones:
		<code>identifier.exists()</code>, o
		<code>value.exists() xor dataAbsentReason.exists()</code>.</li>
	<li><strong>Ruta + comprobación</strong> — un selector FHIRPath y algo
		que afirmar sobre los valores obtenidos: debe estar presente,
		cuántos, coincide con un patrón, uno de una lista, contiene un texto
		o un límite de longitud.</li>
</ul>
<p>Indica <strong>Tipo de recurso</strong> para limitar la regla a Patient,
Observation, etc., o déjalo vacío para aplicarla a todo. Una regla limitada
a un tipo también se dispara para los recursos correspondientes dentro de un
Bundle, informando el hallazgo en <code>entry[n].resource.…</code>.</p>
<p><strong>Probar en el recurso abierto</strong> ejecuta la regla que estás
editando sobre el recurso de la pestaña activa antes de guardarla, y muestra
qué valores seleccionó realmente — la forma más rápida de distinguir una
regla que pasa de una que nunca se ejecutó. Si el recurso abierto es de otro
tipo, el editor lo dice en lugar de informar un éxito.</p>
<p>Las reglas ya escritas funcionan en todos los niveles; el editor requiere
una licencia Professional. Los packs escritos a mano están documentados en
<code>docs/PLUGINS.md</code>.</p>

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
├── fhir/
│   └── user-rules.json
└── anonymization/
    └── eu-national-id.json</code></pre>

<p>En Windows la raíz es <code>%APPDATA%\\BridgeLab\\plugins</code>, en
macOS <code>~/Library/Application Support/BridgeLab/plugins</code>, en
Linux <code>~/.config/BridgeLab/plugins</code>.</p>

<p>Allí viven tres tipos de pack: reglas de validación HL7 v2
(<code>validation/</code>, abajo), reglas de validación FHIR
(<code>fhir/</code> — un invariante FHIRPath, o un selector más una
comprobación; véase <em>Soporte FHIR → Reglas FHIR
personalizadas</em>) y campos PHI adicionales para el anonimizador
(<code>anonymization/</code>, abajo).</p>

<p><strong>Todos los niveles tienen el mecanismo completo</strong> — los
tres tipos, cada comprobación, recarga y activación por pack. La única
diferencia es cuántos packs pueden estar activos a la vez: hasta
<strong>3</strong> en Community, ilimitados en Pro y Enterprise. El editor
integrado que escribe packs FHIR es Pro; un pack FHIR escrito a mano se
ejecuta en Community como cualquier otro.</p>

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
	<tr><td>Envío MLLP, HTTP GET sin cabecera de autenticación</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Conformidad con el núcleo FHIR R4 (integrado)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Exportación XSD</td>
		<td>4 mensajes v2.5</td><td>Catálogo completo</td><td>Catálogo completo</td></tr>
	<tr><td>Detección de PHI (solo visualización)</td>
		<td>✓</td><td>✓</td><td>✓</td></tr>
	<tr><td>Packs de plugins (todos los tipos, todas las comprobaciones)</td>
		<td>3 activos a la vez</td><td>Ilimitados</td><td>Ilimitados</td></tr>
	<tr><td>Listener MLLP</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>HTTP POST/PUT/DELETE/PATCH, y autenticación en cualquier método</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Enmascaramiento de anonimización</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Exportación JSON/CSV</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Paquetes de perfiles FHIR y editor de reglas</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Evaluador FHIRPath + Visualizador de Bundle</td>
		<td>—</td><td>✓</td><td>✓</td></tr>
	<tr><td>Casos de prueba guardados</td>
		<td>10</td><td>Ilimitados</td><td>Ilimitados</td></tr>
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
<p>El primer arranque inicia una <strong>prueba Pro de 14 días</strong>
con todas las funciones Pro habilitadas. El banner de prueba (amarillo)
se puede descartar; cuando quedan 3 días se vuelve rojo y permanece
visible como recordatorio.</p>

<p>Cuando la prueba expira, BridgeLab <strong>no deja de
funcionar</strong> - vuelve al nivel Community y el banner te invita a
actualizar. Tus mensajes, tu configuración, tus plugins y tus casos de
prueba permanecen intactos.</p>

<h3>Actualizaciones</h3>
<p>No se consulta nada hasta que decidas. La primera vez que se inicia,
BridgeLab pregunta, con un banner en la parte superior de la ventana, si
puede buscar nuevas versiones (el instalador de Windows lo pregunta
durante la instalación, y entonces la aplicación no vuelve a hacerlo).
<em>Sí, comprobar</em> y <em>No</em> se guardan en Configuración →
Privacidad; cerrar el banner sin responder vuelve a preguntar en el
siguiente inicio.</p>
<p>Una vez al día, unos segundos después de iniciar, BridgeLab consulta a
GitHub (<code>api.github.com</code>) la última versión. Si hay una más
reciente aparece un banner con <em>Descargar</em> (abre la página de la
versión), <em>Omitir esta versión</em> y un botón de cierre; no se instala
nada automáticamente. La consulta no contiene nada sobre ti, el equipo o
tus archivos, y sin acceso a internet se omite en silencio. Se desactiva
en <strong>Configuración → Privacidad → Buscar nuevas versiones al
iniciar</strong>; <strong>Ayuda → Buscar actualizaciones</strong> funciona
siempre bajo demanda.</p>
<p>El instalador de Windows hace la misma pregunta en la primera
instalación (<em>Sí</em> por defecto; una instalación silenciosa no
pregunta). En equipos gestionados el administrador puede desactivar la
comprobación para todos los usuarios, y la casilla de Configuración
aparece bloqueada: variable de entorno
<code>BRIDGELAB_DISABLE_UPDATE_CHECK=1</code>, o un archivo
<code>policy.json</code> con <code>{"disable_update_check": true}</code> en
<code>%ProgramData%\\BridgeLab\\</code> (Windows),
<code>/Library/Application Support/BridgeLab/</code> (macOS) o
<code>/etc/bridgelab/</code> (Linux).</p>

<h3>Comprar una licencia</h3>
<p><strong>Ayuda → Comprar una licencia…</strong> abre en el navegador
la sección de precios del sitio de BridgeLab, donde Professional y
Enterprise se compran en línea con tarjeta; el código de activación
llega por correo. La misma página está a un clic de los botones
<em>Precios y compra</em> del diálogo de activación, del botón
<em>Comparar planes</em> del banner de prueba y del botón <em>Ver
precios</em> de cada aviso «requiere una licencia Professional».
¿Necesitas factura, orden de compra o presupuesto? Escribe a
<a href="mailto:info@techemv.it">info@techemv.it</a>.</p>

<h3>Activación</h3>
<p>Abre el diálogo de activación desde:</p>
<ul>
	<li><strong>Configuración → Licencia → Activar</strong></li>
	<li><strong>Ayuda → Activar Licencia</strong></li>
	<li>El botón <em>Actualizar</em> del banner de prueba</li>
</ul>

<p><strong>Activación en línea (predeterminada):</strong> tras la
compra recibes por correo un código de activación del tipo
<code>BL-PRO-XXXX-XXXX-XXXX</code>. Pégalo en el campo de la clave: la
aplicación lo intercambia con una sola llamada HTTPS por una licencia
firmada vinculada a este equipo. Usa <em>Desactivar</em> para liberar
el puesto antes de cambiar de ordenador.</p>

<p>&iquest;Has renovado la suscripci&oacute;n? Pulsa <em>Actualizar
licencia</em> en el di&aacute;logo para recuperar al instante la nueva
fecha — o no hagas nada: en los 14 d&iacute;as previos al vencimiento
la aplicaci&oacute;n la recupera sola al arrancar, en silencio (nunca
errores en equipos sin conexi&oacute;n).</p>

<p><strong>Clave sin conexión (entornos aislados / air-gapped):</strong>
escribe a <a href="mailto:info@techemv.it">info@techemv.it</a> con tu
<strong>ID de Hardware</strong> (se muestra bajo "¿Necesitas una clave
sin conexión?" en el diálogo de activación y también en
Configuración → Licencia). TECHEMV SRL te devuelve una licencia firmada
vinculada a tu máquina — nunca se necesita acceso a internet. El
diálogo muestra el nombre del titular y las funciones incluidas antes
de la activación.</p>

<h3>Verificación offline</h3>
<p>Sea cual sea el flujo utilizado, la verificación ordinaria de la
licencia es puramente local - la aplicación nunca necesita contactar
con el servidor de licencias para seguir funcionando. Las llamadas al
servidor solo ocurren cuando las desencadenas explícitamente:
activación con código, liberación del puesto con <em>Desactivar</em>,
o estadísticas de uso opt-in. La clave lleva una firma Ed25519 que la
aplicación verifica contra una clave pública integrada.</p>

<h3>Privacidad y estadísticas de uso</h3>
<p>BridgeLab puede enviar <strong>estadísticas de uso</strong> a
TECHEMV — desactivadas por defecto, activables en
<strong>Configuración → Privacidad</strong>. Si están activas, el
envío automático ocurre como máximo una vez al día; el botón
<em>Enviar ahora</em> transmite de inmediato. Cada informe contiene
contadores de uso, versión de la aplicación, sistema operativo, tipo
de licencia, un <strong>ID de instalación aleatorio</strong> y — solo
para licencias activadas en línea — el <strong>código de
activación</strong> (usado para señalar una licencia revocada). Los
datos son por tanto <strong>seudónimos</strong>, no totalmente
anónimos: nunca se envían contenidos de mensajes, nombres de archivo,
nombres de host, nombres de usuario ni datos de pacientes, y el JSON
exacto puede inspeccionarse con <em>Mostrar lo que se envía</em>. Con
el interruptor apagado (el valor por defecto) no se transmite nada, y
los problemas de red nunca producen errores: una instalación
totalmente sin conexión es un escenario normal y soportado.</p>
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
