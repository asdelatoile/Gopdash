<script lang="ts">
	import type { WidgetConfig, WeatherData } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { weatherIcon, windSpeedLabel } from '$lib/weather-icons';
	import CloudSunIcon from '@lucide/svelte/icons/cloud-sun';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { formatForecastDate, resolveLocale, t } from '$lib/locale';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'weather' }>;
	}

	let { widget }: Props = $props();
	let weather = $state<WeatherData | null>(null);
	let error = $state<string | null>(null);

	const CurrentIcon = $derived(weather ? weatherIcon(weather.icon) : CloudSunIcon);
	const windUnit = $derived(windSpeedLabel(widget.units));
	const showForecast = $derived(
		widget.show_forecast ?? appConfig.data?.weather_show_forecast ?? true
	);
	const locale = $derived(resolveLocale(appConfig.data));

	onMount(async () => {
		try {
			weather = await api.getWeather(widget.location);
		} catch (e) {
			error = e instanceof Error ? e.message : t('weather_error', locale);
		}
	});
</script>

<WidgetHeader {widget} title={widget.title ?? 'Météo'} />
<CardContent>
	{#if error}
		<p class="text-xs text-destructive">{error}</p>
	{:else if weather}
		<div class="space-y-3">
			<div class="flex items-center gap-3">
				<CurrentIcon class="size-12 shrink-0 text-primary" />
				<div>
					<div class="text-2xl font-bold">{Math.round(weather.temp)}°</div>
					<div class="text-xs text-muted-foreground capitalize">{weather.description}</div>
					<div class="text-xs text-muted-foreground">{weather.location}</div>
				</div>
			</div>
			<div class="flex gap-4 text-xs text-muted-foreground">
				<span>{t('feels_like', locale)} {Math.round(weather.feels_like)}°</span>
				<span>{t('humidity', locale)} {weather.humidity}%</span>
				<span>{t('wind', locale)} {weather.wind_speed.toFixed(1)} {windUnit}</span>
			</div>
			{#if showForecast && weather.forecast.length > 0}
				<div class="grid grid-cols-5 gap-1 pt-2 border-t">
					{#each weather.forecast as day}
						{@const DayIcon = weatherIcon(day.icon)}
						<div class="text-center text-[10px]">
							<div class="text-muted-foreground">{formatForecastDate(day.date, locale)}</div>
							<DayIcon class="size-8 mx-auto text-muted-foreground mt-1" />
							<div>{Math.round(day.temp_max)}°</div>
							<div class="text-muted-foreground">{Math.round(day.temp_min)}°</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<p class="text-xs text-muted-foreground">{t('loading', locale)}</p>
	{/if}
</CardContent>
