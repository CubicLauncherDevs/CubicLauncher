import { SvelteMap } from "svelte/reactivity";

const cache = new SvelteMap<string, string>();
const MAX = 20;

/**
 * Versiones reactivas de avatar por UUID.
 * Se incrementan tras cambiar la skin para forzar recarga de heads.
 */
export const avatarVersions = new SvelteMap<string, number>();

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

export function invalidateAvatarCache(url: string) {
	cache.delete(url);
}

export function getAvatarVersion(uuid: string): number {
	return avatarVersions.get(uuid) ?? 0;
}

export function bumpAvatarVersion(uuid: string): number {
	const next = (avatarVersions.get(uuid) ?? 0) + 1;
	avatarVersions.set(uuid, next);
	return next;
}

export function buildAvatarUrl(
	uuid: string,
	username: string,
	user_type: string,
): string {
	const endpoint = user_type === "Yggdrasil" ? "elyby" : "mojang";
	const version = getAvatarVersion(uuid);
	return `https://skins.cubiclauncher.org/api/${endpoint}/head/${username}?t=${version}`;
}
