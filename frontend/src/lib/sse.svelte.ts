import type { ContainerInfo, SystemMetrics } from './types';
import { createSSE } from './api';
import { fetchConfig } from './config.svelte';

export const liveData = $state({
	systemMetrics: null as SystemMetrics | null,
	dockerContainers: [] as ContainerInfo[],
	sseConnected: false
});

export function connectSSE(): () => void {
	return createSSE(
		(data) => {
			liveData.systemMetrics = data.system;
			liveData.dockerContainers = data.docker;
		},
		() => fetchConfig({ silent: true }),
		(connected) => {
			liveData.sseConnected = connected;
		}
	);
}
