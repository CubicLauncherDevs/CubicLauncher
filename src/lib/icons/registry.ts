export const ICON_REGISTRY: Record<string, string> = {
	"ui:check": "/images/icons/ui/check.svg",
	"ui:check-circle": "/images/icons/ui/check-circle.svg",
	"ui:chevron-down": "/images/icons/ui/chevron-down.svg",
	"ui:chevron-left": "/images/icons/ui/chevron-left.svg",
	"ui:chevron-right": "/images/icons/ui/chevron-right.svg",
	"ui:close": "/images/icons/ui/close.svg",
	"ui:copy": "/images/icons/ui/copy.svg",
	"ui:download": "/images/icons/ui/download.svg",
	"ui:error": "/images/icons/ui/error.svg",
	"ui:logout": "/images/icons/ui/logout.svg",
	"ui:play": "/images/icons/ui/play.svg",
	"ui:refresh": "/images/icons/ui/refresh.svg",
	"ui:search": "/images/icons/ui/search.svg",
	"ui:spinner": "/images/icons/ui/spinner.svg",
	"ui:success": "/images/icons/ui/success.svg",
	"ui:trash": "/images/icons/ui/trash.svg",
	"ui:upload": "/images/icons/ui/upload.svg",

	"nav:create": "/images/icons/nav/create.svg",
	"nav:edit": "/images/icons/nav/edit.svg",
	"nav:pencil": "/images/icons/nav/pencil.svg",
	"nav:settings": "/images/icons/nav/settings.svg",
	"nav:sliders": "/images/icons/nav/sliders.svg",

	"instance:box": "/images/icons/instance/box.svg",
	"instance:check-square": "/images/icons/instance/check-square.svg",
	"instance:clock": "/images/icons/instance/clock.svg",
	"instance:code": "/images/icons/instance/code.svg",
	"instance:database": "/images/icons/instance/database.svg",
	"instance:external-link": "/images/icons/instance/external-link.svg",
	"instance:folder": "/images/icons/instance/folder.svg",
	"instance:grid": "/images/icons/instance/grid.svg",
	"instance:image": "/images/icons/instance/image.svg",
	"instance:resources": "/images/icons/instance/resources.svg",
	"instance:screenshots": "/images/icons/instance/screenshots.svg",
	"instance:shader": "/images/icons/instance/shader.svg",
	"instance:terminal": "/images/icons/instance/terminal.svg",
	"instance:wrench": "/images/icons/instance/wrench.svg",

	"log:arrow-down": "/images/icons/log/arrow-down.svg",
	"log:clear": "/images/icons/log/clear.svg",
	"log:copy": "/images/icons/log/copy.svg",
	"log:logs": "/images/icons/log/logs.svg",
	"log:scroll-down": "/images/icons/log/scroll-down.svg",
	"log:search": "/images/icons/log/search.svg",
	"log:spinner": "/images/icons/log/spinner.svg",
	"log:upload": "/images/icons/log/upload.svg",
	"log:chevron-up": "/images/icons/log/chevron-up.svg",
	"log:chevron-down": "/images/icons/log/chevron-down.svg",

	"brand:discord": "/images/icons/brand/discord.svg",
	"brand:microsoft": "/images/icons/brand/microsoft.svg",

	"ui:shaders": "/images/icons/ui/shaders.svg",
	"ui:resources": "/images/icons/ui/resources.svg",

	"brand:cubic": "/images/cubic.svg",
} as const;

export type IconName = keyof typeof ICON_REGISTRY;

export function getIconPath(name: string): string | null {
	return ICON_REGISTRY[name] ?? null;
}

export function isRasterIcon(path: string): boolean {
	const i = path.lastIndexOf(".");
	if (i === -1) return false;
	const ext = path.slice(i + 1).toLowerCase();
	return ext === "png" || ext === "webp" || ext === "jpg" || ext === "jpeg";
}
