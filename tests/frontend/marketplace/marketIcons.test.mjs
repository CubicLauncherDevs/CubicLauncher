import { expect, test } from "bun:test";
import { marketIconSrc } from "../../../src/lib/utils/marketIcons.ts";

const toSrc = (url, protocol) => `${protocol}:${url}`;

test("remote icons from known CDNs go through the cache protocol", () => {
	for (const host of ["cdn.modrinth.com", "media.forgecdn.net", "edge.forgecdn.net"]) {
		const icon = `https://${host}/avatars/1/icon.png`;
		expect(marketIconSrc(icon, toSrc)).toBe(`cubic-market-icon:${icon}`);
	}
});

test("other hosts keep loading straight from the CDN", () => {
	const icon = "https://example.com/icon.png";
	expect(marketIconSrc(icon, toSrc)).toBe(icon);
	expect(marketIconSrc("http://cdn.modrinth.com/a.png", toSrc)).toBe(
		"http://cdn.modrinth.com/a.png",
	);
});

test("local icons and empty values pass through untouched", () => {
	const dataUrl = "data:image/png;base64,AAAA";
	expect(marketIconSrc(dataUrl, toSrc)).toBe(dataUrl);
	const local = "cubic-icon://localhost/instance/mods?file=a.jar";
	expect(marketIconSrc(local, toSrc)).toBe(local);
	expect(marketIconSrc(null, toSrc)).toBeNull();
	expect(marketIconSrc(undefined, toSrc)).toBeNull();
	expect(marketIconSrc("", toSrc)).toBeNull();
});
