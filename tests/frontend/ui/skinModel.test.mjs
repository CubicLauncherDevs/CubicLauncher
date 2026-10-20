import { expect, test } from "bun:test";
import { detectSkinModel } from "../../../src/lib/utils/skinModel.ts";

// Skin sintética: se pintan rectángulos opacos en coordenadas de 64×64.
function skin({ width = 64, height = 64, paint = [], alpha = 255 } = {}) {
	const scale = width / 64;
	const rgba = new Uint8ClampedArray(width * height * 4);
	for (const { x, y, w, h } of paint) {
		for (let py = y * scale; py < (y + h) * scale; py++) {
			for (let px = x * scale; px < (x + w) * scale; px++) {
				rgba[(py * width + px) * 4 + 3] = alpha;
			}
		}
	}
	return { rgba, width, height };
}

const detect = ({ rgba, width, height }) =>
	detectSkinModel(rgba, width, height);

// Brazos tal como los dibuja cada modelo (cara de arriba, abajo y las cuatro laterales).
const CLASSIC_ARMS = [
	{ x: 44, y: 16, w: 8, h: 4 },
	{ x: 40, y: 20, w: 16, h: 12 },
	{ x: 36, y: 48, w: 8, h: 4 },
	{ x: 32, y: 52, w: 16, h: 12 },
];
const SLIM_ARMS = [
	{ x: 44, y: 16, w: 6, h: 4 },
	{ x: 40, y: 20, w: 14, h: 12 },
	{ x: 36, y: 48, w: 6, h: 4 },
	{ x: 32, y: 52, w: 14, h: 12 },
];

test("a skin drawn with 3 px arms is detected as slim", () => {
	expect(detect(skin({ paint: SLIM_ARMS }))).toBe("slim");
});

test("a skin drawn with 4 px arms is detected as classic", () => {
	expect(detect(skin({ paint: CLASSIC_ARMS }))).toBe("classic");
});

test("one drawn column in any slim-only zone means classic", () => {
	for (const zone of [
		{ x: 55, y: 25, w: 1, h: 1 },
		{ x: 51, y: 17, w: 1, h: 1 },
		{ x: 46, y: 60, w: 1, h: 1 },
		{ x: 43, y: 49, w: 1, h: 1 },
	]) {
		expect(detect(skin({ paint: [...SLIM_ARMS, zone] }))).toBe("classic");
	}
});

test("antialiasing leftovers in the slim-only zones do not count", () => {
	const image = skin({ paint: SLIM_ARMS });
	const stray = skin({ paint: [{ x: 54, y: 20, w: 2, h: 12 }], alpha: 3 });
	for (let i = 3; i < image.rgba.length; i += 4) {
		image.rgba[i] = Math.max(image.rgba[i], stray.rgba[i]);
	}
	expect(detect(image)).toBe("slim");
});

test("an empty arm falls back to classic instead of guessing slim", () => {
	expect(detect(skin())).toBe("classic");
});

test("legacy 64x32 skins are always classic", () => {
	const legacy = skin({
		height: 32,
		paint: [
			{ x: 44, y: 16, w: 6, h: 4 },
			{ x: 40, y: 20, w: 14, h: 12 },
		],
	});
	expect(detect(legacy)).toBe("classic");
});

test("HD skins are read with the same rule", () => {
	expect(detect(skin({ width: 128, height: 128, paint: SLIM_ARMS }))).toBe(
		"slim",
	);
	expect(detect(skin({ width: 128, height: 128, paint: CLASSIC_ARMS }))).toBe(
		"classic",
	);
	expect(detect(skin({ width: 128, height: 64, paint: SLIM_ARMS }))).toBe(
		"classic",
	);
});
