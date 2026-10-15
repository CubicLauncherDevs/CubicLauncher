// Fuentes empaquetadas en el proyecto. Al dejar un archivo .woff2/.woff/.ttf/.otf
// en src/lib/assets/fonts/ queda registrado automáticamente (nombre de familia =
// segmento antes del primer guion; variante derivada de sufijos tipo Bold/Italic).
export const FONT_THEME = "theme";

const STYLE_ID = "cubic-project-fonts";

const modules = import.meta.glob("../assets/fonts/*.{woff2,woff,ttf,otf}", {
	eager: true,
	query: "?url",
	import: "default",
}) as Record<string, string>;

export interface ProjectFontFace {
	url: string;
	weight: number;
	style: "normal" | "italic";
	format: string;
}

export interface ProjectFont {
	family: string;
	faces: ProjectFontFace[];
}

function formatFor(path: string): string {
	const ext = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
	switch (ext) {
		case "woff2":
			return "woff2";
		case "woff":
			return "woff";
		case "ttf":
			return "truetype";
		case "otf":
			return "opentype";
		default:
			return ext;
	}
}

function basename(path: string): string {
	const file = path.slice(path.lastIndexOf("/") + 1);
	return file.slice(0, file.lastIndexOf("."));
}

const byFamily = new Map<string, ProjectFontFace[]>();
for (const [path, url] of Object.entries(modules)) {
	const name = basename(path);
	const dash = name.indexOf("-");
	const family = dash === -1 ? name : name.slice(0, dash);
	const variant = dash === -1 ? "" : name.slice(dash + 1);
	const face: ProjectFontFace = {
		url,
		weight: /bold/i.test(variant) ? 700 : 400,
		style: /italic|oblique/i.test(variant) ? "italic" : "normal",
		format: formatFor(path),
	};
	const faces = byFamily.get(family);
	if (faces) faces.push(face);
	else byFamily.set(family, [face]);
}

export const PROJECT_FONTS: ProjectFont[] = [...byFamily.entries()]
	.map(([family, faces]) => ({ family, faces }))
	.sort((a, b) => a.family.localeCompare(b.family));

export function projectFontsCSS(): string {
	let css = "";
	for (const font of PROJECT_FONTS) {
		for (const face of font.faces) {
			css += `@font-face {\n\tfont-family: "${font.family}";\n\tsrc: url("${face.url}") format("${face.format}");\n\tfont-weight: ${face.weight};\n\tfont-style: ${face.style};\n\tfont-display: swap;\n}\n`;
		}
	}
	return css;
}

export function injectProjectFonts(): void {
	if (document.getElementById(STYLE_ID)) return;
	const el = document.createElement("style");
	el.id = STYLE_ID;
	el.textContent = projectFontsCSS();
	document.head.appendChild(el);
}
