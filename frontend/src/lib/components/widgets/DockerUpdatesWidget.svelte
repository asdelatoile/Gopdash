<script lang="ts">
	import type { WidgetConfig, ContainerUpdateInfo } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import { ArrowUpCircle, RefreshCw, Trash2 } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import { formatBytes } from '$lib/utils';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'docker-updates' }>;
	}

	let { widget }: Props = $props();
	let updates = $state<ContainerUpdateInfo[]>([]);
	let checkedAt = $state<string | null>(null);
	let loading = $state(true);
	let refreshing = $state(false);
	let pruning = $state(false);
	let pruneMessage = $state<string | null>(null);
	let updating = $state<Record<string, boolean>>({});
	let error = $state<string | null>(null);

	const locale = $derived(resolveLocale(appConfig.data));

	async function loadUpdates(force = false) {
		if (force) refreshing = true;
		else if (checkedAt === null) loading = true;

		try {
			const filter = widget.containers?.join(',');
			const result = await api.getDockerUpdates(filter, widget.show_all, force);
			updates = result.updates;
			checkedAt = result.checked_at;
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : t('docker_updates_error', locale);
		} finally {
			loading = false;
			refreshing = false;
		}
	}

	onMount(() => {
		void loadUpdates();
	});

	async function pruneImages() {
		if (!confirm(t('docker_updates_prune_confirm', locale))) return;

		pruning = true;
		pruneMessage = null;
		try {
			const result = await api.pruneUnusedImages();
			pruneMessage = t('docker_updates_prune_done', locale)
				.replace('{count}', String(result.images_deleted))
				.replace('{size}', formatBytes(result.space_reclaimed, locale));
		} catch (e) {
			error = e instanceof Error ? e.message : t('docker_updates_error', locale);
		} finally {
			pruning = false;
		}
	}

	async function updateContainer(item: ContainerUpdateInfo) {
		updating[item.id] = true;
		try {
			await api.updateContainer(item.id);
			await loadUpdates(true);
		} catch (e) {
			console.error(e);
		} finally {
			updating[item.id] = false;
		}
	}

	function statusLabel(status: ContainerUpdateInfo['status']): string {
		switch (status) {
			case 'available':
				return t('docker_updates_available', locale);
			case 'up_to_date':
				return t('docker_updates_up_to_date', locale);
			default:
				return t('docker_updates_unknown', locale);
		}
	}

	function statusColor(status: ContainerUpdateInfo['status']): string {
		switch (status) {
			case 'available':
				return 'bg-amber-500';
			case 'up_to_date':
				return 'bg-emerald-500';
			default:
				return 'bg-gray-400';
		}
	}

	function displayName(item: ContainerUpdateInfo): string {
		return item.compose_service ?? item.name;
	}

	function formatCheckedAt(iso: string): string {
		try {
			return new Intl.DateTimeFormat(locale, {
				dateStyle: 'short',
				timeStyle: 'short'
			}).format(new Date(iso));
		} catch {
			return iso;
		}
	}
</script>

<WidgetHeader {widget} title={widget.title ?? t('docker_updates_title', locale)} />
<CardContent class="space-y-2">
	<div class="flex justify-end gap-1 -mt-1 mb-1">
		<Button
			variant="ghost"
			size="icon"
			class="size-7 cursor-pointer"
			disabled={pruning || refreshing}
			onclick={() => void pruneImages()}
			title={t('docker_updates_prune', locale)}
		>
			<Trash2 class="size-3.5 {pruning ? 'animate-pulse' : ''}" />
		</Button>
		<Button
			variant="ghost"
			size="icon"
			class="size-7 cursor-pointer"
			disabled={refreshing || pruning}
			onclick={() => void loadUpdates(true)}
			title={t('docker_updates_refresh', locale)}
		>
			<RefreshCw class="size-3.5 {refreshing ? 'animate-spin' : ''}" />
		</Button>
	</div>
	{#if pruneMessage}
		<p class="text-[10px] text-muted-foreground -mt-1 mb-1 text-right">{pruneMessage}</p>
	{/if}
	{#if error}
		<p class="text-xs text-destructive">{error}</p>
	{:else if loading}
		<p class="text-xs text-muted-foreground">{t('docker_updates_loading', locale)}</p>
	{:else if updates.length === 0}
		<p class="text-xs text-muted-foreground">{t('docker_updates_no_containers', locale)}</p>
	{:else}
		{#each updates as item (item.id)}
			<div class="flex flex-col gap-1.5 p-2 rounded-md bg-secondary/50 text-xs">
				<div class="flex items-center justify-between gap-2">
					<div class="flex items-center gap-2 min-w-0">
						<span class="size-2 rounded-full shrink-0 {statusColor(item.status)}"></span>
						<div class="min-w-0">
							<div class="font-medium truncate">{displayName(item)}</div>
							<div class="text-muted-foreground truncate text-[10px]">{item.image}</div>
						</div>
					</div>
					<Button
						variant="ghost"
						size="icon"
						class="size-7 shrink-0 cursor-pointer"
						disabled={updating[item.id] || item.status !== 'available'}
						title={t('docker_updates_update', locale)}
						onclick={() => void updateContainer(item)}
					>
						<ArrowUpCircle class="size-3.5 {updating[item.id] ? 'animate-pulse' : ''}" />
					</Button>
				</div>
				<div class="flex items-center justify-between gap-2 text-[10px] text-muted-foreground pl-4">
					<span>{statusLabel(item.status)}</span>
					{#if item.error}
						<span class="truncate" title={item.error}>{item.error}</span>
					{/if}
				</div>
			</div>
		{/each}
	{/if}
	{#if checkedAt && !loading}
		<p class="text-[10px] text-muted-foreground text-right pt-1">
			{t('docker_updates_checked_at', locale).replace('{time}', formatCheckedAt(checkedAt))}
		</p>
	{/if}
</CardContent>
