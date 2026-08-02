import type { ManualSection } from '../helpContent';
import { mockupValidationEs as mockupValidation, mockupCommunicationEs as mockupCommunication } from './mockups';

export const validationSection: ManualSection = {
	id: 'validation',
	heading: 'Validación',
	body: `
<p>Pulsa <kbd>F6</kbd> o elige <strong>Herramientas → Validar</strong>
para ejecutar todas las reglas de validación sobre el mensaje activo.
Los resultados aparecen en el panel de Validación acoplado en la parte
inferior, agrupados por severidad.</p>

${mockupValidation}

<h3>Reglas integradas</h3>
<ul>
	<li><strong>Estructurales:</strong> el primer segmento debe ser MSH;
		los códigos de segmento deben tener 3 caracteres alfanuméricos;
		sin MSH duplicado.</li>
	<li><strong>Cabecera MSH:</strong> MSH-9 (tipo de mensaje), MSH-10
		(ID de control), MSH-12 (versión) son obligatorios.</li>
	<li><strong>Campos requeridos:</strong> campos obligatorios por
		segmento según el estándar HL7 (p. ej. PID-3 Patient Identifier
		List).</li>
	<li><strong>Límites de longitud:</strong> avisa cuando un campo
		supera el <code>max_length</code> publicado.</li>
	<li><strong>Tipos de dato:</strong> los campos numéricos (SI, NM) se
		comprueban en busca de caracteres no numéricos; los formatos de
		marca de tiempo (TS) se comprueban por longitud y composición de
		solo dígitos.</li>
</ul>

<h3>Filtrado y navegación</h3>
<p>Haz clic en las insignias Error / Advertencia / Info para filtrar.
Haz clic en cualquier fila de problema para saltar al segmento afectado
en el editor.</p>

<h3>Reglas personalizadas desde packs de plugins</h3>
<p>Coloca un archivo JSON en
<code>&lt;config&gt;/BridgeLab/plugins/validation/</code> para añadir
tus propias comprobaciones sin recompilar. Consulta <em>Plugins</em> más
abajo.</p>

<h3>Validación por lotes (Pro)</h3>
<p><strong>Herramientas → Validación por lotes…</strong> valida una
carpeta entera (o un conjunto elegido a mano) de archivos
<code>.hl7</code>/<code>.txt</code>/<code>.dat</code> en una sola
pasada: una fila por archivo con tipo de mensaje, versión, número de
segmentos y totales de errores/advertencias. Filtra solo los fallos, haz
clic en una fila para abrir ese archivo en el editor y exporta la tabla
completa como CSV para el ticket de revisión de cambios. Los archivos se
procesan en memoria — no se añade nada a tus pestañas.</p>

<h3>Generador de mensajes de prueba</h3>
<p><strong>Herramientas → Generar mensajes de prueba…</strong> crea
mensajes ADT/ORU/ORM sintácticamente válidos con datos de paciente
<em>sintéticos</em> verosímiles — nombres, fechas de nacimiento, MRN,
direcciones y paneles de laboratorio con rangos de referencia (una
proporción realista de los resultados es deliberadamente anómala y va
marcada). Nunca se usa PHI real. Proporciona una <strong>semilla</strong>
para hacer reproducible un conjunto y luego abre los mensajes en
pestañas o guárdalos en una carpeta como archivos <code>.hl7</code>
numerados — fixtures de regresión instantáneos para el validador por
lotes de arriba.</p>

<h3>Validación por CLI</h3>
<p>El complemento <code>bridgelab-cli</code> ofrece el mismo validador
para uso sin interfaz (pipelines de CI, cribado por lotes):</p>
<pre><code>bridgelab-cli validate message.hl7
bridgelab-cli validate '*.hl7' --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Comunicación (MLLP / HTTP)',
	body: `
<p>Abre el panel de comunicación inferior con <kbd>Ctrl</kbd>+<kbd>K</kbd>
o <strong>Herramientas → Panel de comunicación</strong>. Tres pestañas:
MLLP, HTTP e Historial.</p>

${mockupCommunication}

<h3>Cliente MLLP</h3>
<ol>
	<li>Introduce <em>Host</em> + <em>Puerto</em> (p. ej.
		<code>localhost:2575</code>).</li>
	<li>Se usa automáticamente el mensaje de la pestaña activa.</li>
	<li>Haz clic en <strong>Enviar</strong>. El framing (<code>0x0B</code> ... <code>0x1C 0x0D</code>),
		el transporte y la espera del ACK los gestiona el backend en
		Rust.</li>
	<li>El ACK aparece en el área de resultados con el tiempo de ida y
		vuelta. <em>Accept</em> (AA), <em>Error</em> (AE) y <em>Reject</em>
		(AR) se muestran todos con el <code>MSA|AA|{control-id}</code>
		original.</li>
</ol>

<h3>Generador de ACK</h3>
<p>La fila <strong>Generador de ACK</strong> de la pestaña MLLP
construye un acuse de recibo para el mensaje que está actualmente en el
editor: elige el código (AA aceptar, AE error, AR rechazar) y haz clic
en <strong>Generar ACK</strong>. El Message Control ID se lee de MSH-10
(respetando el separador de campo declarado en MSH-1) y el ACK
resultante se abre en una pestaña nueva — listo para devolverlo o
conservarlo como fixture. Si el mensaje actual no tiene MSH-10, el
generador se niega en lugar de producir un ACK imposible de
correlacionar.</p>

<h3>Listener MLLP (Pro)</h3>
<p>Haz clic en <strong>Iniciar escucha</strong> para ejecutar un
servidor en el puerto seleccionado. Los mensajes entrantes se abren en
una pestaña nueva (desactivable, ver más abajo) y se devuelve un ACK
automático con el código configurado (AA/AE/AR). Úsalo para validar
rápidamente lo que está emitiendo tu sistema origen.</p>

<h3>Consola del listener</h3>
<p>Mientras el listener está en marcha, cada mensaje recibido aparece
como una fila en la consola: hora local, dirección del par, tamaño de la
carga, el código ACK que realmente se devolvió (<code>AA</code> en
verde, <code>AE</code>/<code>AR</code> en rojo, — cuando el auto-ACK
está desactivado), la codificación de caracteres usada y la primera
línea del mensaje. <strong>Haz clic en una fila para reabrir ese mensaje
en una pestaña.</strong> Los errores del listener aparecen en línea como
filas rojas.</p>
<p>El interruptor <em>"Abrir los mensajes recibidos en una pestaña
nueva"</em> (activado por defecto) puede desactivarse durante pruebas de
alto volumen: los mensajes quedan entonces solo en la consola y tú
escoges los que necesitas.</p>
<p class="note">El contenido completo de los mensajes que se conserva
para abrirlos con un clic está limitado a un presupuesto rotatorio de
32&nbsp;MB. En sesiones largas sin supervisión, las filas más antiguas
pierden su contenido completo (aparecen atenuadas) — la fila de
metadatos permanece y no se ha "perdido" nada: solo se liberó la copia
de apertura con clic para mantener acotada la memoria.</p>

<h3>Codificación de caracteres</h3>
<p>Tanto el emisor como el listener tienen un selector de
<strong>Codificación</strong> que cubre los juegos de caracteres vistos
en despliegues reales: <code>UTF-8</code>, <code>ISO-8859-1</code>
(Latin-1), <code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> y <code>ASCII</code>. El valor por defecto es
UTF-8 con retorno automático a Latin-1, que acepta la mayor parte del
tráfico heredado incluso cuando MSH-18 está vacío. El listener
recodifica su ACK con el mismo juego de caracteres para que el par nunca
vea mojibake. Las codificaciones de envío y de recepción son
independientes.</p>

<h3>HTTP</h3>
<p>Las peticiones GET están disponibles en el nivel Community.
POST/PUT/DELETE, las cabeceras de autenticación personalizadas (Basic,
Bearer) y el seguimiento de redirecciones requieren Pro. El cuerpo por
defecto es el mensaje de la pestaña actual, pero puede
sobrescribirse.</p>

<h3>Historial</h3>
<p>Cada envío y cada recepción quedan registrados (host, puerto, tamaño,
código de respuesta, tiempo de ida y vuelta). Las últimas 100 entradas
se conservan entre reinicios; haz clic en cualquier fila para ver la
petición y la respuesta completas.</p>

<h3>Perfiles de conexión</h3>
<p>Guarda los endpoints de uso frecuente como perfiles con nombre desde
la fila <strong>Perfil</strong>: escribe un nombre y haz clic en
<em>Guardar</em>. Los perfiles MLLP almacenan host, puerto, tiempo de
espera y auto-ACK; los perfiles HTTP almacenan URL, cabeceras y tiempo
de espera. Al seleccionar un perfil, este se aplica al formulario;
guardar con un nombre existente lo sobrescribe; <em>Eliminar</em> borra
el seleccionado. Los perfiles se guardan en la base de datos local y
sobreviven a los reinicios.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonimización y exportación',
	body: `
<p><strong>Herramientas → Anonimizar</strong> detecta los campos PHI en
los segmentos habituales de identificación del paciente (PID, NK1, IN1,
GT1) y los enmascara según su nivel de sensibilidad.</p>

<table>
	<tr><th>Nivel</th><th>Ejemplo</th><th>Estrategia</th></tr>
	<tr><td><strong>Alto</strong></td><td>Nombre del paciente, SSN, MRN</td>
		<td>El texto pasa a ser <code>REDACTED</code>; lo numérico pasa a
		ceros de la misma longitud (preservando el ancho del campo para
		los parsers posteriores).</td></tr>
	<tr><td><strong>Medio</strong></td><td>Apellido de soltera de la madre,
		teléfono</td>
		<td>Se conserva el primer carácter, el resto se sustituye por
		<code>***</code>.</td></tr>
	<tr><td><strong>Bajo</strong></td><td>Alias, identificadores de bajo
		riesgo</td>
		<td>Se conservan los 3 primeros caracteres, el resto se sustituye
		por <code>...</code>.</td></tr>
</table>

<p>El diálogo lista todos los campos PHI detectados antes de ejecutar el
enmascarado, de modo que puedas revisar qué va a cambiar. La salida:</p>
<ul>
	<li><strong>Se abre en una pestaña nueva</strong> - el mensaje
		original permanece intacto en su propia pestaña.</li>
	<li><strong>Puede copiarse al portapapeles</strong> directamente.</li>
	<li><strong>Preserva la estructura</strong> - el orden de los
		segmentos, el número de pipes y los separadores de componentes no
		cambian, por lo que el resultado sigue analizándose como HL7
		válido.</li>
</ul>

<h3>Campos PHI personalizados mediante plugins</h3>
<p>Los despliegues con identificadores regionales o específicos de un
proveedor (documento nacional de identidad de la UE, campos internos en
Z-segments) pueden ampliar el catálogo colocando un archivo JSON en
<code>&lt;config&gt;/BridgeLab/plugins/anonymization/</code>.</p>

<h3>Anonimización por lotes (Pro)</h3>
<p><strong>Herramientas → Anonimización por lotes…</strong> enmascara
una carpeta entera en una sola pasada: elige archivos de origen o una
carpeta, elige una carpeta de salida y ejecuta. Cada mensaje pasa por el
mismo pipeline que el diálogo interactivo (catálogo PHI integrado +
reglas de plugins activas) y se escribe como copia en la carpeta de
salida — <strong>los originales nunca se tocan</strong>: la herramienta
se niega a sobrescribir cualquier archivo de origen seleccionado, y las
entradas con el mismo nombre procedentes de carpetas distintas reciben
sufijos numéricos en lugar de pisarse entre sí. Una fila por archivo
informa del número de PHI enmascarados o del error; se aplican los
mismos límites de 5000 archivos / 10&nbsp;MB que en la validación por
lotes.</p>

<h3>Exportación</h3>
<p>Los usuarios Pro pueden exportar el mensaje estructurado como JSON o
CSV mediante <strong>Herramientas → Exportar JSON / CSV</strong>. Útil
para cargar datos HL7 en herramientas de análisis (Power BI, Excel,
pandas).</p>

<div class="warn">La anonimización sustituye los valores <em>en el
editor</em>. Conserva siempre tu archivo de origen original como
registro canónico - la copia anonimizada es para compartir, no para
almacenamiento a largo plazo.</div>
`,
};

