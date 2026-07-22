import { convertFileSrc } from "@tauri-apps/api/core";

export const INSTANCE_LOGOS = [
	"vanilla.png",
	"fabric.png",
	"forge.png",
	"neoforged.png",
	"quilt.png",
	"modth.png",
];

export const PRESET_ICON_PREFIX = "/images/instances/";

const LOGO_MAP: Record<string, string> = {
	vanilla: "vanilla.png",
	fabric: "fabric.png",
	forge: "forge.png",
	neoforge: "neoforged.png",
	quilt: "quilt.png",
};

export function getLoaderLogo(loader: string): string {
	return `${PRESET_ICON_PREFIX}${LOGO_MAP[loader.toLowerCase()] ?? "vanilla.png"}`;
}

export function isPresetIcon(iconPath: string | null): boolean {
	return !!iconPath && iconPath.startsWith(PRESET_ICON_PREFIX);
}

export function getDisplayIconSrc(iconPath: string | null): string {
	if (!iconPath) return "/images/cubic.svg";
	if (isPresetIcon(iconPath)) return iconPath;
	return convertFileSrc(iconPath);
}
