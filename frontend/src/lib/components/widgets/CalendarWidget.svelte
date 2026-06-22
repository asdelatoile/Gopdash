<script lang="ts">
	import type { WidgetConfig } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { appConfig } from '$lib/config.svelte';
	import {
		buildMonthGrid,
		firstDayOfWeek,
		formatDateTime,
		formatMonthTitle,
		resolveLocale,
		resolveTimezone,
		shiftMonth,
		t,
		weekdayLabels,
		zonedParts
	} from '$lib/locale';
	import { ChevronLeft, ChevronRight } from 'lucide-svelte';
	import { onMount } from 'svelte';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'calendar' }>;
	}

	let { widget }: Props = $props();
	let now = $state(new Date());
	let viewDate = $state(new Date());

	const locale = $derived(resolveLocale(appConfig.data));
	const timezone = $derived(resolveTimezone(appConfig.data));
	const showToday = $derived(widget.show_today ?? true);
	const showOutsideDays = $derived(widget.show_outside_days ?? false);
	const showNavigation = $derived(widget.show_navigation ?? false);

	const firstDay = $derived(firstDayOfWeek(locale));
	const weekdays = $derived(weekdayLabels(locale, firstDay));
	const todayParts = $derived(zonedParts(now, timezone));
	const month = $derived(buildMonthGrid(viewDate, locale, timezone, todayParts));
	const monthTitle = $derived(formatMonthTitle(viewDate, locale, timezone));
	const clock = $derived(
		formatDateTime(now, locale, timezone, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			hour: '2-digit',
			minute: '2-digit'
		})
	);

	function prevMonth() {
		viewDate = shiftMonth(viewDate, -1, timezone);
	}

	function nextMonth() {
		viewDate = shiftMonth(viewDate, 1, timezone);
	}

	onMount(() => {
		const timer = setInterval(() => {
			now = new Date();
		}, 30_000);
		return () => clearInterval(timer);
	});
</script>

<WidgetHeader {widget} title={widget.title ?? 'Calendrier'} />
<CardContent>
	<div class="space-y-3">
		{#if showToday}
			<div class="text-center">
				<div class="text-3xl font-bold tabular-nums">
					{formatDateTime(now, locale, timezone, { day: 'numeric' })}
				</div>
				<div class="text-xs text-muted-foreground capitalize">{clock}</div>
			</div>
		{/if}

		{#if showNavigation}
			<div class="flex items-center justify-between gap-2">
				<Button
					variant="ghost"
					size="icon"
					class="h-7 w-7 shrink-0"
					onclick={prevMonth}
					aria-label={t('prev_month', locale)}
				>
					<ChevronLeft class="h-4 w-4" />
				</Button>
				<div class="text-sm font-medium capitalize text-center flex-1">{monthTitle}</div>
				<Button
					variant="ghost"
					size="icon"
					class="h-7 w-7 shrink-0"
					onclick={nextMonth}
					aria-label={t('next_month', locale)}
				>
					<ChevronRight class="h-4 w-4" />
				</Button>
			</div>
		{:else}
			<div class="text-sm font-medium capitalize text-center">{monthTitle}</div>
		{/if}

		<div class="grid grid-cols-7 gap-1 text-center text-[10px]">
			{#each weekdays as label}
				<div class="text-muted-foreground font-medium">{label}</div>
			{/each}
			{#each month.cells as cell}
				{@const visible = cell.inMonth || showOutsideDays}
				<div
					class="aspect-square flex items-center justify-center rounded-md text-xs tabular-nums
						{visible ? '' : 'text-transparent'}
						{!cell.inMonth && visible ? 'text-muted-foreground' : ''}
						{cell.isToday ? 'bg-primary text-primary-foreground font-semibold' : ''}"
				>
					{visible ? cell.day : ''}
				</div>
			{/each}
		</div>
	</div>
</CardContent>
