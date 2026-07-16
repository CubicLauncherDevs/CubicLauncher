import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { ThemeEntry } from "../types/types";

const builtinThemes: ThemeEntry[] = [
	{
		id: "dark",
		name: "Oscuro",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
	},
	{
		id: "lima",
		name: "Lima",
		author: "CubicLauncher",
		version: "1.0",
		type: "builtin",
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

function removeDefaultFonts() {
	const el = document.getElementById(DEFAULT_FONTS_ID);
	if (el) el.remove();
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
	if (theme.bg_image_blur != null) {
		css += `  --bg-image-blur: ${theme.bg_image_blur}px;\n`;
	}
	if (theme.bg_image_opacity != null) {
		css += `  --bg-image-opacity: ${theme.bg_image_opacity};\n`;
	}
	css += `  --font-loaded: 1;\n`;
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
}

export async function applyTheme(themeId: string) {
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
		removeDefaultFonts();

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
}
