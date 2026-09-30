/**
 * File path helpers for tab labels and "is this file already open".
 * Lexical only: the frontend has no filesystem access, and the file may
 * not exist any more.
 */

/** A Windows path: drive letter, UNC share, or backslashes and no slash. */
function isWindowsPath(path: string): boolean {
	return /^[A-Za-z]:[\\/]/.test(path) || path.startsWith('\\\\') || (path.includes('\\') && !path.includes('/'));
}

/** The file name. A backslash is a separator only in a Windows path: on
 *  Linux and macOS it is an ordinary character of the name. */
export function baseName(path: string): string {
	const parts = isWindowsPath(path) ? path.split(/[\\/]/) : path.split('/');
	return parts.filter((p) => p !== '').pop() ?? path;
}

/** `path` with `.` and `..` segments and repeated separators removed. */
export function normalizePath(path: string): string {
	const win = isWindowsPath(path);
	const sep = win ? '\\' : '/';
	const parts = win ? path.split(/[\\/]/) : path.split('/');
	// Keep the root: '' for '/x' ('/' root), a drive 'C:', or the UNC '\\'.
	let root = '';
	if (win && path.startsWith('\\\\')) {
		root = '\\\\';
		parts.splice(0, 2);
	} else if (win && /^[A-Za-z]:$/.test(parts[0] ?? '')) {
		root = parts.shift()! + sep;
	} else if (!win && path.startsWith('/')) {
		root = '/';
		parts.shift();
	}
	const out: string[] = [];
	for (const p of parts) {
		if (p === '' || p === '.') continue;
		if (p === '..') {
			if (out.length > 0 && out[out.length - 1] !== '..') out.pop();
			else if (!root) out.push('..');
			continue;
		}
		out.push(p);
	}
	return root + out.join(sep);
}

/** True when both paths name the same file (Windows paths ignore case). */
export function samePath(a: string, b: string): boolean {
	const na = normalizePath(a);
	const nb = normalizePath(b);
	return isWindowsPath(na) && isWindowsPath(nb) ? na.toLowerCase() === nb.toLowerCase() : na === nb;
}

/**
 * `path` with `.ext` appended when its file name does not already end with
 * it (or with one of `alsoAccept`), ignoring case. The Linux save dialog
 * does not add the filter's extension to a name typed without one, so
 * "report" was saved as a file with no extension.
 */
export function withExtension(path: string, ext: string, alsoAccept: string[] = []): string {
	const name = baseName(path).toLowerCase();
	const endings = [ext, ...alsoAccept].map((e) => '.' + e.replace(/^\./, '').toLowerCase());
	if (endings.some((e) => name.endsWith(e) && name.length > e.length)) return path;
	return `${path}.${ext.replace(/^\./, '')}`;
}
