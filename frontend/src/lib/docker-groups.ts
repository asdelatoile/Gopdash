import type { ContainerInfo, DockerGroupBy } from './types';

export interface ContainerGroup {
	key: string;
	name: string;
	isCompose: boolean;
	containers: ContainerInfo[];
}

export function groupContainers(
	containers: ContainerInfo[],
	groupBy: DockerGroupBy = 'flat'
): ContainerGroup[] {
	if (groupBy !== 'compose') {
		return containers.map((c) => ({
			key: c.id,
			name: c.name,
			isCompose: false,
			containers: [c]
		}));
	}

	const composeGroups = new Map<string, ContainerInfo[]>();
	const standalone: ContainerInfo[] = [];

	for (const container of containers) {
		if (container.compose_project) {
			const list = composeGroups.get(container.compose_project) ?? [];
			list.push(container);
			composeGroups.set(container.compose_project, list);
		} else {
			standalone.push(container);
		}
	}

	const result: ContainerGroup[] = [];

	for (const [name, list] of [...composeGroups.entries()].sort(([a], [b]) => a.localeCompare(b))) {
		list.sort((a, b) =>
			(a.compose_service ?? a.name).localeCompare(b.compose_service ?? b.name)
		);
		result.push({ key: name, name, isCompose: true, containers: list });
	}

	for (const container of standalone.sort((a, b) => a.name.localeCompare(b.name))) {
		result.push({
			key: container.id,
			name: container.name,
			isCompose: false,
			containers: [container]
		});
	}

	return result;
}

export function containerDisplayName(container: ContainerInfo): string {
	return container.compose_service ?? container.name;
}

export function aggregateGroupState(containers: ContainerInfo[]): string {
	const running = containers.filter((c) => c.state.toLowerCase() === 'running').length;
	if (running === containers.length) return 'running';
	if (running === 0) return 'exited';
	return 'partial';
}

function runningContainers(containers: ContainerInfo[]): ContainerInfo[] {
	return containers.filter((c) => c.state.toLowerCase() === 'running');
}

export function aggregateCpu(containers: ContainerInfo[]): number {
	return runningContainers(containers).reduce((sum, c) => sum + c.cpu_percent, 0);
}

export function aggregateMemory(containers: ContainerInfo[]): number {
	return runningContainers(containers).reduce((sum, c) => sum + c.memory_usage, 0);
}

export function groupHasRunning(containers: ContainerInfo[]): boolean {
	return runningContainers(containers).length > 0;
}

export function containersMatchingTargets(
	containers: ContainerInfo[],
	targets: string[]
): ContainerInfo[] {
	if (targets.length === 0) return [];

	const seen = new Set<string>();
	const result: ContainerInfo[] = [];

	for (const container of containers) {
		const matches = targets.some(
			(target) =>
				container.name.includes(target) ||
				container.compose_project === target ||
				(container.compose_project?.includes(target) ?? false)
		);
		if (matches && !seen.has(container.id)) {
			seen.add(container.id);
			result.push(container);
		}
	}

	return result.sort((a, b) =>
		(a.compose_service ?? a.name).localeCompare(b.compose_service ?? b.name)
	);
}

export function runningCount(containers: ContainerInfo[]): number {
	return runningContainers(containers).length;
}

export function containerTooltip(containers: ContainerInfo[]): string {
	return containers
		.map((c) => `${containerDisplayName(c)} (${c.state})`)
		.join('\n');
}
