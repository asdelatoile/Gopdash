/** Construit l'URL de recherche à partir d'un modèle `{query}`. */
export function buildSearchUrl(template: string, query: string): string {
	const encoded = encodeURIComponent(query.trim());
	return template.replaceAll('{query}', encoded);
}
