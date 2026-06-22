<script lang="ts">
	import type { WidgetConfig } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { liveData } from '$lib/sse.svelte';
	import { formatBytes, formatUptime } from '$lib/utils';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import { Cpu, HardDrive, MemoryStick, Thermometer } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'system' }>;
	}

	let { widget }: Props = $props();

	onMount(async () => {
		if (!liveData.systemMetrics) {
			try {
				liveData.systemMetrics = await api.getSystem();
			} catch (e) {
				console.error('Failed to load system metrics:', e);
			}
		}
	});

	let metrics = $derived(liveData.systemMetrics);
	const locale = $derived(resolveLocale(appConfig.data));
</script>

<WidgetHeader {widget} title={widget.title ?? 'Système'} />
<CardContent>
	{#if metrics}
		<div class="grid grid-cols-2 gap-3 text-xs">
			<div class="space-y-1">
				<div class="flex items-center gap-1.5 text-muted-foreground">
					<Cpu class="w-3 h-3" />
					{t('cpu', locale)} ({metrics.cpu_cores} {t('cores', locale)})
				</div>
				<div class="font-semibold text-lg">{metrics.cpu_usage.toFixed(1)}%</div>
				<div class="h-1.5 bg-secondary rounded-full overflow-hidden">
					<div
						class="h-full bg-primary rounded-full transition-all"
						style="width: {Math.min(metrics.cpu_usage, 100)}%"
					></div>
				</div>
			</div>

			<div class="space-y-1">
				<div class="flex items-center gap-1.5 text-muted-foreground">
					<MemoryStick class="w-3 h-3" />
					{t('ram', locale)}
				</div>
				<div class="font-semibold text-lg">{metrics.memory_percent.toFixed(1)}%</div>
				<div class="text-muted-foreground">
					{formatBytes(metrics.memory_used, locale)} / {formatBytes(metrics.memory_total, locale)}
				</div>
				<div class="h-1.5 bg-secondary rounded-full overflow-hidden">
					<div
						class="h-full bg-primary rounded-full transition-all"
						style="width: {Math.min(metrics.memory_percent, 100)}%"
					></div>
				</div>
			</div>

			{#each metrics.disks.slice(0, 2) as disk}
				<div class="space-y-1">
					<div class="flex items-center gap-1.5 text-muted-foreground">
						<HardDrive class="w-3 h-3" />
						<span class="truncate">{disk.mount_point}</span>
					</div>
					<div class="font-semibold">{disk.percent.toFixed(1)}%</div>
					<div class="text-muted-foreground">
						{formatBytes(disk.used, locale)} / {formatBytes(disk.total, locale)}
					</div>
				</div>
			{/each}

			{#if metrics.temperatures.length > 0}
				<div class="space-y-1 col-span-2">
					<div class="flex items-center gap-1.5 text-muted-foreground">
						<Thermometer class="w-3 h-3" />
						{t('temperatures', locale)}
					</div>
					<div class="flex flex-wrap gap-2">
						{#each metrics.temperatures.slice(0, 4) as temp}
							<span class="px-2 py-0.5 rounded bg-secondary">{temp.label}: {temp.celsius.toFixed(0)}°C</span>
						{/each}
					</div>
				</div>
			{/if}

			<div class="col-span-2 text-muted-foreground pt-1 border-t">
				{t('uptime', locale)} {formatUptime(metrics.uptime_secs, locale)}
			</div>
		</div>
	{:else}
		<p class="text-xs text-muted-foreground">{t('loading_metrics', locale)}</p>
	{/if}
</CardContent>
