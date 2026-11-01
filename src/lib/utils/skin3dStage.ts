import type { Render } from "skin3d";

/** Lo más cerca que llega la cámara al punto que mira: no entra en la figura. */
const NEAREST = 10;

/**
 * A qué altura gira y hacia dónde acerca la vista: el pecho (en skin3d el torso va de -4 a 8,
 * la figura de -16 a 16). El centro de la figura queda a la altura de la cintura.
 */
const CHEST_Y = 4;

const ZOOM = 0.7;

export interface StageOptions {
	Render: typeof Render;
	width: number;
	height: number;
	/** Suaviza los bordes con FXAA, a cambio de dibujar en un búfer aparte. */
	antialias: boolean;
	pixelRatio: number | "match-device";
	maxPixelRatio: number;
}

/**
 * Un lienzo con el contexto creado de antemano: skin3d no deja elegir sus atributos, pero
 * `getContext` devuelve el contexto que ya existe y Three lo adopta.
 */
function leanCanvas(antialias: boolean): HTMLCanvasElement {
	const canvas = document.createElement("canvas");
	canvas.getContext("webgl2", {
		alpha: true,
		// Con FXAA la figura se dibuja en el búfer del compositor y aquí solo llega la imagen.
		depth: !antialias,
		stencil: false,
		antialias: false,
		// En portátiles con dos GPU, una vista previa no despierta la dedicada.
		powerPreference: "low-power",
	});
	return canvas;
}

/** Una escena de skin3d que no dibuja por su cuenta: quien la crea decide cuándo. */
export function createStage(options: StageOptions): Render {
	const stage = new options.Render({
		canvas: leanCanvas(options.antialias),
		width: options.width,
		height: options.height,
		zoom: ZOOM,
		enableFXAA: options.antialias,
		pixelRatio: options.pixelRatio,
		maxPixelRatio: options.maxPixelRatio,
		// Con las luces de skin3d se ve igual que PBR, con un shader mucho más corto.
		materialType: "lambert",
		renderPaused: true,
	});

	// El pase de FXAA termina en pantalla: sin este intercambio, el compositor alternaría entre
	// dos búferes de color y dos de profundidad para usar siempre solo uno.
	if (stage.fxaaPass) stage.fxaaPass.needsSwap = false;
	// Sin FXAA se dibuja directamente en pantalla, y Three aclararía los colores: con FXAA esa
	// conversión no ocurre, y la figura debe verse igual con cualquier calidad.
	else stage.renderer.outputColorSpace = "srgb-linear";

	stage.controls.minDistance = NEAREST;
	// Sin desplazar con el botón derecho: el centro de giro se queda en el pecho.
	stage.controls.enablePan = false;
	// El centro de giro en el pecho; la cámara sube lo mismo, y la vista no se inclina.
	stage.controls.target.set(0, CHEST_Y, 0);
	stage.camera.position.y += CHEST_Y;
	stage.controls.update();

	return stage;
}

/**
 * Libera la escena de verdad. `dispose` de skin3d borra los objetos pero deja vivo el contexto
 * WebGL hasta que pase el recolector de basura, y Chromium no admite más de 16 a la vez.
 */
export function releaseStage(stage: Render) {
	const canvases = [stage.skinCanvas, stage.capeCanvas, stage.earsCanvas];
	stage.dispose();
	stage.composer.dispose();
	stage.renderer.forceContextLoss();
	for (const canvas of canvases) {
		canvas.width = 0;
		canvas.height = 0;
	}
}
