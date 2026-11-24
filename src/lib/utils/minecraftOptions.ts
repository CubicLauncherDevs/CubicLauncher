/**
 * Utilidades para leer y escribir `options.txt` sin perder información.
 *
 * Minecraft guarda las opciones como pares `clave:valor`, una por línea. Este
 * módulo trabaja sobre el texto completo para conservar el orden original, las
 * keybinds y cualquier clave desconocida al editar desde la interfaz.
 */

export type OptionCategory = "graphics" | "sound" | "interface" | "gameplay";
export type RangeTransform = "plain" | "percent" | "fov";

export interface SelectChoice {
	/** Valor tal y como aparece en `options.txt`. */
	value: string;
}

export type OptionDefinition =
	| {
			kind: "toggle";
			key: string;
			category: OptionCategory;
			defaultRaw: string;
	  }
	| {
			kind: "range";
			key: string;
			category: OptionCategory;
			defaultRaw: string;
			transform: RangeTransform;
			min: number;
			max: number;
			step: number;
			unit?: string;
	  }
	| {
			kind: "select";
			key: string;
			category: OptionCategory;
			defaultRaw: string;
			choices: SelectChoice[];
	  };

export const OPTION_CATEGORIES: readonly OptionCategory[] = [
	"graphics",
	"sound",
	"interface",
	"gameplay",
];

const TRUE = "true";
const FALSE = "false";

const soundKey = (name: string) => `soundCategory_${name}`;

