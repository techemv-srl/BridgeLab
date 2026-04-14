import en from './en.json';
import it from './it.json';

export type Locale = 'en' | 'it';

const translations: Record<Locale, Record<string, string>> = { en, it };

let currentLocale: Locale = 'en';

/** Set the active locale. */
export function setLocale(locale: Locale) {
	currentLocale = locale;
}

/** Get the active locale. */
export function getLocale(): Locale {
	return currentLocale;
}

/** Get available locales. */
export function getLocales(): { code: Locale; name: string }[] {
	return [
		{ code: 'en', name: 'English' },
		{ code: 'it', name: 'Italiano' },
	];
}

/**
 * Translate a key. Returns the key itself if not found.
 * Supports simple {placeholder} interpolation.
 */
export function t(key: string, params?: Record<string, string | number>): string {
	let text = translations[currentLocale]?.[key] ?? translations['en']?.[key] ?? key;
	if (params) {
		for (const [k, v] of Object.entries(params)) {
			text = text.replace(`{${k}}`, String(v));
		}
	}
	return text;
}
