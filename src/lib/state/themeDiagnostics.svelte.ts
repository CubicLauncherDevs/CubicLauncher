import type { ThemeResponse } from "../api/themeManager";

export type ThemeDiagnosticWarning = {
	key:
		| "themes.diagnostics.fonts"
		| "themes.diagnostics.css"
		| "themes.diagnostics.background";
	params: Record<string, number>;
};

export const themeDiagnostics = $state<{
	themeId: string | null;
	warnings: ThemeDiagnosticWarning[];
}>({ themeId: null, warnings: [] });

// Advisory thresholds only: never change, load, or reject theme resources here.
export function setThemeDiagnostics(
	themeId: string,
	theme: ThemeResponse,
): void {
	const warnings: ThemeDiagnosticWarning[] = [];
	const fontCount = theme.fonts?.length ?? 0;
	if (fontCount > 12) {
		warnings.push({
			key: "themes.diagnostics.fonts",
			params: { count: fontCount },
		});
	}
	const cssBytes = new TextEncoder().encode(
		theme.inject_css ?? "",
	).byteLength;
	if (cssBytes > 256 * 1024) {
		warnings.push({
			key: "themes.diagnostics.css",
			params: { kib: Math.ceil(cssBytes / 1024) },
		});
	}
	themeDiagnostics.themeId = themeId;
	themeDiagnostics.warnings = warnings;
}

// Call with the current image's natural dimensions after the theme commits.
// The caller must also guard its image generation (including same-ID reloads).
export function setThemeBackgroundDimensions(
	themeId: string,
	width: number,
	height: number,
): void {
	if (themeDiagnostics.themeId !== themeId) return;
	const pixels = width * height;
	if (
		!Number.isSafeInteger(width) ||
		!Number.isSafeInteger(height) ||
		width <= 0 ||
		height <= 0 ||
		!Number.isSafeInteger(pixels * 4)
	)
		return;

	const warnings = themeDiagnostics.warnings.filter(
		(warning) => warning.key !== "themes.diagnostics.background",
	);
	if (pixels > 4096 * 4096) {
		warnings.push({
			key: "themes.diagnostics.background",
			params: {
				width,
				height,
				mib: Math.ceil((pixels * 4) / (1024 * 1024)),
			},
		});
	}
	themeDiagnostics.warnings = warnings;
}
