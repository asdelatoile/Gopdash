<script lang="ts">
	import { gridItemAttrs, type WidgetConfig } from '$lib/types';
	import { appConfig } from '$lib/config.svelte';
	import { layoutState, notifyLayoutSaved } from '$lib/layout.svelte';
	import { onMount, tick } from 'svelte';
	import { GridStack } from 'gridstack';
	import WidgetRenderer from './WidgetRenderer.svelte';
	import { api } from '$lib/api';
	import { resolveLocale, t } from '$lib/locale';

	let gridEl = $state<HTMLDivElement | undefined>(undefined);
	let grid: GridStack | null = null;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;

	const persistLayout = $derived(appConfig.data?.persist_layout ?? false);
	const locale = $derived(resolveLocale(appConfig.data));
	const gridColumns = $derived(appConfig.data?.grid?.columns ?? 24);
	const gridCellHeight = $derived(appConfig.data?.grid?.cell_height ?? 15);

	const widgets = $derived(
		appConfig.data && !appConfig.loading ? resolveWidgets(appConfig.data.widgets) : []
	);

	onMount(() => {
		return () => {
			if (saveTimer) clearTimeout(saveTimer);
			grid?.destroy(false);
			grid = null;
		};
	});

	$effect(() => {
		if (appConfig.loading || widgets.length === 0 || !gridEl) return;
		const columns = gridColumns;
		const cellHeight = gridCellHeight;

		void tick().then(() => {
			if (!gridEl || widgets.length === 0) return;

			if (grid) {
				grid.destroy(false);
				grid = null;
			}

			grid = GridStack.init(
				{
					column: columns,
					cellHeight: cellHeight,
					margin: 10,
					animate: true,
					float: true,
					staticGrid: layoutState.locked,
					resizable: { handles: 'se' }
				},
				gridEl
			);

			grid.on('dragstop', scheduleSaveLayout);
			grid.on('resizestop', scheduleSaveLayout);
		});
	});

	$effect(() => {
		const locked = layoutState.locked;
		if (!grid) return;
		grid.setStatic(locked);
	});

	function resolveWidgets(serverWidgets: WidgetConfig[]): WidgetConfig[] {
		if (persistLayout) return serverWidgets;
		return applyLocalLayout(serverWidgets);
	}

	function applyLocalLayout(serverWidgets: WidgetConfig[]): WidgetConfig[] {
		try {
			const saved = localStorage.getItem('gopdash-layout');
			if (!saved) return serverWidgets;

			const layout = JSON.parse(saved) as Record<
				string,
				{ x: number; y: number; w: number; h: number }
			>;

			return serverWidgets.map((w) => {
				const pos = layout[w.id];
				if (!pos) return w;
				return {
					...w,
					x: pos.x ?? w.x,
					y: pos.y ?? w.y,
					w: Math.min(Math.max(pos.w ?? w.w, 1), 12),
					h: Math.max(pos.h ?? w.h, 1)
				};
			});
		} catch {
			localStorage.removeItem('gopdash-layout');
			return serverWidgets;
		}
	}

	function scheduleSaveLayout() {
		if (layoutState.locked) return;
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(saveLayout, 500);
	}

	function saveLayout() {
		if (!grid) return;

		const items = grid.getGridItems();
		const layout: Record<string, { x: number; y: number; w: number; h: number }> = {};

		items.forEach((el) => {
			const node = el.gridstackNode;
			const id = el.getAttribute('gs-id');
			if (id && node) {
				layout[id] = { x: node.x ?? 0, y: node.y ?? 0, w: node.w ?? 4, h: node.h ?? 3 };
			}
		});

		const payload = Object.entries(layout).map(([id, pos]) => ({ id, ...pos }));

		if (!persistLayout) {
			localStorage.setItem('gopdash-layout', JSON.stringify(layout));
		}

		api
			.saveLayout(payload)
			.then((res) => {
				if (res.persisted) {
					localStorage.removeItem('gopdash-layout');
					notifyLayoutSaved();
				}
			})
			.catch(console.error);
	}
</script>

{#if appConfig.loading}
	<div class="flex items-center justify-center h-64 text-muted-foreground">{t('dashboard_loading', locale)}</div>
{:else if appConfig.error}
	<div class="flex items-center justify-center h-64 text-destructive">{appConfig.error}</div>
{:else if widgets.length === 0}
	<div class="flex items-center justify-center h-64 text-muted-foreground">{t('dashboard_no_widgets', locale)}</div>
{:else}
	<div class="grid-stack" class:layout-locked={layoutState.locked} bind:this={gridEl}>
		{#each widgets as widget (widget.id)}
			<div class="grid-stack-item" {...gridItemAttrs(widget)}>
				<div class="grid-stack-item-content">
					<WidgetRenderer {widget} />
				</div>
			</div>
		{/each}
	</div>
{/if}
