const API_BASE = "https://skins.cubiclauncher.org/api";

const DEFAULT_STEVE_HASH =
	"60a5bd016b3c9a1b9272e4929e30827a67be4ebb219017adbbc4a4d22ebd5b1";

interface SkinData {
	skinUrl: string | null;
	model: "classic" | "slim";
}

function buildSvg(textureHash: string): string {
	const url = `https://textures.minecraft.net/texture/${textureHash}`;
	return `<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 8 8">
<defs><style>image{image-rendering:pixelated;image-rendering:crisp-edges}</style></defs>
<image href="${url}" x="-8" y="-8" width="64" height="64"/>
<image href="${url}" x="-40" y="-8" width="64" height="64"/>
</svg>`;
}

function extractHash(url: string): string {
	return url.split("/").pop()?.split("?")[0] ?? "";
}

export async function getHeadSvg(
	username: string,
	endpoint: string,
): Promise<string> {
	try {
		const res = await fetch(`${API_BASE}/${endpoint}/skin/${username}`);
		if (!res.ok) return "";

		const data: SkinData = await res.json();

		if (data.skinUrl) {
			const hash = extractHash(data.skinUrl);
			if (!hash) return "";
			return buildSvg(hash);
		}

		return buildSvg(DEFAULT_STEVE_HASH);
	} catch {
		return "";
	}
}
