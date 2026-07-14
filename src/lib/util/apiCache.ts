const MAX_ENTRIES = 200;
const DEFAULT_TTL_MS = 5 * 60 * 1000;

interface CacheEntry<T> {
	data: T;
	timestamp: number;
}

export class ApiCache {
	private cache = new Map<string, CacheEntry<unknown>>();

	get<T>(key: string): T | null {
		const entry = this.cache.get(key);
		if (!entry) return null;
		if (Date.now() - entry.timestamp > DEFAULT_TTL_MS) {
			this.cache.delete(key);
			return null;
		}
		return entry.data as T;
	}

	set(key: string, data: unknown): void {
		if (this.cache.size >= MAX_ENTRIES) {
			const oldest = this.cache.entries().next().value;
			if (oldest) this.cache.delete(oldest[0]);
		}
		this.cache.set(key, { data, timestamp: Date.now() });
	}

	clear(): void {
		this.cache.clear();
	}
}

export const apiCache = new ApiCache();
