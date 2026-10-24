import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const srcDir = path.join(root, "src");
const themesDir = path.join(root, "static", "themes");
const output = path.join(root, "docs", "themes", "v2", "tokens.md");

async function walk(dir) {
	const entries = await readdir(dir, { withFileTypes: true });
	const files = [];
	for (const entry of entries) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			files.push(...(await walk(full)));
		} else {
			files.push(full);
		}
	}
	return files;
}

function escapeCell(value) {
	return value.replaceAll("|", "\\|").replaceAll("\n", " ").trim();
}

function truncate(value, max = 72) {
	return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

function categoryFor(token) {
	const key = token.replace(/^--/, "");
	if (
		/^(accent|color-|error$|warning$|success$|info$|danger$|clr-|download-)/.test(
			key,
		)
	) {
		return "Color y estado";
	}
	if (/^(bg-|surface-|market-|toast-|media-|viewer-|tutorial-)/.test(key)) {
		return "Fondos y superficies";
	}
	if (key.startsWith("text-")) return "Texto";
	if (/^(border|shadow|glow)/.test(key)) return "Bordes, radios y sombras";
	if (/^(font|line-height)/.test(key)) return "Tipografía";
	if (
		/^(space|sidebar|layout|drawer|grid|columns|control|button|content|detail|empty|icon-|disabled|virtual|confirmation)/.test(
			key,
		)
	) {
		return "Espaciado y layout";
	}
	if (/^(transition|animation)/.test(key)) return "Movimiento";
	if (key.startsWith("scrollbar")) return "Scrollbar";
	if (key.startsWith("log-")) return "Logs";
	if (key.startsWith("backdrop")) return "Backdrop";
	if (key.startsWith("component-")) return "Componentes";
	return "Otros";
}

const files = (await walk(srcDir)).filter((file) =>
	/\.(svelte|css)$/.test(file),
);
const usage = new Map();

for (const file of files) {
	const content = await readFile(file, "utf8");
	for (const match of content.matchAll(/var\(\s*(--[A-Za-z0-9_-]+)/g)) {
		const token = match[1];
		let entry = usage.get(token);
		if (!entry) {
			entry = { count: 0, files: new Set() };
			usage.set(token, entry);
		}
		entry.count += 1;
		entry.files.add(path.relative(root, file));
	}
}

const resetCss = await readFile(
	path.join(srcDir, "styles", "shared", "reset.css"),
	"utf8",
);
const defaults = new Map();
for (const match of resetCss.matchAll(
	/^\s*(--[A-Za-z0-9_-]+)\s*:\s*([^;]+);/gm,
)) {
	defaults.set(match[1], match[2].trim());
}

const builtin = new Map();
try {
	const themeDirs = await readdir(themesDir, { withFileTypes: true });
	for (const dir of themeDirs) {
		if (!dir.isDirectory()) continue;
		const themePath = path.join(themesDir, dir.name, `${dir.name}.json`);
		try {
			const json = JSON.parse(await readFile(themePath, "utf8"));
			for (const token of Object.keys(json.variables ?? {})) {
				builtin.set(token, (builtin.get(token) ?? 0) + 1);
			}
		} catch {
			// Ignore themes that are not V1 JSON.
		}
	}
} catch {
	// No built-in themes directory.
}

const groups = new Map();
for (const token of [...usage.keys()].sort()) {
	const category = categoryFor(token);
	if (!groups.has(category)) groups.set(category, []);
	groups.get(category).push(token);
}

const totalUsages = [...usage.values()].reduce(
	(total, entry) => total + entry.count,
	0,
);
const usedWithoutDefault = [...usage.keys()].filter(
	(token) => !defaults.has(token),
);
const defaultsWithoutUsage = [...defaults.keys()].filter(
	(token) => !usage.has(token),
);

const lines = [
	"# Catálogo de tokens CSS",
	"",
	"> Generado automáticamente por `bun run themes:tokens`. No editar a mano.",
	"",
	"Este catálogo lista las variables CSS que el frontend consume actualmente.",
	"Los temas V2 pueden sobrescribirlas desde las secciones de",
	"`Definition.toml` (`colors`, `backgrounds`, `text`, `layout`, etc.).",
	"",
	"## Resumen",
	"",
	`- Tokens únicos usados: **${usage.size}**`,
	`- Usos totales en \`src/\`: **${totalUsages}**`,
	`- Tokens con valor por defecto en \`reset.css\`: **${usage.size - usedWithoutDefault.length}**`,
	`- Tokens usados sin valor por defecto en \`reset.css\`: **${usedWithoutDefault.length}**`,
	`- Tokens definidos en \`reset.css\` sin uso actual: **${defaultsWithoutUsage.length}**`,
	"",
	"## Cómo regenerar",
	"",
	"```bash",
	"bun run themes:tokens",
	"```",
	"",
	"## Tokens por categoría",
	"",
];

for (const [category, tokens] of groups) {
	lines.push(`### ${category}`, "");
	lines.push(
		"| Token | Usos | Valor en `reset.css` | Temas built-in | Ejemplos |",
	);
	lines.push("|---|---:|---|---:|---|");
	for (const token of tokens) {
		const entry = usage.get(token);
		const defaultValue = defaults.get(token);
		const examples = [...entry.files]
			.sort()
			.slice(0, 2)
			.map((file) => `\`${file}\``)
			.join("<br>");
		lines.push(
			`| \`${token}\` | ${entry.count} | ${
				defaultValue ? `\`${escapeCell(truncate(defaultValue))}\`` : "—"
			} | ${builtin.get(token) ?? "—"} | ${examples || "—"} |`,
		);
	}
	lines.push("");
}

if (usedWithoutDefault.length > 0) {
	lines.push("## Tokens usados sin valor por defecto", "");
	lines.push(
		"Estos tokens dependen de que el tema o `reset.css` los defina. Si un tema no los define, el navegador usará el fallback del `var()` correspondiente o el valor inicial de la propiedad.",
		"",
	);
	for (const token of usedWithoutDefault.sort()) {
		lines.push(`- \`${token}\``);
	}
	lines.push("");
}

if (defaultsWithoutUsage.length > 0) {
	lines.push("## Tokens definidos en `reset.css` sin uso actual", "");
	for (const token of defaultsWithoutUsage.sort()) {
		lines.push(
			`- \`${token}\`: \`${escapeCell(truncate(defaults.get(token), 96))}\``,
		);
	}
	lines.push("");
}

await mkdir(path.dirname(output), { recursive: true });
await writeFile(output, `${lines.join("\n")}\n`, "utf8");
console.log(`Catálogo generado en ${path.relative(root, output)}`);
