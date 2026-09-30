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
		se señala un segundo MSH (dos mensajes en un mismo texto)
		(STRUCT-004). Un código de más de tres caracteres (<code>PIDX</code>)
		se informa, nunca se lee como <code>PID</code>; un archivo por lotes
		puede empezar con FHS/BHS antes del MSH.</li>
	<li><strong>Cabecera MSH:</strong> MSH-9 (tipo de mensaje), MSH-10
		(ID de control), MSH-12 (versión) son obligatorios. Una versión MSH-12
		que BridgeLab no conoce (una futura v2.8, una errata) se valida con
		el catálogo más cercano, v2.5 si no hay ninguno, y una nota
		informativa (MSH-005) lo indica.</li>
	<li><strong>Campos requeridos:</strong> campos obligatorios por
		segmento según el estándar HL7 (p. ej. PID-3 Patient Identifier
		List).</li>
	<li><strong>Límites de longitud:</strong> avisa cuando un campo
		supera el <code>max_length</code> publicado, contado en
		caracteres, para cada repetición de un campo repetido.</li>
	<li><strong>Tipos de dato</strong> (avisos): números (SI, NM), fechas
		(DT, <code>YYYY[MM[DD]]</code>), marcas de tiempo (TS/DTM,
		<code>YYYY[MM[DD[HH[MM[SS[.S]]]]]][+/-ZZZZ]</code>) y horas (TM) se
		comprueban en su formato y contra el calendario y el reloj, así que
		<code>19801399</code> o <code>2024-01-01</code> se señalan. OBX-5 se
		comprueba como el tipo que declara OBX-2 (NM, DT, TS…). Los demás
		tipos no tienen control de formato.</li>
</ul>

<h3>Filtrado y navegación</h3>
<p>Haz clic en las insignias Error / Advertencia / Info para filtrar.
Haz clic en cualquier fila de problema para seleccionar el segmento o el
campo afectado en el editor y en el árbol. Cada fila indica el segmento por
su número, como el árbol (<code>OBX (4)</code>), para distinguir dos
problemas iguales en dos segmentos. Si editas el mensaje después de una
comprobación, el informe se repite con el siguiente análisis en segundo
plano; con el auto-análisis desactivado, el panel indica que el informe no
está al día hasta que pulses <kbd>F6</kbd>.</p>

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
lotes de arriba. Un archivo con el mismo nombre que ya esté en la
carpeta queda como estaba y aparece como no guardado.</p>

<h3>Validación por CLI</h3>
<p>El complemento <code>bridgelab-cli</code> ejecuta los mismos
validadores — HL7 v2 y FHIR, con el núcleo R4 integrado, los paquetes
instalados y los plugins — sin interfaz, para pipelines de CI y cribado
por lotes. No lee ninguna licencia ni la necesita. Además de
<code>validate</code>, <code>info</code>, <code>anonymize</code>,
<code>to-json</code> y <code>batch</code>, ejecuta los paquetes de casos
de prueba exportados de la biblioteca (<code>test</code>, con informe
JUnit para CI), evalúa FHIRPath (<code>fhirpath</code>), envía un
mensaje por MLLP y termina con error si el ACK no es AA o CA
(<code>send</code>; un archivo con varios mensajes se envía en una trama
por mensaje), y exporta el XSD de los mensajes de la edición
Community (<code>xsd</code>; el catálogo completo está en Pro). Todo
comando que lee un mensaje acepta <code>-</code> para la entrada
estándar. Un binario por plataforma acompaña cada versión. Pon entre
comillas dobles los patrones y los argumentos con <code>^</code>
(<code>"*.hl7"</code>, <code>"ORU^R01"</code>): <code>cmd.exe</code> de
Windows elimina un <code>^</code> sin comillas y conserva las comillas
simples.</p>
<p>El <code>batch</code> de la CLI recorre las subcarpetas, lee los
archivos <code>.hl7</code> salvo que <code>--extension</code> diga otra
cosa y comprueba FHIR además de HL7 v2, así que sus totales pueden
diferir de <strong>Herramientas → Validación por lotes…</strong> en la
misma carpeta (una carpeta, <code>.hl7</code>, <code>.txt</code> y
<code>.dat</code>, solo HL7 v2); <code>--extension hl7,txt,dat</code> lee
los mismos archivos.</p>
<pre><code>bridgelab-cli validate message.hl7 bundle.json
bridgelab-cli validate "*.hl7" --format junit &gt; report.xml
bridgelab-cli batch ./inbox --json
bridgelab-cli test regression.bltests.json --format junit &gt; tests.xml
bridgelab-cli fhirpath "Patient.name.family" patient.json
cat message.hl7 | bridgelab-cli send - --host 10.0.0.5 --port 2575</code></pre>
`,
};

export const communicationSection: ManualSection = {
	id: 'communication',
	heading: 'Comunicación (MLLP / HTTP / SOAP)',
	body: `
