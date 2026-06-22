<script lang="ts">
	import type { WidgetConfig } from '$lib/types';
	import { CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import AppIcon from '$lib/components/AppIcon.svelte';
	import { defaultWidgetIcon } from '$lib/icons';

	interface Props {
		widget: WidgetConfig;
		title: string;
	}

	let { widget, title }: Props = $props();

	const icon = $derived(widget.icon ?? defaultWidgetIcon(widget.type));
	const showHeader = $derived(widget.show_header !== false);
</script>

{#if showHeader}
<CardHeader class="widget-header">
	<CardTitle class="flex items-center gap-2">
		<AppIcon value={icon} class="size-4" alt={title} />
		{title}
	</CardTitle>
</CardHeader>
{/if}
