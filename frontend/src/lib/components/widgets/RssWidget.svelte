<script lang="ts">
	import type { WidgetConfig, RssFeedData } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { formatRssDate, resolveLocale, resolveTimezone, t } from '$lib/locale';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'rss' }>;
	}

	let { widget }: Props = $props();
	let feed = $state<RssFeedData | null>(null);
	let error = $state<string | null>(null);

	const locale = $derived(resolveLocale(appConfig.data));
	const timezone = $derived(resolveTimezone(appConfig.data));

	onMount(async () => {
		if (!widget.service_id) return;
		try {
			feed = await api.getRss(widget.service_id);
		} catch (e) {
			error = e instanceof Error ? e.message : t('rss_error', locale);
		}
	});
</script>

<WidgetHeader {widget} title={widget.title ?? widget.service_id ?? 'RSS'} />
<CardContent>
	{#if error}
		<p class="text-xs text-destructive">{error}</p>
	{:else if feed}
		<ul class="space-y-2">
			{#each feed.items as item}
				<li class="text-xs border-b border-border/50 pb-2 last:border-0">
					<a
						href={item.link}
						target="_blank"
						rel="noopener noreferrer"
						class="font-medium hover:text-primary line-clamp-2"
					>
						{item.title}
					</a>
					{#if item.pub_date}
						<div class="text-muted-foreground text-[10px] mt-0.5">
							{formatRssDate(item.pub_date, locale, timezone)}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="text-xs text-muted-foreground">{t('loading', locale)}</p>
	{/if}
</CardContent>