<p>Abre el panel de comunicación inferior con <kbd>Ctrl</kbd>+<kbd>K</kbd>
o <strong>Herramientas → Panel de comunicación</strong>. Cuatro
pestañas: MLLP, HTTP, SOAP e Historial.</p>

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
construye un acuse de recibo para el mensaje que está en el editor:
elige el código (AA aceptar, AE error, AR rechazar) y haz clic en
<strong>Generar ACK</strong>. El ACK refleja el mensaje, como debe
hacerlo un receptor: el mismo separador de campo y caracteres de
codificación, emisor y receptor intercambiados (MSH-3/4 y MSH-5/6),
<code>ACK^&lt;trigger&gt;^ACK</code> en MSH-9, el mismo processing ID
(MSH-11), versión (MSH-12) y juego de caracteres (MSH-18), y el Message
Control ID (MSH-10, leído con el separador declarado en MSH-1) en
MSA-2. Cada ACK tiene su propio MSH-10. El auto-ACK del listener se
construye igual. El ACK se abre en una pestaña nueva — listo para
devolverlo o para guardarlo como fixture. Si el mensaje actual no tiene
MSH-10, el generador se niega en lugar de producir un ACK imposible de
correlacionar.</p>

<h3>Listener MLLP (Pro)</h3>
<p>Haz clic en <strong>Iniciar escucha</strong> para ejecutar un
servidor en el puerto seleccionado. Los mensajes entrantes se abren en
una pestaña nueva (desactivable, ver más abajo) y se devuelve un ACK
automático con el código configurado (AA/AE/AR). Úsalo para validar
rápidamente lo que está emitiendo tu sistema origen.</p>
<p>Por defecto el listener escucha en <code>127.0.0.1</code>, así que
solo lo alcanzan los programas de este equipo. Para recibir un flujo de
otra máquina, pon <strong>Escuchar en</strong> en <code>0.0.0.0</code> (todas
las interfaces) o en la dirección de una tarjeta de red, y abre el puerto
en el firewall solo para los sistemas previstos.</p>
<p><strong>Detener</strong> cierra también las conexiones que siguen
abiertas: después, ya no se recibe ni se confirma nada.</p>

<h3>Consola del listener</h3>
<p>Mientras el listener está en marcha, cada mensaje recibido aparece
como una fila en la consola: hora local, dirección del par, tamaño de la
carga, el código ACK que realmente se devolvió (<code>AA</code> en
verde, <code>AE</code>/<code>AR</code> en rojo, — cuando el auto-ACK
está desactivado), la codificación de caracteres usada y la primera
línea del mensaje. <strong>Haz clic en una fila para reabrir ese mensaje
en una pestaña.</strong> Los errores del listener aparecen en línea como
filas rojas.</p>
<p>Los chips de la cabecera de la consola filtran las filas por
resultado — <em>AA</em>, <em>AE</em>, <em>AR</em>, <em>Sin ACK</em>
(recibidos con el auto-ACK desactivado), <em>Errores</em> — y cada uno
lleva un contador actualizado, de modo que "AE&nbsp;12" de 300 salta a
la vista antes de desplazarse. Los mismos chips están en la pestaña
Historial.</p>
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
<p>Tanto el envío como el listener tienen un selector
<strong>Codificación</strong> con los juegos de caracteres de las
instalaciones reales: <code>UTF-8</code>, <code>ISO-8859-1</code>
(Latin-1), <code>ISO-8859-2</code>, <code>ISO-8859-15</code>,
<code>windows-1252</code>, <code>windows-1250</code>,
<code>windows-1251</code> y <code>ASCII</code>. El valor por defecto es
<strong>Automática</strong>. En el listener decodifica cada mensaje con
el juego que declara su MSH-18, si no como UTF-8 con alternativa
automática Latin-1, que acepta la mayor parte del tráfico heredado
incluso con MSH-18 vacío; la consola muestra el juego elegido, y el ACK
se recodifica con él para que el par nunca vea caracteres corruptos. En
el envío usa el juego que declara MSH-18, si no aquel con el que se leyó
el archivo de la pestaña, si no UTF-8, y el resultado indica el juego
enviado. Un mensaje con caracteres que el juego elegido no puede
representar no se envía: el error los nombra, para que un nombre nunca
llegue con un <code>?</code>. Las codificaciones de envío y recepción
son independientes.</p>
<p>Los segmentos viajan terminados en CR, sean cuales sean los finales de
línea del editor: LF y CR LF se convierten, como hace la CLI, y lo mismo
vale para un cuerpo HL7 v2 enviado por HTTP o SOAP.</p>
<p>Los <strong>archivos</strong> siguen las mismas reglas. Un archivo que
no está en UTF-8 se abre con la codificación que declara MSH-18
(<code>8859/1</code>, <code>8859/2</code>, <code>8859/15</code>…, y
<code>BIG-5</code>, <code>GB 18030-2000</code> y <code>KS X 1001</code>), o
como Windows-1252 si MSH-18 está vacío: los nombres con acentos se leen
bien en lugar de rechazarse. Un archivo cuyo MSH-18 nombra una
codificación que BridgeLab no sabe decodificar (<code>CNS 11643-1992</code>,
los juegos japoneses ISO 2022) se abre con un aviso, porque su texto e
incluso sus campos pueden leerse mal; la validación y la anonimización por
lotes lo rechazan. Guardar vuelve a escribir el archivo en la
codificación en que se leyó, igual que <code>anonymize</code> de la CLI y
la anonimización por lotes; <code>send</code> sin <code>--encoding</code>
transmite en ella (un archivo UTF-16 sale en la codificación que declara
MSH-18, si no en UTF-8). Al guardar, un carácter que la codificación no
puede representar se escribe como <code>?</code>; el <code>send</code> de
la CLI, como la aplicación, rechaza en cambio el mensaje.</p>

