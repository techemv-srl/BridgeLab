import en from './en.json';
import it from './it.json';

export type Locale = 'en' | 'it';

const translations: Record<Locale, Record<string, string>> = { en, it };

// Reactive locale using a simple pub/sub pattern
let _locale: Locale = 'en';
let _version = 0;
const subscribers: Set<() => void> = new Set();

/** Set the active locale and notify all subscribers. */
export function setLocale(locale: Locale) {
	_locale = locale;
	_version++;
	subscribers.forEach((fn) => fn());
}

/** Get the active locale. */
export function getLocale(): Locale {
	return _locale;
}

/** Get available locales. */
export function getLocales(): { code: Locale; name: string }[] {
	return [
		{ code: 'en', name: 'English' },
		{ code: 'it', name: 'Italiano' },
	];
}

/** Subscribe to locale changes. Returns unsubscribe function. */
export function subscribeLocale(fn: () => void): () => void {
	subscribers.add(fn);
	return () => subscribers.delete(fn);
}

/** Get current version (for reactivity triggers). */
export function getLocaleVersion(): number {
	return _version;
}

/**
 * Translate a key. Returns the key itself if not found.
 * Supports simple {placeholder} interpolation.
 */
export function t(key: string, params?: Record<string, string | number>): string {
	let text = translations[_locale]?.[key] ?? translations['en']?.[key] ?? key;
	if (params) {
		for (const [k, v] of Object.entries(params)) {
			text = text.replace(`{${k}}`, String(v));
		}
	}
	return text;
}
