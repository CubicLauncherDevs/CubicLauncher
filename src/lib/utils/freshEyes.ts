// Ojos vivos de Fresh Moves en la vista previa: el paquete de recursos Fresh Moves (para
// Entity Model Features) no busca los ojos en la cara, los dibuja a partir de una copia que la
// skin lleva en esquinas que el juego nunca muestra. Aquí se lee esa copia y se mueven los
// ojos igual que en las tarjetas de mosaicary.com (los mismos tiempos y la misma forma de
// moverse), para que quien elige una skin vea lo que verá en el juego con el paquete.
//
// Sin copia en la skin no hay nada que mover: no se adivinan ojos en la cara.

/** Un color RGBA, 0 a 255. */
export type Rgba = readonly [number, number, number, number];

/** Dónde lleva la skin la copia de sus ojos: su forma, las filas de la cara y la esquina. */
interface EyeCell {
	rows: readonly number[];
	/** Columna izquierda y primera fila de la copia; la fila de encima es el color del párpado. */
	u: number;
	v: number;
}

// Las celdas del paquete (su player.jem, versión 3.1.1): dos por uno y dos por dos, a cada
// una de las cuatro alturas.
const EYE_CELLS: readonly EyeCell[] = [
	{ rows: [6], u: 4, v: 7 },
	{ rows: [5], u: 4, v: 5 },
	{ rows: [4], u: 4, v: 3 },
	{ rows: [3], u: 4, v: 1 },
	{ rows: [5, 6], u: 24, v: 6 },
	{ rows: [4, 5], u: 24, v: 3 },
	{ rows: [3, 4], u: 28, v: 6 },
	{ rows: [2, 3], u: 28, v: 3 },
];

/** Las columnas de la cara que se copian: blanco y pupila de un ojo, pupila y blanco del otro. */
const COLUMNS = [1, 2, 5, 6] as const;

/** Los ojos de una cara, listos para moverlos. */
export interface LiveEyes {
	/** Las filas de la cara en que están, de arriba abajo. */
	rows: readonly number[];
	/** Por fila, las columnas 1, 2, 5 y 6 tal como están dibujadas. */
	drawn: readonly (readonly Rgba[])[];
	/** El color del párpado sobre cada una de esas columnas. */
	lids: readonly Rgba[];
}

/**
 * Los ojos que la skin lleva copiados para Fresh Moves, o null. `rgba` es la skin de 64 de
 * ancho (64×64 o 64×32). La celda buscada es la que copia exactamente la cara: una skin
 * preparada a mano puede tener cejas o pestañas en otras celdas de las esquinas.
 */
export function readEyeCopy(
	rgba: ArrayLike<number>,
	width: number,
	height: number,
): LiveEyes | null {
	if (width !== 64 || (height !== 64 && height !== 32)) return null;
	const at = (u: number, v: number): Rgba => {
		const i = (v * width + u) * 4;
		return [
			rgba[i] ?? 0,
			rgba[i + 1] ?? 0,
			rgba[i + 2] ?? 0,
			rgba[i + 3] ?? 0,
		];
	};
	const same = (a: Rgba, b: Rgba) =>
		a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
	for (const cell of EYE_CELLS) {
		const lids = COLUMNS.map((_, i) => at(cell.u + i, cell.v - 1));
		const drawn = cell.rows.map((row, j) =>
			COLUMNS.map((column, i) => {
				const copy = at(cell.u + i, cell.v + j);
				return { copy, face: at(8 + column, 8 + row) };
			}),
		);
		const whole =
			lids.every((one) => one[3] >= 128) &&
			drawn.every((row) =>
				row.every(
					({ copy, face }) => copy[3] >= 128 && same(copy, face),
				),
			);
		if (whole) {
			return {
				rows: cell.rows,
				drawn: drawn.map((row) => row.map(({ copy }) => copy)),
				lids,
			};
		}
	}
	return null;
}

/**
 * Un momento de los ojos: `look` de -1 (todas las pupilas a la izquierda) a 1 (a la
 * derecha), 0 como están dibujadas; `lid`, la parte de su alto que cubre el párpado.
 */
export interface EyesPose {
	look: number;
	lid: number;
}

/** Un rectángulo que se pinta sobre la cara, en sus píxeles; fracciones a medio camino. */
export interface EyePatch {
	x: number;
	y: number;
	w: number;
	h: number;
	colour: Rgba;
}