<h3>HTTP</h3>
<p>Las peticiones GET están disponibles en la edición Community. POST/PUT/DELETE/PATCH
y cualquier forma de autenticación requieren Pro: una cabecera
Authorization o Proxy-Authorization, una cookie, cualquier cabecera cuyo
nombre sugiera una clave, un token, un secreto, una firma o una sesión
(como <code>X-API-Key</code> u <code>Ocp-Apim-Subscription-Key</code>),
un usuario:contraseña en la URL, o una clave o un token en la query
string (<code>access_token</code>, <code>api_key</code>,
<code>key</code>, <code>token</code>, <code>sig</code>,
<code>client_secret</code>…). Los parámetros de búsqueda FHIR como
<code>code</code> no son credenciales. Las redirecciones se
siguen en todas las ediciones, pero solo en el mismo servidor (o de http a
https en él): una redirección a otro servidor se muestra como el 3xx que es
y nunca se sigue, para que un mensaje no acabe donde no lo has elegido.
Cuando se ha seguido una redirección, el resultado indica la URL que ha
respondido. Las respuestas de más de 50 MB se cortan y lo indican, y una
respuesta se lee con el juego de caracteres que declara su Content-Type.
POST, PUT y PATCH envían el mensaje de la pestaña actual cuando el campo
Cuerpo está vacío; GET y DELETE solo envían un cuerpo si lo escribes.</p>

<h3>Cliente SOAP (Enterprise)</h3>
<p>La pestaña SOAP envía el mensaje actual (o un cuerpo personalizado)
a endpoints SOAP 1.1/1.2 — middlewares de estilo IHE, pasarelas
regionales y servicios web hospitalarios heredados. Indica la URL del
endpoint, la versión SOAP y la <em>SOAPAction</em>; BridgeLab
construye el envelope, lo envía con el content type correcto
(<code>text/xml</code> más la cabecera SOAPAction en 1.1,
<code>application/soap+xml</code> con el parámetro action en 1.2) y
muestra el estado HTTP, el tiempo de ida y vuelta, el XML interno del
Body y el posible SOAP Fault, decodificado para ambas versiones.</p>
<p>Un mensaje HL7 v2 en bruto se escapa en XML automáticamente
(sus CR de fin de segmento pasan a <code>&amp;#13;</code>, para que el
análisis XML no los convierta en LF) y se envuelve en un elemento
<code>&lt;payload&gt;</code>; el contenido que ya es XML se inserta tal
cual. La configuración avanzada añade credenciales <strong>WS-Security
UsernameToken</strong>, cabeceras <strong>WS-Addressing</strong>
(To / Action / MessageID) y una <strong>plantilla de sobre
personalizada</strong> en la que el marcador literal
<code>{payload}</code> se sustituye por el mensaje — útil cuando el
servicio de destino espera un envoltorio concreto. Con una plantilla,
las cabeceras WS-Security y WS-Addressing se colocan en el SOAP Header
de la plantilla (si no tiene, se añade uno antes de su Body); una
plantilla sin SOAP Envelope ni Body no puede llevarlas, y el envío se
rechaza en lugar de hacerse sin ellas. La importación de WSDL está
prevista como paso siguiente.</p>

