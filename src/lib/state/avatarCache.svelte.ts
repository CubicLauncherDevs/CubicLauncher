import { SvelteMap } from "svelte/reactivity";
import { getAvatarSvg } from "$lib/api/cubicApi";

interface CacheEntry {
	svg: string;
	fetchedAt: number;
}

const cache = new SvelteMap<string, CacheEntry>();
const MAX = 50;
const TTL_MS = 5 * 60 * 1000; // 5 minutos

/**
 * Versiones reactivas de avatar por UUID.
 * Se incrementan tras cambiar la skin para forzar recarga de heads.
 */
export const avatarVersions = new SvelteMap<string, number>();

function cacheKey(uuid: string, version: number): string {
	return `${uuid}@${version}`;
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
	return cache.get(cacheKey(uuid, version));
}

export function invalidateAvatarCache(url: string) {
	cache.delete(url);
}

export function invalidateAvatarFor(uuid: string) {
	for (const key of cache.keys()) {
		if (key.startsWith(`${uuid}@`)) cache.delete(key);
	}
}

export function getAvatarVersion(uuid: string): number {
	return avatarVersions.get(uuid) ?? 0;
}

export function bumpAvatarVersion(uuid: string): number {
	const next = (avatarVersions.get(uuid) ?? 0) + 1;
	avatarVersions.set(uuid, next);
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
	return `https://skins.cubiclauncher.org/api/${endpoint}/head/${username}?t=${version}${serverParam}`;
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
	const cached = getAvatarFor(uuid, version);
	if (cached && Date.now() - cached.fetchedAt < TTL_MS) {
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
		try {
			const res = await fetch(fallbackUrl);
			const svg = await res.text();
			setAvatarFor(uuid, version, svg);
			return svg;
		} catch {
			return cached?.svg ?? DEFAULT_AVATAR_SVG;
		}
	}
}
