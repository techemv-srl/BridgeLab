<script lang="ts">
	import { getLocale } from '$lib/i18n';
	import { generateManualHtml } from './helpContent';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	let helpWindow: Window | null = null;

	$effect(() => {
		const locale = getLocale();
		const html = generateManualHtml(locale);
		const blob = new Blob([html], { type: 'text/html;charset=utf-8' });
		const url = URL.createObjectURL(blob);

		helpWindow = window.open(url, 'bridgelab-help', 'width=780,height=640,menubar=no,toolbar=no,status=no');

		if (helpWindow) {
			const checkClosed = setInterval(() => {
				if (helpWindow?.closed) {
					clearInterval(checkClosed);
					URL.revokeObjectURL(url);
					onClose();
				}
			}, 500);
		} else {
			URL.revokeObjectURL(url);
			onClose();
		}
	});
</script>
