/**
 * Where the app sends someone who wants to buy a license: the pricing
 * section of the landing page, which runs the FastSpring checkout.
 *
 * `utm_source=app` plus a `utm_medium` naming the entry point reach the
 * order as its tag (the landing keeps utm_source for the tab's lifetime
 * and passes it to the checkout and to the quote e-mail), so sales can
 * tell the trial banner from an upgrade prompt.
 */

const LANDING = 'https://techemv-srl.github.io/BridgeLab/';

export type PricingEntry = 'menu' | 'activation' | 'trial_banner' | 'upgrade_prompt';

export function pricingUrl(entry: PricingEntry): string {
	return `${LANDING}?utm_source=app&utm_medium=${entry}#pricing`;
}

/** Open the pricing page in the OS browser (window.open is a no-op in the webview). */
export async function openPricing(entry: PricingEntry): Promise<void> {
	const url = pricingUrl(entry);
	try {
		const { openUrl } = await import('@tauri-apps/plugin-opener');
		await openUrl(url);
	} catch {
		window.open(url, '_blank'); // web mode
	}
}
