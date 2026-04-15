/**
 * Global store for in-app dialogs (info, warning, error, confirm).
 * Replaces native alert()/confirm() with styled, translatable modals.
 */

export type DialogKind = 'info' | 'warning' | 'error' | 'success' | 'confirm';

export interface DialogOptions {
	kind?: DialogKind;
	title?: string;
	message: string;
	details?: string;
	okLabel?: string;
	cancelLabel?: string;
	showCancel?: boolean;
}

export interface ActiveDialog extends DialogOptions {
	id: number;
	resolve: (confirmed: boolean) => void;
}

class DialogStore {
	active = $state<ActiveDialog | null>(null);
	private nextId = 1;

	/** Show a dialog. Returns a promise that resolves with true=OK, false=cancel. */
	async show(options: DialogOptions): Promise<boolean> {
		return new Promise((resolve) => {
			this.active = {
				...options,
				kind: options.kind ?? 'info',
				id: this.nextId++,
				resolve,
			};
		});
	}

	/** Shortcut for info dialogs. */
	async info(message: string, title?: string): Promise<void> {
		await this.show({ kind: 'info', message, title });
	}

	/** Shortcut for error dialogs. */
	async error(message: string, title?: string, details?: string): Promise<void> {
		await this.show({ kind: 'error', message, title, details });
	}

	/** Shortcut for warnings. */
	async warning(message: string, title?: string): Promise<void> {
		await this.show({ kind: 'warning', message, title });
	}

	/** Shortcut for confirmations. */
	async confirm(message: string, title?: string): Promise<boolean> {
		return this.show({ kind: 'confirm', message, title, showCancel: true });
	}

	close(confirmed: boolean) {
		if (this.active) {
			const d = this.active;
			this.active = null;
			d.resolve(confirmed);
		}
	}
}

export const dialogStore = new DialogStore();
