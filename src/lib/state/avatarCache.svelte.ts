import { SvelteMap } from "svelte/reactivity";
import { getAvatarSvg } from "$lib/api/cubicApi";

interface CacheEntry {
	// Render SVGs as <img> data URLs, never as inline HTML from the fallback service.
	svg: string;
	fetchedAt: number;
}

const cache = new SvelteMap<string, CacheEntry>();
const MAX = 50;
const TTL_MS = 5 * 60 * 1000; // 5 minutos

// AbortControllers for in-flight fallback fetches, keyed by uuid@version.
// Not reactive: these are short-lived request handles, not UI state.
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const pendingFetches = new Map<string, AbortController>();

/**
 * Versiones reactivas de avatar por UUID.
 * Se incrementan tras cambiar la skin para forzar recarga de heads.
 */
export const avatarVersions = new SvelteMap<string, number>();

function cacheKey(uuid: string, version: number): string {
	return `${uuid}@${version}`;
}

function abortPending(uuid: string, version: number): void {
	const key = cacheKey(uuid, version);
	const controller = pendingFetches.get(key);
	if (controller) {
		controller.abort();
		pendingFetches.delete(key);
	}
}

function pruneExpired(): void {
	const now = Date.now();
	for (const [key, entry] of cache.entries()) {
		if (now - entry.fetchedAt >= TTL_MS) {
			cache.delete(key);
		}
	}
}

export function getAvatar(url: string): string | undefined {
	return cache.get(url)?.svg;
}

export function setAvatar(url: string, svg: string) {
	if (cache.size >= MAX) {
		const first = cache.keys().next();
		if (!first.done) cache.delete(first.value);
	}
	cache.set(url, { svg, fetchedAt: Date.now() });
}

function setAvatarFor(uuid: string, version: number, svg: string) {
	const key = cacheKey(uuid, version);
	if (cache.size >= MAX) {
		const first = cache.keys().next();
		if (!first.done) cache.delete(first.value);
	}
	cache.set(key, { svg, fetchedAt: Date.now() });
}

function getAvatarFor(uuid: string, version: number): CacheEntry | undefined {
	pruneExpired();
	return (
		(cache.get(cacheKey(uuid, version)) as CacheEntry | undefined) ??
		undefined
	);
}

export function invalidateAvatarCache(url: string) {
	cache.delete(url);
}

export function invalidateAvatarFor(uuid: string) {
	const prefix = `${uuid}@`;
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
	// Abort any in-flight fetch for the old version; the component will retry
	// with the new version on its next render cycle.
	abortPending(uuid, next - 1);
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
	if (!uuid || userType === "Cracked") {
		return DEFAULT_AVATAR_SVG;
	}

	const version = getAvatarVersion(uuid);
	const cached = getAvatarFor(uuid, version) as CacheEntry | undefined;
	if (cached) {
		return cached.svg;
	}

	try {
		const svg = await getAvatarSvg(uuid);
		setAvatarFor(uuid, version, svg);
		return svg;
	} catch (err) {
		console.error("Error cargando avatar:", err);
		const fallbackUrl = buildAvatarUrl(
			uuid,
			username ?? "",
			userType,
			serverUrl,
		);
		const key = cacheKey(uuid, version);
		abortPending(uuid, version);
		const controller = new AbortController();
		pendingFetches.set(key, controller);

		try {
			const res = await fetch(fallbackUrl, {
				signal: controller.signal,
			});
			pendingFetches.delete(key);
			if (!res.ok) return DEFAULT_AVATAR_SVG;
			const svg = await res.text();
			setAvatarFor(uuid, version, svg);
			return svg;
		} catch {
			pendingFetches.delete(key);
			return DEFAULT_AVATAR_SVG;
		}
	}
}
