/**
 * Whether a modal dialog is open in the window. The app's shortcuts stand
 * down while one is: behind Settings, Ctrl+N opened the template dialog
 * over it, and Ctrl+W closed the tab the dialog was about.
 *
 * Every modal draws one of these: the dialogs' `.modal-overlay` or
 * `.modal-backdrop`, or the app dialog's `aria-modal` box (modal.test.ts
 * checks that each dialog in the components does).
 */
export const MODAL_SELECTOR = '.modal-overlay, .modal-backdrop, [aria-modal="true"]';

export function isModalOpen(doc: Pick<Document, 'querySelector'> | undefined = typeof document === 'undefined' ? undefined : document): boolean {
	return !!doc?.querySelector(MODAL_SELECTOR);
}