<h3>Historial</h3>
<p>Cada envío, y cada mensaje que recibe el listener, queda
registrado: destino (host y puerto, o la URL con contraseñas o claves
sustituidas por <code>***</code>), tamaño, código de respuesta y tiempo
de ida y vuelta. Las últimas 100 entradas se conservan entre reinicios y
las más antiguas se eliminan; haz clic en una fila para ver la petición
y la respuesta completas — el mensaje y su ACK, o el cuerpo HTTP y la
respuesta (en SOAP el payload, nunca el sobre con su contraseña). Una
petición o respuesta de más de 256 KB se guarda cortada. Un envío MLLP
registra además el <strong>código ACK</strong> con el que respondió el
receptor (MSA-1), mostrado como insignia verde o roja en la fila: un
envío que llegó al par y recibió un <code>AE</code> es "OK" a nivel de
transporte y un rechazo a nivel de aplicación, y la insignia distingue
ambos casos. Los chips de filtro sobre la lista — <em>AA</em>,
<em>AE</em>, <em>AR</em>, <em>Sin ACK</em>, <em>Fallidos</em> — llevan
contadores y reducen la lista a un solo resultado; los códigos
commit-mode CA, CE y CR cuentan como AA, AE y AR. <em>Fallidos</em> es
una petición sin respuesta: un servidor que respondió con un error
(HTTP 404, 500…) aparece con su código.</p>

<h3>Perfiles de conexión</h3>
<p>Guarda los endpoints de uso frecuente como perfiles con nombre desde
la fila <strong>Perfil</strong>: escribe un nombre y haz clic en
<em>Guardar</em>. Los perfiles MLLP almacenan host, puerto, tiempo de
espera y auto-ACK; los perfiles HTTP almacenan URL, cabeceras y tiempo
de espera; los perfiles SOAP almacenan endpoint, SOAPAction y tiempo de
espera. Al seleccionar un perfil, este se aplica al formulario;
guardar con un nombre existente lo sobrescribe; <em>Eliminar</em> borra
el seleccionado. Los perfiles se guardan en la base de datos local y
sobreviven a los reinicios.</p>
<p class="note">Los perfiles HTTP guardan el campo Cabeceras tal
como se escribe, cabecera <code>Authorization</code> incluida, en la
base de datos local. Para no guardar ahí una contraseña o un token,
introdúcelo en <em>Autenticación</em> en la configuración HTTP
avanzada, que nunca se guarda.</p>
`,
};

export const anonymizationSection: ManualSection = {
	id: 'anonymization',
	heading: 'Anonimización y exportación',
	body: `
<p><strong>Herramientas → Anonimizar</strong> detecta los campos PHI
en los segmentos que identifican al paciente y los enmascara según su
nivel de sensibilidad: 89 campos integrados en PID, PV1 (números de
visita), MRG, NK1, GT1, IN1 e IN2, además de los comentarios NTE y los
resultados OBX de texto libre (TX/FT). Cubren nombres, fechas de
nacimiento y defunción, direcciones, teléfonos, SSN y otros
identificadores del paciente, familiares, garante y asegurado. Un
identificador conserva su autoridad asignadora y su tipo
(<code>HOSP</code>, <code>MR</code>). Los demás segmentos no se
enmascaran: revisa el texto libre en otros sitios (OBR, ORC, segmentos
Z) antes de compartir, o añade esos campos con un plugin.</p>

