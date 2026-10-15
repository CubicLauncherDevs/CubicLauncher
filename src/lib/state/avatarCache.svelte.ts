import { SvelteMap } from "svelte/reactivity";
import { getAvatarSvg } from "$lib/api/cubicApi";
import offlineAvatarSvg from "$lib/assets/offline-avatar.svg?raw";

interface CacheEntry {
	// Render SVGs as <img> data URLs, never as inline HTML from the fallback service.
	svg: string;
	fetchedAt: number;
}

const cache = new SvelteMap<string, CacheEntry>();
const MAX = 50;
const TTL_MS = 5 * 60 * 1000; // 5 minutos

interface PendingAvatar {
	controller: AbortController;
	promise: Promise<string>;
}

// Request handles are not UI state. Share both native and fallback requests.
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const pendingFetches = new Map<string, PendingAvatar>();

/**
 * Versiones reactivas de avatar por UUID.
 * Se incrementan tras cambiar la skin para forzar recarga de heads.
 */
export const avatarVersions = new SvelteMap<string, number>();

function cacheKey(uuid: string, version: number): string {
	return `${uuid}@${version}`;
}

function getCacheEntry(key: string): CacheEntry | undefined {
	const entry = cache.get(key);
	if (entry && Date.now() - entry.fetchedAt >= TTL_MS) {
		cache.delete(key);
		return undefined;
	}
	return entry;
}

export function getAvatar(url: string): string | undefined {
	return getCacheEntry(url)?.svg;
}

export function setAvatar(url: string, svg: string): void {
	// Refreshing an existing entry must not evict an unrelated avatar.
	cache.delete(url);
	if (cache.size >= MAX) {
		const first = cache.keys().next();
		if (!first.done) cache.delete(first.value);
	}
	cache.set(url, { svg, fetchedAt: Date.now() });
}

export function invalidateAvatarCache(url: string) {
	cache.delete(url);
}

export function invalidateAvatarFor(uuid: string) {
	const prefix = `${uuid}@`;
	for (const [key, pending] of pendingFetches) {
		if (key.startsWith(prefix)) {
			pending.controller.abort();
			pendingFetches.delete(key);
		}
	}
	for (const key of cache.keys()) {
		if (key.startsWith(prefix)) cache.delete(key);
	}
}

export function getAvatarVersion(uuid: string): number {
	return avatarVersions.get(uuid) ?? 0;
}

export function bumpAvatarVersion(uuid: string): number {
	const next = (avatarVersions.get(uuid) ?? 0) + 1;
	avatarVersions.set(uuid, next);
	// Discard old requests and cached textures before loading the new skin.
	invalidateAvatarFor(uuid);
	return next;
}

export function buildAvatarUrl(
	uuid: string,
	username: string,
	user_type: string,
	serverUrl?: string | null,
): string {
	const endpoint = user_type === "Yggdrasil" ? "elyby" : "mojang";
	const version = getAvatarVersion(uuid);
	const serverParam =
		user_type === "Yggdrasil" && serverUrl
			? `&server=${encodeURIComponent(serverUrl)}`
			: "";
	return `https://skins.cubiclauncher.org/api/${endpoint}/head/${encodeURIComponent(username)}?t=${version}${serverParam}`;
}

export const DEFAULT_AVATAR_SVG = "";

export async function fetchAvatarSvg(
	uuid: string,
	userType: string,
	serverUrl?: string | null,
	username?: string,
): Promise<string> {
	if (userType === "Cracked") {
		return offlineAvatarSvg;
	}

	if (!uuid) {
		return DEFAULT_AVATAR_SVG;
	}

	const version = getAvatarVersion(uuid);
	const key = cacheKey(uuid, version);
	const cached = getCacheEntry(key);
	if (cached) return cached.svg;

	const pending = pendingFetches.get(key);
	if (pending) return pending.promise;

	const controller = new AbortController();
	const fallbackUrl = buildAvatarUrl(
		uuid,
		username ?? "",
		userType,
		serverUrl,
	);
	const promise = loadAvatar(uuid, fallbackUrl, controller.signal)
		.then((svg) => {
			// Native requests cannot be aborted; reject results invalidated in flight.
			if (
				controller.signal.aborted ||
				getAvatarVersion(uuid) !== version
			) {
				return DEFAULT_AVATAR_SVG;
			}
			if (svg) setAvatar(key, svg);
			return svg;
		})
		.finally(() => {
			// An invalidation may already have started a replacement request.
			if (pendingFetches.get(key)?.controller === controller) {
				pendingFetches.delete(key);
			}
		});
	pendingFetches.set(key, { controller, promise });
	return promise;
}

async function loadAvatar(
	uuid: string,
	fallbackUrl: string,
	signal: AbortSignal,
): Promise<string> {
	try {
		return await getAvatarSvg(uuid);
	} catch (error) {
		if (signal.aborted) return DEFAULT_AVATAR_SVG;
		console.error("Error cargando avatar:", error);
	}

	try {
		const response = await fetch(fallbackUrl, { signal });
		return response.ok ? await response.text() : DEFAULT_AVATAR_SVG;
	} catch {
		return DEFAULT_AVATAR_SVG;
	}
}
