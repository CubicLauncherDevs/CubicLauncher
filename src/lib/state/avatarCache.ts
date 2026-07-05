const cache = new Map<string, string>();
const MAX = 20;

export function getAvatar(url: string): string | undefined {
	return cache.get(url);
}

export function setAvatar(url: string, svg: string) {
	if (cache.size >= MAX) {
		const first = cache.keys().next();
		if (!first.done) cache.delete(first.value);
	}
	cache.set(url, svg);
}