<table>
	<tr><th>Nivel</th><th>Ejemplo</th><th>Estrategia</th></tr>
	<tr><td><strong>Alto</strong></td><td>Nombre del paciente, fecha de
		nacimiento, dirección, teléfono particular, SSN, MRN, número de
		visita</td>
		<td>El texto pasa a ser <code>REDACTED</code>; lo numérico pasa a
		ceros de la misma longitud (preservando el ancho del campo para
		los parsers posteriores); una fecha pasa a 1900-01-01 con la
		misma precisión.</td></tr>
	<tr><td><strong>Medio</strong></td><td>Apellido de soltera de la madre,
		alias, teléfono del trabajo, contactos de familiares, garante y
		asegurado</td>
		<td>Se conserva el primer carácter, el resto se sustituye por
		<code>***</code>.</td></tr>
	<tr><td><strong>Bajo</strong></td><td>Ningún campo integrado;
		disponible para las reglas de los plugins</td>
		<td>Se conservan los 3 primeros caracteres, seguidos de
		<code>...</code>; un valor de 3 caracteres o menos se conserva
		entero.</td></tr>
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
sufijos numéricos en lugar de pisarse entre sí. Un archivo que ya esté
en la carpeta de salida nunca se sustituye (y un enlace nunca se sigue):
su fila lo indica y el archivo queda como estaba, así que elige una
carpeta vacía. Una fila por archivo
informa del número de PHI enmascarados o del error; se aplican los
mismos límites de 5000 archivos / 10&nbsp;MB que en la validación por
lotes.</p>

<h3>Exportación</h3>
<p>Los usuarios Pro pueden exportar el mensaje estructurado como JSON o
CSV mediante <strong>Herramientas → Exportar JSON / CSV</strong>; un
diálogo de guardado pregunta dónde escribir el archivo. Útil para
cargar datos HL7 en herramientas de análisis (Power BI, Excel,
pandas).</p>

<div class="warn">La anonimización escribe su resultado en una pestaña
nueva; la pestaña original queda como estaba. Conserva siempre tu archivo de origen original como
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
(solo se comparan los componentes indicados: <code>ADT</code> coincide con
cualquier evento ADT, <code>ADT^A01</code> coincide con <code>ADT^A01</code>
y <code>ADT^A01^ADT_A01</code> pero no con <code>ADT^A04</code>) y un <strong>resultado de validación
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

<h3>Compartir casos de prueba</h3>
<p><strong>Exportar…</strong> escribe los casos de prueba visibles
—todos, o solo los que coinciden con la búsqueda— en un paquete
<code>.bltests.json</code> para enviarlo a un compañero o guardarlo en
un repositorio Git. Antes de guardar, BridgeLab busca datos personales
en los mensajes HL7 v2 y lista los casos y campos encontrados; los
recursos FHIR se indican como no revisados campo por campo. Con Pro
puedes marcar <em>Enmascarar los datos personales</em> para anonimizar
los mensajes HL7 v2 solo en el archivo exportado: la biblioteca no
cambia.</p>
<p><strong>Importar…</strong> abre un paquete y muestra, antes de
escribir nada, qué es cada caso: <em>Nuevo</em>, <em>Ya en la
biblioteca</em> (se omite) o <em>Distinto</em> de uno que ya tienes, y
entonces eliges conservar el tuyo, sustituirlo o conservar ambos. Los
casos importados mantienen su identificador, así que importar de nuevo
el mismo paquete solo trae lo que ha cambiado. En Community una
importación no puede llevar la biblioteca por encima de 10 casos de
prueba; si fuera así, no se escribe nada.</p>

<h3>Restauración de sesión</h3>
<p>BridgeLab guarda tus pestañas abiertas (incluidas las ediciones sin
guardar) y las reabre en el siguiente arranque, al estilo Notepad++.
Contrólalo en <strong>Configuración → Rendimiento</strong>, en el grupo <em>Sesión</em>: activa o
desactiva <em>Restaurar pestañas abiertas al iniciar</em>, o usa
<em>Borrar sesión guardada</em> para eliminar el conjunto de pestañas
almacenado (esto también desactiva la restauración, de modo que el
siguiente arranque empieza en la pantalla de bienvenida). Desactivar la
restauración también elimina las pestañas guardadas, que pueden contener
datos de pacientes.</p>

<h3>Archivos modificados por otros programas</h3>
<p>BridgeLab detecta cuando otro programa modifica o elimina un archivo
abierto: al volver a la ventana, o al abrir de nuevo el archivo, ofrece
cargar la nueva versión (y avisa si el archivo ya no está).
<strong>Guardar</strong> pide confirmación antes de sobrescribir un
archivo modificado en el disco desde que lo abrió o lo guardó por
última vez, o antes de volver a crear un archivo eliminado. Al iniciar,
una pestaña restaurada sin cambios sin guardar muestra su archivo tal
como está ahora en el disco. Una pestaña restaurada con cambios sin guardar pide confirmación al guardarla por primera vez, porque el archivo puede haber cambiado mientras BridgeLab estaba cerrado.</p>
`,
};
