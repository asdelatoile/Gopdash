<script lang="ts">
	import type { WidgetConfig } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import { buildSearchUrl } from '$lib/search';
	import SearchIcon from '@lucide/svelte/icons/search';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'search' }>;
	}

	let { widget }: Props = $props();
	let query = $state('');

	const locale = $derived(resolveLocale(appConfig.data));
	const engine = $derived(
		appConfig.data?.search_engines.find((e) => e.id === widget.service_id) ?? null
	);
	const openInNewTab = $derived((widget.target ?? 'new-tab') === 'new-tab');

	const inputClass =
		'flex h-9 w-full rounded-lg border border-input bg-background px-3 text-sm shadow-xs outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50';

	function submitSearch(e: SubmitEvent) {
		e.preventDefault();
		const trimmed = query.trim();
		if (!trimmed || !engine) return;

		const url = buildSearchUrl(engine.url, trimmed);
		if (openInNewTab) {
			window.open(url, '_blank', 'noopener,noreferrer');
		} else {
			window.location.href = url;
		}
	}
</script>

<WidgetHeader {widget} title={widget.title ?? engine?.name ?? 'Recherche'} />
<CardContent>
	{#if !engine}
		<p class="text-xs text-muted-foreground">{t('search_no_engines', locale)}</p>
	{:else}
		<form class="flex gap-2" onsubmit={submitSearch}>
			<input
				type="search"
				bind:value={query}
				placeholder={t('search_placeholder', locale)}
				class={inputClass}
				autocomplete="off"
			/>
			<Button type="submit" size="icon" class="shrink-0" aria-label={t('search_button', locale)}>
				<SearchIcon class="size-4" />
			</Button>
		</form>
	{/if}
</CardContent>
