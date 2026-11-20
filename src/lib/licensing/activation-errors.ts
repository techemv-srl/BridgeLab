/** The user's text for an online activation that got no answer from the
 *  license server, from the backend's sentinel codes: null for any other
 *  error, which the dialog shows as the backend words it. */
export function activationNetworkError(
	error: string,
	translate: (key: string, params?: Record<string, string | number>) => string,
): string | null {
	// The server was reached and its certificate refused: usually a
	// TLS-inspecting proxy whose root certificate is not installed. Not
	// "no internet access", which sent users after the wrong cause.
	const certificate = /ERR_SERVER_CERTIFICATE(?::\s*(.*))?/s.exec(error);
	if (certificate) return translate('act.serverCertificate', { detail: (certificate[1] ?? '').trim() });
	// Isolated / air-gapped site: no technical noise, one clear message
	// pointing at the offline-key flow.
	if (error.includes('ERR_SERVER_UNREACHABLE')) return translate('act.serverUnreachable');
	return null;
}