export const testCasesSection: ManualSection = {
	id: 'testcases',
	heading: 'Biblioteca de Casos de Prueba',
	body: `
<p>La Biblioteca de Casos de Prueba (<kbd>Ctrl</kbd>+<kbd>L</kbd>)
almacena mensajes reutilizables con nombre, categoría, etiquetas y
descripción. Usa <strong>Guardar Mensaje Actual</strong> para capturar
la pestaña activa, o crea casos desde cero. Los casos persisten en la
base de datos local y pueden buscarse por cualquiera de sus campos.</p>

<p class="note">El nivel Community conserva hasta 10 casos de prueba
guardados — los casos existentes permanecen siempre visibles, editables
y ejecutables; solo los guardados nuevos por encima del límite piden una
actualización.</p>

<h3>Resultados esperados</h3>
<p>Cada caso puede declarar un <strong>tipo de mensaje esperado</strong>
(<code>ADT</code> coincide con cualquier evento ADT,
<code>ADT^A01</code> es exacto) y un <strong>resultado de validación
esperado</strong> (válido / no válido). Eso convierte un fragmento en un
test.</p>

<h3>Ejecutar verificaciones</h3>
<p><strong>Verificar</strong> analiza y valida de verdad un caso
individual — HL7 v2 o FHIR, detectado automáticamente — y compara el
resultado con sus expectativas. <strong>Ejecutar todos</strong> hace lo
mismo con cada caso que coincide con la búsqueda actual, con una
insignia de superado/fallido por fila y un resumen superados/total en la
barra de herramientas. Tras un cambio en una interfaz, un solo clic te
dice cuál de tus mensajes de referencia se rompió. Editar un caso borra
su resultado almacenado hasta la siguiente ejecución.</p>

<h3>Restauración de sesión</h3>
<p>BridgeLab guarda tus pestañas abiertas (incluidas las ediciones sin
guardar) y las reabre en el siguiente arranque, al estilo Notepad++.
Contrólalo en <strong>Configuración → Sesión</strong>: activa o
desactiva <em>Restaurar pestañas abiertas al iniciar</em>, o usa
<em>Borrar sesión guardada</em> para eliminar el conjunto de pestañas
almacenado (esto también desactiva la restauración, de modo que el
siguiente arranque empieza en la pantalla de bienvenida).</p>
`,
};
