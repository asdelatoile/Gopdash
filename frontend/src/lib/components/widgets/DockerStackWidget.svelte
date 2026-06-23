<script lang="ts">
	import type { WidgetConfig, ContainerInfo } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { liveData } from '$lib/sse.svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import {
		aggregateGroupState,
		containerTooltip,
		containersMatchingTargets,
		groupHasRunning,
		runningCount
	} from '$lib/docker-groups';
	import { Play, Square, RotateCw } from 'lucide-svelte';
	import { onMount } from 'svelte';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'docker-stack' }>;
	}

	let { widget }: Props = $props();
	let loading = $state<Record<string, boolean>>({});

	const locale = $derived(resolveLocale(appConfig.data));
	const allTargets = $derived(
		[...new Set(widget.stacks.flatMap((stack) => stack.targets))].join(',')
	);

	onMount(async () => {
		await refreshContainers();
	});

	async function refreshContainers() {
		if (!allTargets) return;
		try {
			const data = await api.getContainers(allTargets, false);
			const ids = new Set(data.map((c) => c.id));
			const rest = liveData.dockerContainers.filter((c) => !ids.has(c.id));
			liveData.dockerContainers = [...rest, ...data];
		} catch (e) {
			console.error('Failed to load containers:', e);
		}
	}

	function stackContainers(stack: { targets: string[] }): ContainerInfo[] {
		return containersMatchingTargets(liveData.dockerContainers, stack.targets);
	}

	async function runContainerAction(id: string, fn: 'start' | 'stop' | 'restart') {
		if (fn === 'start') await api.startContainer(id);
		else if (fn === 'stop') await api.stopContainer(id);
		else await api.restartContainer(id);
	}

	function stackKey(name: string): string {
		return `stack:${name}`;
	}

	function isStackLoading(name: string, containers: ContainerInfo[]): boolean {
		const key = stackKey(name);
		return !!loading[key] || containers.some((c) => loading[c.id]);
	}

	async function stackAction(
		stack: { name: string; targets: string[] },
		fn: 'start' | 'stop' | 'restart'
	) {
		const containers = stackContainers(stack);
		const key = stackKey(stack.name);
		const targets = containers.filter((c) => {
			const running = c.state.toLowerCase() === 'running';
			if (fn === 'start') return !running;
			if (fn === 'stop') return running;
			return true;
		});
		if (targets.length === 0) return;

		loading[key] = true;
		for (const c of targets) loading[c.id] = true;

		try {
			await Promise.all(targets.map((c) => runContainerAction(c.id, fn)));
			await refreshContainers();
		} catch (e) {
			console.error(e);
		} finally {
			loading[key] = false;
			for (const c of targets) loading[c.id] = false;
		}
	}

	function stateColor(state: string): string {
		switch (state.toLowerCase()) {
			case 'running':
				return 'bg-green-500';
			case 'exited':
				return 'bg-red-500';
			case 'partial':
				return 'bg-yellow-500';
			default:
				return 'bg-gray-400';
		}
	}

	function runningLabel(containers: ContainerInfo[]): string {
		return t('docker_stack_running', locale)
			.replace('{running}', String(runningCount(containers)))
			.replace('{total}', String(containers.length));
	}

	function stackHasStopped(containers: ContainerInfo[]): boolean {
		return containers.some((c) => c.state.toLowerCase() !== 'running');
	}
</script>

{#snippet stackActions(stack: { name: string; targets: string[] }, containers: ContainerInfo[])}
	<div class="flex gap-1 shrink-0" role="group" aria-label="Actions {stack.name}">
		{#if stackHasStopped(containers)}
			<Button
				variant="ghost"
				size="icon"
				class="size-7 cursor-pointer"
				disabled={isStackLoading(stack.name, containers)}
				onclick={() => void stackAction(stack, 'start')}
			>
				<Play class="size-3.5" />
			</Button>
		{/if}
		{#if groupHasRunning(containers)}
			<Button
				variant="ghost"
				size="icon"
				class="size-7 cursor-pointer"
				disabled={isStackLoading(stack.name, containers)}
				onclick={() => void stackAction(stack, 'stop')}
			>
				<Square class="size-3.5" />
			</Button>
		{/if}
		<Button
			variant="ghost"
			size="icon"
			class="size-7 cursor-pointer"
			disabled={isStackLoading(stack.name, containers) || containers.length === 0}
			onclick={() => void stackAction(stack, 'restart')}
		>
			<RotateCw class="size-3.5" />
		</Button>
	</div>
{/snippet}

<WidgetHeader {widget} title={widget.title ?? t('docker_stack_title', locale)} />
<CardContent class="space-y-2">
	{#if widget.stacks.length === 0}
		<p class="text-xs text-muted-foreground">{t('docker_stack_no_stacks', locale)}</p>
	{:else}
		{#each widget.stacks as stack (stack.name)}
			{@const containers = stackContainers(stack)}
			<div class="flex flex-col gap-1.5 p-2 rounded-md bg-secondary/50 text-xs">
				<div class="flex items-center justify-between gap-2">
					<div class="flex items-center gap-2 min-w-0">
						<span
							class="w-2 h-2 rounded-full shrink-0 {stateColor(
								containers.length > 0 ? aggregateGroupState(containers) : 'exited'
							)}"
						></span>
						<span class="font-medium truncate">{stack.name}</span>
					</div>
					{@render stackActions(stack, containers)}
				</div>
				<div class="flex items-center justify-between gap-2 pl-4 text-muted-foreground">
					{#if containers.length > 0}
						<span
							class="cursor-default"
							title={containerTooltip(containers)}
						>
							{runningLabel(containers)}
						</span>
					{:else}
						<span>{t('docker_stack_empty', locale)}</span>
					{/if}
				</div>
			</div>
		{/each}
	{/if}
</CardContent>
