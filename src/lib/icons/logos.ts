export const INSTANCE_LOGOS = [
	"vanilla.png",
	"fabric.png",
	"forge.png",
	"neoforged.png",
	"quilt.png",
	"modth.png",
];

const LOGO_MAP: Record<string, string> = {
	vanilla: "vanilla.png",
	fabric: "fabric.png",
	forge: "forge.png",
	neoforge: "neoforged.png",
	quilt: "quilt.png",
};

export function getLoaderLogo(loader: string): string {
	return `/images/instances/${LOGO_MAP[loader.toLowerCase()] ?? "vanilla.png"}`;
}
