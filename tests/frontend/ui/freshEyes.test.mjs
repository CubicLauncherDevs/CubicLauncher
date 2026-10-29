import { expect, test } from "bun:test";
import {
	EYE_STEPS,
	eyePatchesOf,
	moveOf,
	poseAt,
	poseKeyOf,
	readEyeCopy,
} from "../../../src/lib/utils/freshEyes.ts";

const WHITE = [250, 250, 250, 255];
const IRIS = [40, 60, 160, 255];
const SKIN = [200, 150, 110, 255];
const LID = [190, 140, 100, 255];

// Una skin de 64×64 con la cara llena de piel y ojos en las filas dadas; con `copy`, la
// copia de los ojos en la celda (u, v) de las esquinas, como la escribe mosaicary.com.
function skin(rows, copy) {
	const rgba = new Uint8ClampedArray(64 * 64 * 4);
	const put = (u, v, colour) => rgba.set(colour, (v * 64 + u) * 4);
	for (let row = 0; row < 8; row++)
		for (let col = 0; col < 8; col++) put(8 + col, 8 + row, SKIN);
	const eye = [WHITE, IRIS, IRIS, WHITE];
	for (const row of rows)
		[1, 2, 5, 6].forEach((col, i) => put(8 + col, 8 + row, eye[i]));
	if (copy) {
		const [u, v] = copy;
		for (let i = 0; i < 4; i++) {
			put(u + i, v - 1, LID);
			rows.forEach((_, j) => put(u + i, v + j, eye[i]));
		}
	}
	return rgba;
}

test("lee la copia de unos ojos de una fila en su celda", () => {
	const eyes = readEyeCopy(skin([4], [4, 3]), 64, 64);
	expect(eyes?.rows).toEqual([4]);
	expect(eyes?.drawn[0]).toEqual([WHITE, IRIS, IRIS, WHITE]);
	expect(eyes?.lids).toEqual([LID, LID, LID, LID]);
});

test("lee unos ojos de dos filas", () => {
	expect(readEyeCopy(skin([4, 5], [24, 3]), 64, 64)?.rows).toEqual([4, 5]);
});

test("sin copia, o con una copia que no es la cara, no hay ojos", () => {
	expect(readEyeCopy(skin([4]), 64, 64)).toBeNull();
	// Copia de la fila 4 en la celda de la fila 5: no coincide con la cara.
	const wrong = skin([4], [4, 5]);
	expect(readEyeCopy(wrong, 64, 64)).toBeNull();
	// Las skins HD no llevan copia que el paquete lea.
	expect(
		readEyeCopy(new Uint8ClampedArray(128 * 128 * 4), 128, 128),
	).toBeNull();
});

test("la pupila se desliza y el párpado baja desde arriba", () => {
	const eyes = readEyeCopy(skin([4, 5], [24, 3]), 64, 64);
	const left = eyePatchesOf(eyes, { look: -1, lid: 0 });
	expect(
		left.find((p) => String(p.colour) === String(IRIS) && p.y === 4)?.x,
	).toBe(1);
	const shut = eyePatchesOf(eyes, { look: 0, lid: 0.5 }).filter(
		(p) => String(p.colour) === String(LID),
	);
	expect(shut.map((p) => [p.x, p.y, p.h])).toEqual(
		[1, 2, 5, 6].map((x) => [x, 4, 1]),
	);
});

test("cada movimiento empieza y acaba con los ojos como están dibujados", () => {
	for (const roll of [0.1, 0.55, 0.9]) {
		const keys = moveOf(1, () => roll);
		const end = keys[keys.length - 1].at;
		expect(poseKeyOf(poseAt(keys, 0), 1)).toBe("0 0");
		expect(poseKeyOf(poseAt(keys, end), 1)).toBe("0 0");
		expect(end).toBeGreaterThan(0);
	}
	expect(EYE_STEPS).toBe(8);
});
