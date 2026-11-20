import { t } from '$lib/i18n';

/**
 * The name of a licence feature as the user reads it: the backend's tags
 * (core, hl7v2, fhir_parse...) are identifiers, and the License view and
 * the activation dialog listed them as they are, in every language. A tag
 * a newer backend adds and this list does not know yet is shown as it is.
 */
export function featureName(tag: string): string {
	const key = `feature.${tag}`;
	const name = t(key);
	return name === key ? tag : name;
}
