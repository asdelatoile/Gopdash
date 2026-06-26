<script lang="ts">
	import type { WidgetConfig, HomeAssistantWidgetState, HaSwitchState } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import { onDestroy } from 'svelte';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'home-assistant' }>;
	}

	let { widget }: Props = $props();
	let widgetState = $state<HomeAssistantWidgetState | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let toggling = $state<Record<string, boolean>>({});

	const locale = $derived(resolveLocale(appConfig.data));
	const refreshMs = $derived(
		Math.max(1, widget.refresh_seconds ?? appConfig.data?.refresh_interval ?? 30) * 1000
	);
	const configured = $derived(appConfig.data?.homeassistant_configured ?? false);
	const hasEntities = $derived(
		(widget.switchs?.length ?? 0) > 0 || (widget.sensors?.length ?? 0) > 0
	);
	const sensorColumns = $derived(Math.min(6, Math.max(1, widget.sensor_columns ?? 2)));
	const switchsColumns = $derived(Math.min(6, Math.max(1, widget.switchs_columns ?? 1)));

	let refreshTimer: ReturnType<typeof setInterval> | null = null;

	function clearRefreshTimer() {
		if (refreshTimer) {
			clearInterval(refreshTimer);
			refreshTimer = null;
		}
	}

	function updateSwitch(entityId: string, updated: HaSwitchState) {
		if (!widgetState) return;
		widgetState = {
			...widgetState,
			switchs: widgetState.switchs.map((sw) =>
				sw.entity_id === entityId ? updated : sw
			)
		};
	}

	async function loadState(force = false) {
		if (!configured || !hasEntities) {
			loading = false;
			return;
		}

		try {
			widgetState = await api.getHomeAssistantState(widget.id, force);
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : t('homeassistant_error', locale);
		} finally {
			loading = false;
		}
	}

	async function toggleSwitch(entityId: string, on: boolean) {
		if (!widgetState) return;

		const previous = widgetState.switchs.find((sw) => sw.entity_id === entityId);
		if (previous) {
			updateSwitch(entityId, { ...previous, on });
		}

		toggling = { ...toggling, [entityId]: true };
		try {
			const updated = await api.setHomeAssistantSwitch(widget.id, entityId, on);
			updateSwitch(entityId, updated);
			error = null;
		} catch (e) {
			if (previous) {
				updateSwitch(entityId, previous);
			}
			error = e instanceof Error ? e.message : t('homeassistant_error', locale);
		} finally {
			toggling = { ...toggling, [entityId]: false };
		}
	}

	function formatSensorValue(value: string, unit?: string | null): string {
		const num = Number(value);
		if (!Number.isNaN(num) && value.trim() !== '') {
			const formatted = Number.isInteger(num) ? String(num) : num.toFixed(1);
			return unit ? `${formatted} ${unit}` : formatted;
		}
		return unit ? `${value} ${unit}` : value;
	}

	$effect(() => {
		const ms = refreshMs;
		if (!configured || !hasEntities) {
			loading = false;
			return;
		}

		void loadState();
		refreshTimer = setInterval(() => void loadState(), ms);

		return () => clearRefreshTimer();
	});

	onDestroy(() => {
		clearRefreshTimer();
	});
</script>

{#snippet body()}
	<WidgetHeader {widget} title={widget.title ?? 'Home Assistant'} />
	<CardContent>
		{#if !configured}
			<p class="text-xs text-muted-foreground">{t('homeassistant_not_configured', locale)}</p>
		{:else if !hasEntities}
			<p class="text-xs text-muted-foreground">{t('homeassistant_no_entities', locale)}</p>
		{:else if loading && !widgetState}
			<p class="text-xs text-muted-foreground">{t('homeassistant_loading', locale)}</p>
		{:else if error && !widgetState}
			<p class="text-xs text-destructive">{error}</p>
		{:else if widgetState}
			<div class="space-y-3">
				{#if widgetState.sensors.length > 0}
					<div
						class="grid gap-2"
						style="grid-template-columns: repeat({sensorColumns}, minmax(0, 1fr));"
					>
						{#each widgetState.sensors as sensor (sensor.entity_id)}
							<div class="rounded-md bg-muted/50 px-2 py-1.5">
								<div class="truncate text-[10px] text-muted-foreground" title={sensor.label}>
									{sensor.label}
								</div>
								<div class="text-sm font-medium tabular-nums">
									{formatSensorValue(sensor.value, sensor.unit)}
								</div>
							</div>
						{/each}
					</div>
				{/if}

				{#if widgetState.switchs.length > 0}
					<div
						class="grid gap-2"
						style="grid-template-columns: repeat({switchsColumns}, minmax(0, 1fr));"
					>
						{#each widgetState.switchs as sw (sw.entity_id)}
							<div class="flex items-center justify-between gap-2 rounded-md bg-muted/50 px-2 py-1.5">
								<span class="min-w-0 truncate text-xs" title={sw.label}>{sw.label}</span>
								<button
									type="button"
									role="switch"
									aria-checked={sw.on}
									aria-label={sw.label}
									disabled={!sw.available || toggling[sw.entity_id]}
									onclick={(e) => {
										e.stopPropagation();
										void toggleSwitch(sw.entity_id, !sw.on);
									}}
									class="relative h-5 w-9 shrink-0 rounded-full transition-colors disabled:opacity-50 {sw.on
										? 'bg-primary'
										: 'bg-muted-foreground/30'}"
								>
									<span
										class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-background shadow transition-transform {sw.on
											? 'translate-x-4'
											: 'translate-x-0'}"
									></span>
								</button>
							</div>
						{/each}
					</div>
				{/if}

				{#if error}
					<p class="text-xs text-destructive">{error}</p>
				{/if}
			</div>
		{/if}
	</CardContent>
{/snippet}

{@render body()}
