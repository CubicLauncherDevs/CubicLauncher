/**
 * Detecta si una skin fue dibujada para el modelo clásico (Steve, brazos de
 * 4 px) o delgado (Alex, brazos de 3 px), mirando la propia imagen.
 *
 * El modelo delgado deja sin usar dos columnas en la cara de atrás de cada
 * brazo y dos en la parte de abajo de la mano. Si una skin delgada se sube como
 * clásica, el juego lee esas columnas vacías y, como dibuja opaca la capa base,
 * aparece una franja (casi siempre negra) en la parte de atrás del brazo.
 *
 * Ante la duda se devuelve clásico: es el modelo por defecto del juego y el que
 * no deja franjas vacías en una skin que no las tenía.
 */

export type SkinModel = "classic" | "slim";

interface Zone {
	x: number;
	y: number;
	w: number;
	h: number;
}

/** Por debajo de este alfa un píxel cuenta como vacío: el antialiasing deja restos. */
const CLEAR_ALPHA = 8;

/** Zonas que el modelo delgado nunca usa, en píxeles de una skin de 64×64. */
const SLIM_UNUSED: readonly Zone[] = [
	{ x: 54, y: 20, w: 2, h: 12 }, // brazo derecho, cara de atrás
	{ x: 50, y: 16, w: 2, h: 4 }, // brazo derecho, parte de abajo
	{ x: 46, y: 52, w: 2, h: 12 }, // brazo izquierdo, cara de atrás
	{ x: 42, y: 48, w: 2, h: 4 }, // brazo izquierdo, parte de abajo
];

/** Cara de adelante del brazo derecho: si está vacía, no hay nada que mirar. */
const RIGHT_ARM_FRONT: Zone = { x: 44, y: 20, w: 4, h: 12 };

/**
 * @param rgba píxeles RGBA de la imagen completa, fila por fila
 * @param width ancho real de la imagen (64, 128, ... para skins HD)
 * @param height alto real de la imagen
 */
export function detectSkinModel(
	rgba: ArrayLike<number>,
	width: number,
	height: number,
): SkinModel {
	const scale = Math.max(1, Math.round(width / 64));
	// Las skins de 64×32 (anteriores a 1.8) no tienen modelo delgado.
	if (Math.round(height / scale) < 64) return "classic";

	const alphaAt = (x: number, y: number): number => {
		// Centro del bloque en skins HD, para que un píxel suelto en el borde no decida.
		const px = Math.min(width - 1, x * scale + (scale >> 1));
		const py = Math.min(height - 1, y * scale + (scale >> 1));
		return rgba[(py * width + px) * 4 + 3] ?? 0;
	};
	const hasPixels = (zone: Zone): boolean => {
		for (let y = zone.y; y < zone.y + zone.h; y++) {
			for (let x = zone.x; x < zone.x + zone.w; x++) {
				if (alphaAt(x, y) >= CLEAR_ALPHA) return true;
			}
		}
		return false;
	};

	if (!hasPixels(RIGHT_ARM_FRONT)) return "classic";
	return SLIM_UNUSED.some(hasPixels) ? "classic" : "slim";
}

/**
 * Igual que {@link detectSkinModel}, a partir de una URL que el canvas pueda leer
 * (por ejemplo, el data URL de la vista previa). Devuelve `null` si la imagen no
 * se puede leer, para que quien llama conserve el modelo elegido.
 */
export async function detectSkinModelFromUrl(
	url: string,
): Promise<SkinModel | null> {
	try {
		const image = await loadImage(url);
		const canvas = document.createElement("canvas");
		canvas.width = image.naturalWidth;
		canvas.height = image.naturalHeight;
		const context = canvas.getContext("2d", { willReadFrequently: true });
		if (!context) return null;
		context.drawImage(image, 0, 0);
		const { data } = context.getImageData(
			0,
			0,
			canvas.width,
			canvas.height,
		);
		return detectSkinModel(data, canvas.width, canvas.height);
	} catch {
		return null;
	}
}

function loadImage(url: string): Promise<HTMLImageElement> {
	return new Promise((resolve, reject) => {
		const image = new Image();
		image.onload = () => resolve(image);
		image.onerror = () => reject(new Error("No se pudo leer la skin"));
		image.src = url;
	});
}
