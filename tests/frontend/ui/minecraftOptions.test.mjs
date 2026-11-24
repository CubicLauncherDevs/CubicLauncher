import { expect, test } from "bun:test";
import {
	OPTION_DEFINITIONS,
	countAdvancedOptions,
	formatRangeValue,
	parseOptions,
	presetContent,
	rangeDisplayValue,
	rangeRawValue,
	removeOption,
	serializeOptions,
	setOption,
} from "../../../src/lib/utils/minecraftOptions.ts";

const SAMPLE = [
	"version:3465",
	"fov:0.5",
	"key_key.forward:key.keyboard.w",
	"key_key.jump:key.keyboard.space",
	'resourcePacks:["vanilla","fabric"]',
	'renderClouds:"fast"',
	"anUnknownOption:hello:world",
].join("\n");

test("parseOptions keeps the first value and tolerates colons in values", () => {
	const values = parseOptions(SAMPLE);
	expect(values.get("fov")).toBe("0.5");
	expect(values.get("key_key.forward")).toBe("key.keyboard.w");
	expect(values.get("resourcePacks")).toBe('["vanilla","fabric"]');
	expect(values.get("renderClouds")).toBe('"fast"');
	expect(values.get("anUnknownOption")).toBe("hello:world");
});

test("setOption replaces in place and preserves unknown lines", () => {
	const updated = setOption(SAMPLE, "fov", "0.75");
	expect(updated.split("\n")).toEqual([
		"version:3465",
		"fov:0.75",
		"key_key.forward:key.keyboard.w",
		"key_key.jump:key.keyboard.space",
		'resourcePacks:["vanilla","fabric"]',
		'renderClouds:"fast"',
		"anUnknownOption:hello:world",
	]);
});

test("setOption appends new keys and keeps the trailing newline", () => {
	const updated = setOption("a:1\n", "b", "2");
	expect(updated).toBe("a:1\nb:2\n");
});

test("setOption supports CRLF files", () => {
	const updated = setOption("a:1\r\nb:2\r\n", "b", "3");
	expect(updated).toBe("a:1\r\nb:3\r\n");
});

test("removeOption deletes only the matching key", () => {
	expect(removeOption("a:1\nb:2\nc:3\n", "b")).toBe("a:1\nc:3\n");
	expect(removeOption("a:1\nb:2", "b")).toBe("a:1");
	expect(removeOption("a:1\r\nb:2\r\n", "a")).toBe("b:2\r\n");
	expect(removeOption("a:1", "missing")).toBe("a:1");
	expect(removeOption("", "a")).toBe("");
});

test("countAdvancedOptions ignores known keys and keybinds", () => {
	expect(countAdvancedOptions(SAMPLE)).toBe(5);
});

test("range conversions map between raw and displayed values", () => {
	const fov = OPTION_DEFINITIONS.find((option) => option.key === "fov");
	expect(fov?.kind).toBe("range");
	if (fov?.kind !== "range") return;

	expect(rangeDisplayValue(fov, "0.5")).toBe(90);
	expect(rangeDisplayValue(fov, "0.0")).toBe(70);
	expect(rangeRawValue(fov, 90)).toBe("0.5");
	expect(rangeRawValue(fov, 70)).toBe("0");
	expect(formatRangeValue(fov, 90)).toBe("90°");

	const master = OPTION_DEFINITIONS.find(
		(option) => option.key === "soundCategory_master",
	);
	if (master?.kind !== "range") throw new Error("missing master volume");
	expect(rangeDisplayValue(master, "0.5")).toBe(50);
	expect(rangeRawValue(master, 25)).toBe("0.25");

	const renderDistance = OPTION_DEFINITIONS.find(
		(option) => option.key === "renderDistance",
	);
	if (renderDistance?.kind !== "range") throw new Error("missing render");
	expect(rangeDisplayValue(renderDistance, "12")).toBe(12);
	expect(rangeRawValue(renderDistance, 16)).toBe("16");
});

test("serializeOptions keeps insertion order", () => {
	expect(
		serializeOptions([
			["fov", "0.5"],
			["renderDistance", "12"],
		]),
	).toBe("fov:0.5\nrenderDistance:12");
});

test("vanilla preset covers every supported option", () => {
	const content = presetContent("vanilla");
	expect(content.split("\n")).toHaveLength(OPTION_DEFINITIONS.length);
	const values = parseOptions(content);
	for (const option of OPTION_DEFINITIONS) {
		expect(values.get(option.key)).toBe(option.defaultRaw);
	}
});

test("performance and pvp presets set their signature values", () => {
	const performance = parseOptions(presetContent("performance"));
	expect(performance.get("renderDistance")).toBe("6");
	expect(performance.get("graphicsMode")).toBe("0");
	expect(performance.get("renderClouds")).toBe('"false"');
	expect(performance.get("maxFps")).toBe("120");

	const pvp = parseOptions(presetContent("pvp"));
	expect(pvp.get("fov")).toBe("0.5");
	expect(pvp.get("maxFps")).toBe("240");
	expect(pvp.get("toggleSprint")).toBe("true");
});

test("presetContent returns an empty string for unknown presets", () => {
	expect(presetContent("does-not-exist")).toBe("");
});
