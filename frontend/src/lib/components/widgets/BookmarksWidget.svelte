<script lang="ts">
	import type { WidgetConfig, BookmarkGroup, BookmarkHealthResult } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import AppIcon from '$lib/components/AppIcon.svelte';
	import { parseIcon } from '$lib/icons';
	import { onMount, onDestroy } from 'svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'bookmarks' }>;
	}

	let { widget }: Props = $props();
	let groups = $state<BookmarkGroup[]>([]);
	let healthByName = $state<Record<string, BookmarkHealthResult>>({});

	const locale = $derived(resolveLocale(appConfig.data));
	const refreshMs = $derived((appConfig.data?.refresh_interval ?? 30) * 1000);
	const columns = $derived(Math.min(6, Math.max(1, widget.columns ?? 3)));

	const serviceId = $derived(widget.service_id);

	let refreshTimer: ReturnType<typeof setInterval> | null = null;

	async function loadHealth() {
		if (!serviceId) return;
		try {
			const results = await api.getBookmarkHealth(serviceId);
			healthByName = Object.fromEntries(results.map((r) => [r.name, r]));
		} catch (e) {
			console.error('Failed to load bookmark health:', e);
		}
	}

	async function loadAll() {
		if (!serviceId) return;
		try {
			groups = await api.getBookmarks(serviceId);
		} catch (e) {
			console.error('Failed to load bookmarks:', e);
		}

		const hasHealthChecks = groups.some((g) => g.links.some((l) => l.health_check));
		if (hasHealthChecks) {
			await loadHealth();
		}
	}

	onMount(() => {
		void loadAll();
		refreshTimer = setInterval(() => void loadHealth(), refreshMs);
	});

	onDestroy(() => {
		if (refreshTimer) clearInterval(refreshTimer);
	});
</script>

<WidgetHeader {widget} title={widget.title ?? (serviceId || 'Liens')} />
<CardContent>
	{#each groups as group}
		<div class="grid gap-2" style="grid-template-columns: repeat({columns}, minmax(0, 1fr));">
			{#each group.links as link}
				{@const health = link.health_check ? healthByName[link.name] : undefined}
				<a
					href={link.url}
					target="_blank"
					rel="noopener noreferrer"
					class="flex items-center gap-2 p-2 rounded-md bg-secondary/50 hover:bg-secondary transition-colors text-xs"
					title={health?.error ?? (health ? `${health.latency_ms} ms` : undefined)}
				>
					{#if health}
						<span
							class="size-2 shrink-0 rounded-full {health.status === 'up'
								? 'bg-emerald-500'
								: 'bg-red-500'}"
							title={health.status === 'up' ? t('health_up', locale) : t('health_down', locale)}
						></span>
					{/if}
					{#if link.icon && parseIcon(link.icon)}
						<AppIcon value={link.icon} class="size-5 rounded" alt={link.name} />
					{:else}
						<div class="w-5 h-5 rounded bg-primary/20 flex items-center justify-center text-[10px] font-bold">
							{link.name.charAt(0).toUpperCase()}
						</div>
					{/if}
					<div class="min-w-0">
						<div class="font-medium truncate">{link.name}</div>
						{#if link.description}
							<div class="text-muted-foreground truncate text-[10px]">{link.description}</div>
						{/if}
						{#if health}
							<div class="text-muted-foreground truncate text-[10px] tabular-nums">
								{health.latency_ms} ms
							</div>
						{/if}
					</div>
				</a>
			{/each}
		</div>
	{:else}
		<p class="text-xs text-muted-foreground">{t('bookmarks_no_links', locale)}</p>
	{/each}
</CardContent>
