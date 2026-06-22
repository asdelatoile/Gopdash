<script lang="ts">
	import type { WidgetConfig, BookmarkGroup } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import AppIcon from '$lib/components/AppIcon.svelte';
	import { parseIcon } from '$lib/icons';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'bookmarks' }>;
	}

	let { widget }: Props = $props();
	let groups = $state<BookmarkGroup[]>([]);
	const locale = $derived(resolveLocale(appConfig.data));

	onMount(async () => {
		try {
			groups = await api.getBookmarks(widget.group);
		} catch (e) {
			console.error('Failed to load bookmarks:', e);
		}
	});
</script>

<WidgetHeader {widget} title={widget.title ?? widget.group ?? 'Liens'} />
<CardContent>
	{#each groups as group}
		<div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
			{#each group.links as link}
				<a
					href={link.url}
					target="_blank"
					rel="noopener noreferrer"
					class="flex items-center gap-2 p-2 rounded-md bg-secondary/50 hover:bg-secondary transition-colors text-xs"
				>
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
					</div>
				</a>
			{/each}
		</div>
	{:else}
		<p class="text-xs text-muted-foreground">{t('bookmarks_no_links', locale)}</p>
	{/each}
</CardContent>
