// SOAP client IPC bindings (Enterprise feature).
//
// Licensed under the Business Source License 1.1 — see ../LICENSE.
// Community-only builds keep the `soap_send` command registered but it
// answers with a "not included in this build" error; official builds
// gate it behind the Enterprise feature check server-side.

import { invoke } from '@tauri-apps/api/core';

export interface WsSecurity {
	username: string;
	password: string;
}

export interface SoapRequest {
	endpoint: string;
	/** "1.1" or "1.2" */
	soap_version: string;
	/** SOAPAction (1.1 header / 1.2 content-type action parameter). */
	action: string;
	/**
	 * Inner payload. Content starting with '<' is inserted as-is;
	 * anything else (raw HL7 v2 pipes) is XML-escaped and wrapped in a
	 * <payload> element by the backend.
	 */
	payload: string;
	/** Optional custom envelope; `{payload}` is replaced verbatim. */
	envelope_template?: string | null;
	ws_security?: WsSecurity | null;
	ws_addressing: boolean;
	timeout_secs: number;
}

export interface SoapFault {
	code: string;
	reason: string;
}

export interface SoapResult {
	success: boolean;
	status_code: number;
	fault: SoapFault | null;
	/** Inner XML of soap:Body (without the Body element itself). */
	body: string | null;
	response_time_ms: number;
	error: string | null;
}

export async function soapSend(req: SoapRequest, profileName?: string): Promise<SoapResult> {
	return invoke('soap_send', { req, profileName });
}
