import type { Component } from 'svelte';
import CloudIcon from '@lucide/svelte/icons/cloud';
import CloudFogIcon from '@lucide/svelte/icons/cloud-fog';
import CloudRainIcon from '@lucide/svelte/icons/cloud-rain';
import CloudSnowIcon from '@lucide/svelte/icons/cloud-snow';
import CloudSunIcon from '@lucide/svelte/icons/cloud-sun';
import CloudDrizzleIcon from '@lucide/svelte/icons/cloud-drizzle';
import SunIcon from '@lucide/svelte/icons/sun';
import CloudLightningIcon from '@lucide/svelte/icons/cloud-lightning';

const ICONS: Record<string, Component> = {
	clear: SunIcon,
	'partly-cloudy': CloudSunIcon,
	cloudy: CloudIcon,
	fog: CloudFogIcon,
	drizzle: CloudDrizzleIcon,
	rain: CloudRainIcon,
	snow: CloudSnowIcon,
	showers: CloudRainIcon,
	thunderstorm: CloudLightningIcon
};

export function weatherIcon(icon: string): Component {
	return ICONS[icon] ?? CloudIcon;
}

export function windSpeedLabel(units?: string | null): string {
	return units === 'imperial' ? 'mph' : 'm/s';
}
