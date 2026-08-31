import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { SvelteMap } from "svelte/reactivity";
import type { ThemeEntry } from "../types/types";

const builtinThemes: ThemeEntry[] = [
	{
		id: "dark",
		name: "Oscuro",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
		preview: { bg: "#0c0c0c", accent: "#ffffff", text: "#d8d8d8" },
	},
	{
		id: "lima",
		name: "Lima",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
		preview: { bg: "#0a0f0a", accent: "#97C459", text: "#c8ddb0" },
	},
	{
		id: "light",
		name: "Claro",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
		preview: { bg: "#f5f5f5", accent: "#2563eb", text: "#1a1a1a" },
	},
	{
		id: "catppuccin-latte",
		name: "Catppuccin Latte",
		author: "Catppuccin / CubicLauncher",
		version: "1.0",
		type: "builtin",
		preview: { bg: "#eff1f5", accent: "#dc8a78", text: "#4c4f69" },
	},
	{
		id: "rose-pine",
		name: "Rosé Pine",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
		preview: { bg: "#191724", accent: "#ebbcba", text: "#e0def4" },
	},
	{
		id: "rose-pine-dawn",
		name: "Rosé Pine Dawn",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
		preview: { bg: "#faf4ed", accent: "#d7827a", text: "#575279" },
	},
];

export interface ThemeFontFace {
	family: string;
	src: string;
	format?: string | null;
	weight?: string | null;
	style?: string | null;
}

export interface ThemeResponse {
	name: string;
	author: string;
	version: string;
	type: string;
	variables: Record<string, string>;
	bg_image?: string | null;
	bg_image_blur?: number | null;
	bg_image_opacity?: number | null;
	fonts: ThemeFontFace[];
	icons: Record<string, string>;
	inject_css?: string | null;
}

const THEME_VARS_ID = "cubic-theme-vars";
const DEFAULT_FONTS_ID = "cubic-default-fonts";
const CUSTOM_CSS_ID = "cubic-theme-css";
const CUSTOM_FONTS_ID = "cubic-theme-fonts";

let currentImage: HTMLImageElement | null = null;
let currentGeneration = 0;
let currentBlobUrl: string | null = null;
const addedFonts: Set<globalThis.FontFace> = new Set();
let appliedThemeId: string | null = null;

export const themeIcons = new SvelteMap<string, string>();

const defaultFontsCSS = `
@font-face {
	font-family: "Cantarell";
	src: url("/fonts/Cantarell-Regular.woff2") format("woff2");
	font-weight: 400;
	font-style: normal;
	font-display: swap;
}
@font-face {
	font-family: "Cantarell";
	src: url("/fonts/Cantarell-Italic.woff2") format("woff2");
	font-weight: 400;
	font-style: italic;
	font-display: swap;
}
@font-face {
	font-family: "Cantarell";
	src: url("/fonts/Cantarell-Bold.woff2") format("woff2");
	font-weight: 700;
	font-style: normal;
	font-display: swap;
}
@font-face {
	font-family: "Cantarell";
	src: url("/fonts/Cantarell-BoldItalic.woff2") format("woff2");
	font-weight: 700;
	font-style: italic;
	font-display: swap;
}
`;

function injectDefaultFonts() {
	let el = document.getElementById(DEFAULT_FONTS_ID);
	if (el) return;
	el = document.createElement("style");
	el.id = DEFAULT_FONTS_ID;
	el.textContent = defaultFontsCSS;
	document.head.appendChild(el);
}

export async function listThemes(): Promise<ThemeEntry[]> {
	let userThemes: ThemeEntry[] = [];
	try {
		userThemes = await invoke<ThemeEntry[]>("list_themes");
	} catch (e) {
		console.error("Error listing user themes:", e);
	}
	const prefixed = userThemes.map((t: ThemeEntry) => ({
		...t,
		id: `user:${t.id}`,
	}));
	return [...builtinThemes, ...prefixed];
}

export async function importThemeZip(zipPath: string): Promise<ThemeEntry> {
	return invoke<ThemeEntry>("import_theme_zip", { zipPath });
}

export async function import_theme_cbth(cbthPath: string): Promise<ThemeEntry> {
	return invoke<ThemeEntry>("import_theme_cbth", { cbthPath });
}

export async function removeTheme(id: string): Promise<void> {
	return invoke("remove_theme", { id });
}

export async function exportTheme(id: string, dest: string): Promise<string> {
	return invoke<string>("export_theme", { id, dest });
}

function releaseImage(img: HTMLImageElement | null) {
	if (!img) return;
	img.onload = null;
	img.onerror = null;
	img.src = "";
	img.removeAttribute("src");
}

function buildThemeCSS(theme: ThemeResponse): string {
	let css = ":root {\n";
	for (const [key, value] of Object.entries(theme.variables)) {
		css += `  ${key}: ${value};\n`;
	}
	if (
		theme.bg_image_blur != null &&
		!("--bg-image-blur" in theme.variables)
	) {
		css += `  --bg-image-blur: ${theme.bg_image_blur}px;\n`;
	}
	if (
		theme.bg_image_opacity != null &&
		!("--bg-image-opacity" in theme.variables)
	) {
		css += `  --bg-image-opacity: ${theme.bg_image_opacity};\n`;
	}
	css += "}\n";
	return css;
}

