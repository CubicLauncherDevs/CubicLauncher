/**
 * Detecta si una skin fue dibujada para el modelo clásico (Steve, brazos de
 * 4 px) o delgado (Alex, brazos de 3 px), mirando la propia imagen.
 *
 * El modelo delgado deja sin usar dos columnas en la cara de atrás de cada
 * brazo y dos en la parte de abajo de la mano. Si una skin delgada se sube como
 * clásica, el juego lee esas columnas vacías y, como dibuja opaca la capa base,
 * aparece una franja (casi siempre negra) en la parte de atrás del brazo.
 *
 * Solo se decide con pruebas en la imagen. Si las caras que distinguen los dos
 * modelos están vacías (brazo sin dibujar, transparente o imagen incompleta), se
 * devuelve `null` y quien llama conserva el modelo elegido.
 */

export type SkinModel = "classic" | "slim";

/**
 * Decide si se puede usar un modelo con la skin detectada.
 *
 * Una skin delgada no puede pasar a clásico: el juego lee las columnas que la
 * skin delgada no dibuja y, como la capa base es opaca, aparecen franjas
 * (casi siempre negras) en la parte de atrás de los brazos. Al revés no pasa
 * nada: una skin clásica guardada como delgada solo se ve con brazos más finos.
 *
 * Si no hay datos suficientes para detectar el modelo (`null`), se deja elegir
 * al usuario.
 */
export function canUseSkinModel(
	detected: SkinModel | null,
	target: SkinModel,
): boolean {
	return !(detected === "slim" && target === "classic");
}

interface Zone {
	x: number;
	y: number;
	w: number;
	h: number;
}

/**
 * Una cara del brazo cuyo ancho cambia entre modelos: la parte que ambos dibujan
 * y las columnas que solo existen en el clásico.
 */
interface ArmFace {
	shared: Zone;
	classicOnly: Zone;
}

/** Por debajo de este alfa un píxel cuenta como vacío: el antialiasing deja restos. */
const CLEAR_ALPHA = 8;

/** Cara de atrás y parte de abajo de cada brazo, en píxeles de una skin de 64×64. */
const ARM_FACES: readonly ArmFace[] = [
	// brazo derecho, cara de atrás
	{
		shared: { x: 52, y: 20, w: 2, h: 12 },
		classicOnly: { x: 54, y: 20, w: 2, h: 12 },
	},
	// brazo derecho, parte de abajo
	{
		shared: { x: 48, y: 16, w: 2, h: 4 },
		classicOnly: { x: 50, y: 16, w: 2, h: 4 },
	},
	// brazo izquierdo, cara de atrás
	{
		shared: { x: 44, y: 52, w: 2, h: 12 },
		classicOnly: { x: 46, y: 52, w: 2, h: 12 },
	},
	// brazo izquierdo, parte de abajo
	{
		shared: { x: 40, y: 48, w: 2, h: 4 },
		classicOnly: { x: 42, y: 48, w: 2, h: 4 },
	},
];

/**
 * @param rgba píxeles RGBA de la imagen completa, fila por fila
 * @param width ancho real de la imagen (64, 128, ... para skins HD)
 * @param height alto real de la imagen
 */
export function detectSkinModel(
	rgba: ArrayLike<number>,
	width: number,
	height: number,
): SkinModel | null {
	const scale = Math.max(1, Math.round(width / 64));
	// El formato de 64×32 (anterior a 1.8) solo tiene brazos de 4 px.
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

	// Algo dibujado donde el delgado no llega solo puede ser de un brazo clásico.
	if (ARM_FACES.some((face) => hasPixels(face.classicOnly))) return "classic";
	// Una cara dibujada que termina antes de esas columnas es de un brazo delgado.
	if (ARM_FACES.some((face) => hasPixels(face.shared))) return "slim";
	return null;
}

/**
 * Igual que {@link detectSkinModel}, a partir de una URL que el canvas pueda leer
 * (por ejemplo, el data URL de la vista previa). Devuelve `null` si la imagen no
 * se puede leer o no alcanza para decidir, para que quien llama conserve el
 * modelo elegido.
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
		// Sin CORS el canvas queda contaminado y getImageData falla: hace falta
		// para leer skins remotas (textures.minecraft.net las sirve con
		// Access-Control-Allow-Origin).
		image.crossOrigin = "anonymous";
		image.onload = () => resolve(image);
		image.onerror = () => reject(new Error("No se pudo leer la skin"));
		image.src = url;
	});
}
