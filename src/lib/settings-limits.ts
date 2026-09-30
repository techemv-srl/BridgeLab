/**
 * Ranges of the numeric settings. A number input's min/max only guide the
 * spinner: typed values (400, -5, an empty box) reach Save as they are, so
 * Settings clamps them on save and the stores clamp them again on load.
 */
export const SETTING_LIMITS = {
	fontSize: { min: 8, max: 32, fallback: 13 },
	tabSize: { min: 1, max: 8, fallback: 4 },
	/** 0 = never fold. */
	foldThreshold: { min: 0, max: 100000, fallback: 100 },
	autoParseDelay: { min: 100, max: 5000, fallback: 500 },
} as const;

export type SettingName = keyof typeof SETTING_LIMITS;

/** `value` as an integer within the setting's range; the default when it
 *  is not a number at all. */
export function clampSetting(name: SettingName, value: unknown): number {
	const { min, max, fallback } = SETTING_LIMITS[name];
	const n = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
	if (!Number.isFinite(n)) return fallback;
	return Math.min(max, Math.max(min, Math.round(n)));
}