function setThemeStyle(css: string) {
	let el = document.getElementById(THEME_VARS_ID) as HTMLStyleElement | null;
	if (!el) {
		el = document.createElement("style");
		el.id = THEME_VARS_ID;
		document.head.appendChild(el);
	}
	el.textContent = css;
}

function removeThemeStyle() {
	const el = document.getElementById(THEME_VARS_ID);
	if (el) el.remove();
}

function clearThemeResources() {
	removeThemeStyle();

	const root = document.documentElement;
	root.style.removeProperty("--bg-image");
	root.style.removeProperty("--bg-image-loaded");
	root.style.removeProperty("--font-loaded");

	releaseImage(currentImage);
	currentImage = null;

	if (currentBlobUrl) {
		URL.revokeObjectURL(currentBlobUrl);
		currentBlobUrl = null;
	}

	const existingCustomCss = document.getElementById(CUSTOM_CSS_ID);
	if (existingCustomCss) existingCustomCss.remove();

	const existingCustom = document.getElementById(CUSTOM_FONTS_ID);
	if (existingCustom) existingCustom.remove();

	for (const face of addedFonts) {
		document.fonts.delete(face);
	}
	addedFonts.clear();

	themeIcons.clear();
}

export async function applyTheme(themeId: string, opts?: { force?: boolean }) {
	if (!opts?.force && themeId === appliedThemeId) {
		return;
	}

	const gen = ++currentGeneration;

	clearThemeResources();

	if (gen !== currentGeneration) return;

	let theme: ThemeResponse | null = null;

	if (builtinThemes.find((t) => t.id === themeId)) {
		const res = await fetch(`/themes/${themeId}/${themeId}.json`);
		if (!res.ok) return;
		theme = await res.json();
	} else if (themeId.startsWith("user:")) {
		const id = themeId.slice(5);
		try {
			theme = await invoke<ThemeResponse>("get_user_theme", { id });
		} catch (e) {
			console.error("Error loading user theme:", e);
			return;
		}
	}

	if (!theme) return;
	if (gen !== currentGeneration) return;

	setThemeStyle(buildThemeCSS(theme));

	if (theme.icons) {
		for (const [key, val] of Object.entries(theme.icons)) {
			if (!val) continue;
			const url = themeId.startsWith("user:") ? convertFileSrc(val) : val;
			themeIcons.set(key, url);
		}
	}

	const root = document.documentElement;

	const bgImg = theme.bg_image;
	if (bgImg) {
		const imgUrl = themeId.startsWith("user:")
			? convertFileSrc(bgImg)
			: bgImg;

		root.style.setProperty("--bg-image-loaded", "0");

		const img = new Image();
		currentImage = img;
		img.onload = () => {
			img.onload = null;
			img.onerror = null;
			if (gen !== currentGeneration || currentImage !== img) return;
			currentImage = null;
			root.style.setProperty("--bg-image", `url("${imgUrl}")`);
			root.style.setProperty("--bg-image-loaded", "1");
		};
		img.onerror = () => {
			img.onload = null;
			img.onerror = null;
			if (gen !== currentGeneration || currentImage !== img) return;
			currentImage = null;
			root.style.setProperty("--bg-image", "none");
		};
		img.src = imgUrl;
	}

	if (theme.fonts && theme.fonts.length > 0) {
		root.style.setProperty("--font-loaded", "0");

		const loaded: Promise<void>[] = [];

		for (const font of theme.fonts) {
			const fontSrc = themeId.startsWith("user:")
				? convertFileSrc(font.src)
				: font.src;

			const descriptors: FontFaceDescriptors = {};
			if (font.weight) descriptors.weight = font.weight;
			if (font.style) descriptors.style = font.style;

			const face = new FontFace(
				font.family,
				`url(${fontSrc})`,
				descriptors,
			);
			face.display = "swap";

			loaded.push(
				face
					.load()
					.then(() => {
						document.fonts.add(face);
						addedFonts.add(face);
					})
					.catch((err) => {
						console.warn(
							`Font "${font.family}" failed to load:`,
							err,
							`src: ${fontSrc}`,
						);
					}),
			);
		}

		Promise.allSettled(loaded).then(() => {
			document.documentElement.style.setProperty("--font-loaded", "1");
		});
	} else {
		injectDefaultFonts();
	}

	if (theme.inject_css) {
		const blob = new Blob([theme.inject_css], { type: "text/css" });
		const url = URL.createObjectURL(blob);
		currentBlobUrl = url;
		const link = document.createElement("link");
		link.rel = "stylesheet";
		link.href = url;
		link.id = CUSTOM_CSS_ID;
		document.head.appendChild(link);
	}

	appliedThemeId = themeId;
}