/**
 * Los ojos en un momento, como rectángulos que se pintan en orden sobre la cara: el blanco
 * de dos píxeles, la pupila de uno que se desliza por él, como mucho un píxel, y el párpado
 * que baja desde arriba. Como los mueve el paquete.
 */
export function eyePatchesOf(
	eyes: LiveEyes,
	{ look, lid }: EyesPose,
): EyePatch[] {
	const aside = Math.max(-1, Math.min(1, look));
	const patches: EyePatch[] = [];
	eyes.rows.forEach((row, j) => {
		const [white1, iris2, iris5, white6] = eyes.drawn[j]!;
		patches.push(
			{ x: 1, y: row, w: 2, h: 1, colour: white1! },
			{ x: 2 + Math.min(0, aside), y: row, w: 1, h: 1, colour: iris2! },
			{ x: 5, y: row, w: 2, h: 1, colour: white6! },
			{ x: 5 + Math.max(0, aside), y: row, w: 1, h: 1, colour: iris5! },
		);
	});
	const shut = Math.max(0, Math.min(1, lid)) * eyes.rows.length;
	if (shut > 0) {
		COLUMNS.forEach((column, i) =>
			patches.push({
				x: column,
				y: eyes.rows[0]!,
				w: 1,
				h: shut,
				colour: eyes.lids[i]!,
			}),
		);
	}
	return patches;
}

/** Cuántas partes de un píxel recorre la pupila: octavos. */
export const EYE_STEPS = 8;

export const EYES_AT_REST: EyesPose = { look: 0, lid: 0 };

// Los tiempos de las tarjetas de mosaicary.com, en milisegundos: más lentos que los del
// paquete, porque de cerca su velocidad parece un salto.
const BLINK = { go: 100, stay: 60, back: 150, again: 130 };
const GLANCE = { go: 220, stay: 1100, across: 320, back: 260 };
/** Cuánto bajan los párpados al mirar a un lado, en píxeles. */
const GLANCE_LID = 0.3;
/** Pausa entre un movimiento y el siguiente: lo mínimo y cuánto más como mucho. */
export const EYE_GAP = { least: 2200, more: 3600 };

const eased = (k: number) => (k < 0.5 ? 2 * k * k : 1 - (-2 * k + 2) ** 2 / 2);

/** Un momento de un movimiento: cuándo, desde su inicio, y dónde están los ojos. */
export interface EyeKey extends EyesPose {
	at: number;
}

/** Un movimiento al azar: un parpadeo, a veces dos, o una mirada a un lado y vuelta. */
export function moveOf(
	rows: number,
	random: () => number = Math.random,
): EyeKey[] {
	const keys: EyeKey[] = [{ at: 0, ...EYES_AT_REST }];
	const to = (after: number, look: number, lid: number) =>
		keys.push({ at: keys[keys.length - 1]!.at + after, look, lid });
	const blink = () => {
		to(BLINK.go, 0, 1);
		to(BLINK.stay, 0, 1);
		to(BLINK.back, 0, 0);
	};
	const roll = random();
	if (roll < 0.5) blink();
	else if (roll < 0.62) {
		blink();
		to(BLINK.again, 0, 0);
		blink();
	} else {
		const side = random() < 0.5 ? -1 : 1;
		const lid = GLANCE_LID / rows;
		to(GLANCE.go, side, lid);
		to(GLANCE.stay * (0.8 + random() * 0.5), side, lid);
		if (random() < 0.4) {
			to(GLANCE.across, -side, lid);
			to(GLANCE.stay * (0.6 + random() * 0.4), -side, lid);
		}
		to(GLANCE.back, 0, 0);
	}
	return keys;
}

/** Dónde están los ojos a un tiempo dentro de un movimiento. */
export function poseAt(keys: EyeKey[], time: number): EyesPose {
	for (let i = 1; i < keys.length; i++) {
		const [a, b] = [keys[i - 1]!, keys[i]!];
		if (time >= b.at) continue;
		const k = eased((time - a.at) / (b.at - a.at));
		return {
			look: a.look + (b.look - a.look) * k,
			lid: a.lid + (b.lid - a.lid) * k,
		};
	}
	const last = keys[keys.length - 1]!;
	return { look: last.look, lid: last.lid };
}

/** Igual para dos momentos que se dibujarían igual, a `EYE_STEPS` por píxel. */
export function poseKeyOf(pose: EyesPose, rows: number): string {
	return `${Math.round(pose.look * EYE_STEPS)} ${Math.round(pose.lid * rows * EYE_STEPS)}`;
}