export const OPTION_DEFINITIONS: readonly OptionDefinition[] = [
	// ── Gráficos ────────────────────────────────────────────────────────────
	{
		kind: "select",
		key: "graphicsMode",
		category: "graphics",
		defaultRaw: "1",
		choices: [{ value: "0" }, { value: "1" }, { value: "2" }],
	},
	{
		kind: "range",
		key: "renderDistance",
		category: "graphics",
		defaultRaw: "12",
		transform: "plain",
		min: 2,
		max: 32,
		step: 1,
	},
	{
		kind: "range",
		key: "simulationDistance",
		category: "graphics",
		defaultRaw: "12",
		transform: "plain",
		min: 5,
		max: 32,
		step: 1,
	},
	{
		kind: "select",
		key: "maxFps",
		category: "graphics",
		defaultRaw: "120",
		choices: [
			{ value: "30" },
			{ value: "60" },
			{ value: "90" },
			{ value: "120" },
			{ value: "144" },
			{ value: "240" },
			{ value: "260" },
		],
	},
	{
		kind: "range",
		key: "fov",
		category: "graphics",
		defaultRaw: "0.0",
		transform: "fov",
		min: 30,
		max: 110,
		step: 1,
		unit: "°",
	},
	{
		kind: "range",
		key: "gamma",
		category: "graphics",
		defaultRaw: "0.5",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "select",
		key: "guiScale",
		category: "graphics",
		defaultRaw: "0",
		choices: [
			{ value: "0" },
			{ value: "1" },
			{ value: "2" },
			{ value: "3" },
			{ value: "4" },
		],
	},
	{
		kind: "select",
		key: "particles",
		category: "graphics",
		defaultRaw: "0",
		choices: [{ value: "0" }, { value: "1" }, { value: "2" }],
	},
	{
		kind: "select",
		key: "renderClouds",
		category: "graphics",
		defaultRaw: '"true"',
		choices: [
			{ value: '"true"' },
			{ value: '"fast"' },
			{ value: '"false"' },
		],
	},
	{
		kind: "toggle",
		key: "ao",
		category: "graphics",
		defaultRaw: TRUE,
	},
	{
		kind: "toggle",
		key: "entityShadows",
		category: "graphics",
		defaultRaw: TRUE,
	},
	{
		kind: "toggle",
		key: "enableVsync",
		category: "graphics",
		defaultRaw: TRUE,
	},
	{
		kind: "range",
		key: "biomeBlendRadius",
		category: "graphics",
		defaultRaw: "2",
		transform: "plain",
		min: 0,
		max: 7,
		step: 1,
	},
	{
		kind: "select",
		key: "mipmapLevels",
		category: "graphics",
		defaultRaw: "4",
		choices: [
			{ value: "0" },
			{ value: "1" },
			{ value: "2" },
			{ value: "3" },
			{ value: "4" },
		],
	},

	// ── Sonido ──────────────────────────────────────────────────────────────
	{
		kind: "range",
		key: soundKey("master"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("music"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("record"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("weather"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("block"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("hostile"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("neutral"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("player"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("ambient"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: soundKey("voice"),
		category: "sound",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "select",
		key: "narrator",
		category: "sound",
		defaultRaw: "0",
		choices: [
			{ value: "0" },
			{ value: "1" },
			{ value: "2" },
			{ value: "3" },
		],
	},

	// ── Interfaz ────────────────────────────────────────────────────────────
	{
		kind: "select",
		key: "lang",
		category: "interface",
		defaultRaw: "es_es",
		choices: [
			{ value: "es_es" },
			{ value: "es_mx" },
			{ value: "en_us" },
			{ value: "pt_br" },
			{ value: "fr_fr" },
			{ value: "de_de" },
			{ value: "it_it" },
			{ value: "ru_ru" },
			{ value: "pl_pl" },
			{ value: "tr_tr" },
			{ value: "zh_cn" },
			{ value: "ja_jp" },
			{ value: "ko_kr" },
		],
	},
	{
		kind: "range",
		key: "chatOpacity",
		category: "interface",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: "chatScale",
		category: "interface",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: "chatWidth",
		category: "interface",
		defaultRaw: "1.0",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: "textBackgroundOpacity",
		category: "interface",
		defaultRaw: "0.5",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "toggle",
		key: "chatColors",
		category: "interface",
		defaultRaw: TRUE,
	},
	{
		kind: "toggle",
		key: "chatLinks",
		category: "interface",
		defaultRaw: TRUE,
	},
	{
		kind: "toggle",
		key: "backgroundForChatOnly",
		category: "interface",
		defaultRaw: TRUE,
	},
	{
		kind: "toggle",
		key: "reducedDebugInfo",
		category: "interface",
		defaultRaw: FALSE,
	},
	{
		kind: "toggle",
		key: "advancedItemTooltips",
		category: "interface",
		defaultRaw: FALSE,
	},
	{
		kind: "toggle",
		key: "heldItemTooltips",
		category: "interface",
		defaultRaw: TRUE,
	},

	// ── Juego y controles ───────────────────────────────────────────────────
	{
		kind: "toggle",
		key: "autoJump",
		category: "gameplay",
		defaultRaw: FALSE,
	},
	{
		kind: "toggle",
		key: "toggleSprint",
		category: "gameplay",
		defaultRaw: FALSE,
	},
	{
		kind: "toggle",
		key: "toggleCrouch",
		category: "gameplay",
		defaultRaw: FALSE,
	},
	{
		kind: "toggle",
		key: "invertYMouse",
		category: "gameplay",
		defaultRaw: FALSE,
	},
	{
		kind: "range",
		key: "mouseSensitivity",
		category: "gameplay",
		defaultRaw: "0.5",
		transform: "percent",
		min: 0,
		max: 100,
		step: 1,
		unit: "%",
	},
	{
		kind: "range",
		key: "mouseWheelSensitivity",
		category: "gameplay",
		defaultRaw: "1.0",
		transform: "plain",
		min: 0.1,
		max: 2,
		step: 0.1,
	},
	{
		kind: "select",
		key: "mainHand",
		category: "gameplay",
		defaultRaw: '"right"',
		choices: [{ value: '"right"' }, { value: '"left"' }],
	},
	{
		kind: "select",
		key: "attackIndicator",
		category: "gameplay",
		defaultRaw: "1",
		choices: [{ value: "0" }, { value: "1" }, { value: "2" }],
	},
	{
		kind: "toggle",
		key: "pauseOnLostFocus",
		category: "gameplay",
		defaultRaw: TRUE,
	},
	{
		kind: "toggle",
		key: "forceUnicodeFont",
		category: "gameplay",
		defaultRaw: FALSE,
	},
	{
		kind: "toggle",
		key: "skipMultiplayerWarning",
		category: "gameplay",
		defaultRaw: FALSE,
	},
];

export const OPTION_BY_KEY: ReadonlyMap<string, OptionDefinition> = new Map(
	OPTION_DEFINITIONS.map((option) => [option.key, option]),
);

export const OPTIONS_BY_CATEGORY: Readonly<
	Record<OptionCategory, readonly OptionDefinition[]>
> = OPTION_CATEGORIES.reduce(
	(acc, category) => {
		acc[category] = OPTION_DEFINITIONS.filter(
			(option) => option.category === category,
		);
		return acc;
	},
	{} as Record<OptionCategory, OptionDefinition[]>,
);

// ── Parseo y serialización ───────────────────────────────────────────────────

function splitLines(text: string): { lines: string[]; separator: string } {
	const separator = text.includes("\r\n") ? "\r\n" : "\n";
	return { lines: text.split(separator), separator };
}

function lineKey(line: string): string | null {
	const index = line.indexOf(":");
	if (index <= 0) return null;
	return line.slice(0, index);
}

/** Devuelve un mapa con la primera aparición de cada clave. */
export function parseOptions(text: string): Map<string, string> {
	const values = new Map<string, string>();
	if (!text) return values;
	for (const line of text.split(/\r?\n/)) {
		const key = lineKey(line);
		if (key !== null && !values.has(key)) {
			values.set(key, line.slice(key.length + 1));
		}
	}
	return values;
}

export function getOption(text: string, key: string): string | undefined {
	return parseOptions(text).get(key);
}

export function hasOption(text: string, key: string): boolean {
	return parseOptions(text).has(key);
}

/** Claves presentes en el archivo que no tienen un control en la interfaz. */
export function countAdvancedOptions(text: string): number {
	let count = 0;
	for (const key of parseOptions(text).keys()) {
		if (!OPTION_BY_KEY.has(key)) count++;
	}
	return count;
}

/**
 * Reemplaza el valor de `key` conservando el resto del archivo.
 *
 * Si la clave ya existe se reemplaza en su sitio; si no, se añade al final
 * respetando el salto de línea final original.
 */
export function setOption(text: string, key: string, value: string): string {
	if (!text) return `${key}:${value}`;
	const { lines, separator } = splitLines(text);
	const prefix = `${key}:`;
	for (let index = 0; index < lines.length; index++) {
		if (lines[index].startsWith(prefix)) {
			lines[index] = `${prefix}${value}`;
			return lines.join(separator);
		}
	}
	const last = lines.length - 1;
	if (last >= 0 && lines[last] === "") {
		lines.splice(last, 0, `${prefix}${value}`);
	} else {
		lines.push(`${prefix}${value}`);
	}
	return lines.join(separator);
}

export function removeOption(text: string, key: string): string {
	if (!text) return text;
	const { lines, separator } = splitLines(text);
	const prefix = `${key}:`;
	const filtered = lines.filter((line) => !line.startsWith(prefix));
	if (filtered.length === lines.length) return text;
	return filtered.join(separator);
}

export function serializeOptions(entries: Iterable<[string, string]>): string {
	return Array.from(entries, ([key, value]) => `${key}:${value}`).join("\n");
}

// ── Conversión entre valor crudo y valor mostrado ────────────────────────────

function trimNumber(value: number): string {
	if (!Number.isFinite(value)) return "0";
	return value
		.toFixed(2)
		.replace(/\.00$/, "")
		.replace(/(\.\d)0$/, "$1");
}

function toRawForTransform(transform: RangeTransform, display: number): string {
	if (transform === "percent") return trimNumber(display / 100);
	if (transform === "fov") return trimNumber((display - 70) / 40);
	return trimNumber(display);
}

function toDisplayForTransform(
	transform: RangeTransform,
	raw: string | undefined,
	defaultRaw: string,
): number {
	const parsed = Number(raw ?? defaultRaw);
	const safe = Number.isFinite(parsed) ? parsed : Number(defaultRaw);
	if (transform === "percent") return Math.round(safe * 100);
	if (transform === "fov") return Math.round(70 + safe * 40);
	return safe;
}

/** Valor que debe mostrar un slider para el `raw` actual. */
export function rangeDisplayValue(
	option: Extract<OptionDefinition, { kind: "range" }>,
	raw: string | undefined,
): number {
	return toDisplayForTransform(option.transform, raw, option.defaultRaw);
}

/** Convierte el valor visible de un slider al `raw` de `options.txt`. */
export function rangeRawValue(
	option: Extract<OptionDefinition, { kind: "range" }>,
	display: number,
): string {
	return toRawForTransform(option.transform, display);
}

export function formatRangeValue(
	option: Extract<OptionDefinition, { kind: "range" }>,
	display: number,
): string {
	const value = Number.isInteger(display)
		? String(display)
		: trimNumber(display);
	return `${value}${option.unit ?? ""}`;
}

// ── Presets ──────────────────────────────────────────────────────────────────

export interface OptionsPreset {
	id: string;
	values: Readonly<Record<string, string>>;
}

export const OPTION_PRESETS: readonly OptionsPreset[] = [
	{
		id: "vanilla",
		values: Object.fromEntries(
			OPTION_DEFINITIONS.map((option) => [option.key, option.defaultRaw]),
		),
	},
	{
		id: "performance",
		values: {
			graphicsMode: "0",
			renderDistance: "6",
			simulationDistance: "5",
			maxFps: "120",
			fov: "0.0",
			guiScale: "2",
			particles: "2",
			renderClouds: '"false"',
			ao: FALSE,
			entityShadows: FALSE,
			enableVsync: FALSE,
			biomeBlendRadius: "0",
			mipmapLevels: "0",
			[soundKey("master")]: "1.0",
			[soundKey("music")]: "0.0",
			mouseSensitivity: "0.5",
		},
	},
	{
		id: "pvp",
		values: {
			graphicsMode: "0",
			renderDistance: "8",
			simulationDistance: "8",
			maxFps: "240",
			fov: "0.5",
			guiScale: "2",
			particles: "2",
			renderClouds: '"false"',
			ao: FALSE,
			entityShadows: FALSE,
			enableVsync: FALSE,
			biomeBlendRadius: "2",
			mipmapLevels: "0",
			[soundKey("master")]: "1.0",
			[soundKey("music")]: "0.0",
			mouseSensitivity: "0.5",
			toggleSprint: TRUE,
			mainHand: '"right"',
		},
	},
	{
		id: "quality",
		values: {
			graphicsMode: "2",
			renderDistance: "16",
			simulationDistance: "12",
			maxFps: "144",
			fov: "0.0",
			guiScale: "2",
			particles: "0",
			renderClouds: '"true"',
			ao: TRUE,
			entityShadows: TRUE,
			enableVsync: TRUE,
			biomeBlendRadius: "5",
			mipmapLevels: "4",
			[soundKey("master")]: "1.0",
			[soundKey("music")]: "1.0",
			mouseSensitivity: "0.5",
		},
	},
];

export const OPTION_PRESET_BY_ID: ReadonlyMap<string, OptionsPreset> = new Map(
	OPTION_PRESETS.map((preset) => [preset.id, preset]),
);

/** Contenido de `options.txt` para un preset, en el orden de las definiciones. */
export function presetContent(presetId: string): string {
	const preset = OPTION_PRESET_BY_ID.get(presetId);
	if (!preset) return "";
	const entries: [string, string][] = [];
	for (const option of OPTION_DEFINITIONS) {
		const value = preset.values[option.key];
		if (value !== undefined) entries.push([option.key, value]);
	}
	return serializeOptions(entries);
}
