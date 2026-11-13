import { convertFileSrc } from "@tauri-apps/api/core";

/// Keep in sync with `ALLOWED_HOSTS` in
/// `src-tauri/src/services/market_icon_cache.rs`. Other hosts keep loading
/// straight from the CDN instead of failing against the cache protocol.
const CACHEABLE_HOSTS = new Set([
	"cdn.modrinth.com",
	"media.forgecdn.net",
	"edge.forgecdn.net",
]);

/**
 * Rewrites a remote market icon to the local cache protocol. Local icons
 * (`data:`, `cubic-icon:`) pass through untouched.
 */
export function marketIconSrc(
	icon: string | null | undefined,
	toSrc: (url: string, protocol: string) => string = convertFileSrc,
): string | null {
	if (!icon) return null;
	if (!icon.startsWith("https://")) return icon;
	try {
		if (!CACHEABLE_HOSTS.has(new URL(icon).hostname)) return icon;
	} catch {
		return icon;
	}
	return toSrc(icon, "cubic-market-icon");
}
