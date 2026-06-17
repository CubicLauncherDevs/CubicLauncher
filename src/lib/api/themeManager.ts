import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { ThemeEntry } from "../types/types";
import { t } from "$lib/i18n";
import { showWarning } from "$lib/state/state.svelte";

const builtinThemes: ThemeEntry[] = [
	{ id: "dark", name: "Oscuro", author: "CubicLauncher", version: "1.0", type: "builtin" },
	{ id: "lima", name: "Lima", author: "CubicLauncher", version: "1.0", type: "builtin" },
];

export interface UserTheme {
	name: string;
	version?: string;
	variables: Record<string, string>;
	bg_image?: string | null;
	bg_image_blur?: string | null;
	bg_image_opacity?: number | null;
	bg_image_warning_key?: string | null;
}

let currentImage: HTMLImageElement | null = null;
let currentGeneration = 0;

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

export async function applyTheme(themeId: string) {
	const gen = ++currentGeneration;

	let theme: UserTheme | null = null;

	if (builtinThemes.find((t) => t.id === themeId)) {
		const res = await fetch(`/themes/${themeId}/${themeId}.json`);
		if (!res.ok) return;
		theme = await res.json();
	} else if (themeId.startsWith("user:")) {
		const id = themeId.slice(5);
		try {
			theme = await invoke<UserTheme>("get_user_theme", { id });
		} catch (e) {
			console.error("Error loading user theme:", e);
			return;
		}
	}

	if (!theme) return;
	if (gen !== currentGeneration) return;

	if (currentImage) {
		currentImage.src = "";
		currentImage = null;
	}

	const root = document.documentElement;
	const style = root.style;
	for (let i = style.length - 1; i >= 0; i--) {
		const prop = style.item(i);
		if (prop.startsWith("--")) {
			style.removeProperty(prop);
		}
	}

	if (theme.bg_image_warning_key) {
		showWarning(t("themes.warning.title"), t(theme.bg_image_warning_key));
	}

	for (const [key, value] of Object.entries(theme.variables)) {
		root.style.setProperty(key, value);
	}

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
	if (theme.bg_image_blur) {
		root.style.setProperty("--bg-image-blur", theme.bg_image_blur);
	}
	if (theme.bg_image_opacity != null) {
		root.style.setProperty(
			"--bg-image-opacity",
			String(theme.bg_image_opacity),
		);
	}
}
