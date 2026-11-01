/**
 * Las imágenes del armario: una sola escena 3D las dibuja todas por turno y se libera al acabar.
 * Una tarjeta no gira ni se anima, así que no necesita un contexto WebGL propio.
 */

import { createStage, releaseStage } from "$lib/utils/skin3dStage";
import {
	createThumbnailQueue,
	type ThumbnailPainter,
} from "$lib/utils/thumbnailQueue";

/** Doble de la altura de la tarjeta: al reducirla, el navegador suaviza los bordes. */
const THUMBNAIL_SIZE = 280;

async function makeStagePainter(): Promise<ThumbnailPainter> {
	const { Render } = await import("skin3d");
	const stage = createStage({
		Render,
		width: THUMBNAIL_SIZE,
		height: THUMBNAIL_SIZE,
		antialias: false,
		pixelRatio: 1,
		maxPixelRatio: 1,
	});
	return {
		async paint(url, model) {
			const separator = url.includes("?") ? "&" : "?";
			await stage.loadSkin(
				url.startsWith("data:") ? url : `${url}${separator}_skin3d=1`,
				{ model },
			);
			stage.render();
			return new Promise<Blob>((resolve, reject) => {
				stage.canvas.toBlob((blob) => {
					if (blob) resolve(blob);
					else reject(new Error("No se pudo dibujar la skin"));
				});
			});
		},
		release() {
			releaseStage(stage);
		},
	};
}

export const requestThumbnail = createThumbnailQueue(makeStagePainter).request;
