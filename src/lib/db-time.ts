/**
 * A timestamp from the local database as a Date. SQLite's datetime('now')
 * writes UTC without a zone ("2026-09-30 11:30:08"), which `new Date`
 * reads as local time, hours off; such a value is read as UTC. Anything
 * else (RFC 3339 with its offset) is parsed as it is.
 */
export function parseDbTime(value: string): Date {
	const m = /^(\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}:\d{2}(?:\.\d+)?)$/.exec(value.trim());
	return new Date(m ? `${m[1]}T${m[2]}Z` : value);
}
