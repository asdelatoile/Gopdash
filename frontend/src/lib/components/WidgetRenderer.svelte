<script lang="ts">
	import type { WidgetConfig } from '$lib/types';
	import DockerWidget from './widgets/DockerWidget.svelte';
	import DockerUpdatesWidget from './widgets/DockerUpdatesWidget.svelte';
	import SystemWidget from './widgets/SystemWidget.svelte';
	import WeatherWidget from './widgets/WeatherWidget.svelte';
	import BookmarksWidget from './widgets/BookmarksWidget.svelte';
	import RssWidget from './widgets/RssWidget.svelte';
	import CalendarWidget from './widgets/CalendarWidget.svelte';
	import SearchWidget from './widgets/SearchWidget.svelte';

	let { widget }: { widget: WidgetConfig } = $props();
</script>

{#if widget.type === 'docker'}
	<DockerWidget {widget} />
{:else if widget.type === 'docker-updates'}
	<DockerUpdatesWidget {widget} />
{:else if widget.type === 'system'}
	<SystemWidget {widget} />
{:else if widget.type === 'weather'}
	<WeatherWidget {widget} />
{:else if widget.type === 'bookmarks'}
	<BookmarksWidget {widget} />
{:else if widget.type === 'rss'}
	<RssWidget {widget} />
{:else if widget.type === 'calendar'}
	<CalendarWidget {widget} />
{:else if widget.type === 'search'}
	<SearchWidget {widget} />
{/if}
