export const ICON_REGISTRY: Record<string, string> = {
	"ui:bell": "/icons/ui/bell.svg",
	"ui:interface": "/icons/ui/interface.svg",
	"ui:check": "/icons/ui/check.svg",
	"ui:check-circle": "/icons/ui/check-circle.svg",
	"ui:chevron-down": "/icons/ui/chevron-down.svg",
	"ui:lock": "/icons/ui/lock.svg",
	"ui:unlock": "/icons/ui/unlock.svg",
	"ui:chevron-left": "/icons/ui/chevron-left.svg",
	"ui:chevron-right": "/icons/ui/chevron-right.svg",
	"ui:close": "/icons/ui/close.svg",
	"ui:copy": "/icons/ui/copy.svg",
	"ui:download": "/icons/ui/download.svg",
	"ui:error": "/icons/ui/error.svg",
	"ui:play": "/icons/ui/play.svg",
	"ui:eye": "/icons/ui/eye.svg",
	"ui:refresh": "/icons/ui/refresh.svg",
	"ui:loader": "/icons/ui/loader.svg",
	"ui:search": "/icons/ui/search.svg",
	"ui:trash": "/icons/ui/trash.svg",
	"ui:upload": "/icons/ui/upload.svg",
	"ui:pin": "/icons/ui/pin.svg",
	"ui:flag": "/icons/ui/flag.svg",
	"ui:performance": "/icons/ui/performance.svg",
	"ui:user": "/icons/ui/user.svg",
	"ui:layers": "/icons/ui/layers.svg",
	"ui:status-success": "/icons/ui/status-success.svg",
	"ui:status-error": "/icons/ui/status-error.svg",
	"ui:status-warning": "/icons/ui/status-warning.svg",
	"ui:status-info": "/icons/ui/status-info.svg",

	"nav:create": "/icons/nav/create.svg",
	"nav:edit": "/icons/nav/edit.svg",
	"nav:pencil": "/icons/nav/pencil.svg",
	"nav:settings": "/icons/nav/settings.svg",
	"nav:sliders": "/icons/nav/sliders.svg",

	"instance:box": "/icons/instance/box.svg",
	"instance:check-square": "/icons/instance/check-square.svg",
	"instance:clock": "/icons/instance/clock.svg",
	"instance:code": "/icons/instance/code.svg",
	"instance:database": "/icons/instance/database.svg",
	"instance:external-link": "/icons/instance/external-link.svg",
	"instance:folder": "/icons/instance/folder.svg",
	"instance:grid": "/icons/instance/grid.svg",
	"instance:market": "/icons/instance/grid.svg",
	"instance:multiplayer": "/icons/instance/servers.svg",
	"instance:worlds": "/icons/instance/worlds.svg",
	"instance:image": "/icons/instance/image.svg",
	"instance:puzzle": "/icons/instance/puzzle.svg",
	"instance:resources": "/icons/instance/resources.svg",
	"instance:screenshots": "/icons/instance/screenshots.svg",
	"instance:terminal": "/icons/instance/terminal.svg",
	"instance:servers": "/icons/instance/servers.svg",

	"log:arrow-down": "/icons/log/arrow-down.svg",
	"log:clear": "/icons/log/clear.svg",
	"log:copy": "/icons/log/copy.svg",
	"log:logs": "/icons/log/logs.svg",
	"log:search": "/icons/log/search.svg",
	"log:upload": "/icons/log/upload.svg",
	"log:chevron-up": "/icons/log/chevron-up.svg",
	"log:chevron-down": "/icons/log/chevron-down.svg",

	"brand:curseforge": "/images/instances/curseforge.png",
	"brand:discord": "/icons/brand/discord.svg",
	"brand:fabric": "/images/instances/fabric.png",
	"brand:forge": "/images/instances/forge.png",
	"brand:github": "/icons/brand/github.svg",
	"brand:mosaicary": "/icons/brand/mosaicary.png",
	"brand:modrinth": "/images/instances/modth.png",
	"brand:neoforged": "/images/instances/neoforged.png",
	"brand:quilt": "/images/instances/quilt.png",
	"brand:optifine": "/images/instances/optifine.png",
	"brand:vanilla": "/images/instances/vanilla.png",

	"ui:shaders": "/icons/ui/shaders.svg",
	"ui:resources": "/icons/ui/resources.svg",

	"brand:cubic": "/icons/brand/cubic.svg",
} as const;

export type IconName = keyof typeof ICON_REGISTRY;

export function getIconPath(name: string): string | null {
	return ICON_REGISTRY[name] ?? null;
}

export function isRasterIcon(path: string): boolean {
	const pathname = path.split(/[?#]/, 1)[0];
	const i = pathname.lastIndexOf(".");
	if (i === -1) return false;
	const ext = pathname.slice(i + 1).toLowerCase();
	return ext === "png" || ext === "webp" || ext === "jpg" || ext === "jpeg";
}
