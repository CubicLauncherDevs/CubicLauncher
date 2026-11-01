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

/** En Linux la ventana usa WebKitGTK; en Windows, WebView2, que se presenta como Chrome. */
export function isWebKitGtk(userAgent: string): boolean {
	return /\bLinux\b/.test(userAgent) && !/\bChrome\//.test(userAgent);
}

function samplerTypesOf(gl: WebGL2RenderingContext): Set<number> {
	return new Set([
		gl.SAMPLER_2D,
		gl.SAMPLER_CUBE,
		gl.SAMPLER_3D,
		gl.SAMPLER_2D_ARRAY,
		gl.SAMPLER_2D_SHADOW,
		gl.SAMPLER_CUBE_SHADOW,
		gl.SAMPLER_2D_ARRAY_SHADOW,
	]);
}

/**
 * En WebKitGTK (Linux, medido con 2.54 y Mesa 26) la unidad de textura 0 puede dejar de leer para
 * todos los contextos de la página, de forma intermitente y hasta reiniciar: la figura sale vacía
 * o negra, sin ningún error de WebGL. Las demás unidades siguen bien, y Three empieza siempre por
 * la 0, así que aquí lo que pide para ella va a la última.
 */
export function avoidTextureUnitZero(gl: WebGL2RenderingContext) {
	const spare = (gl.getParameter(gl.MAX_TEXTURE_IMAGE_UNITS) as number) - 1;
	const samplerTypes = samplerTypesOf(gl);
	const samplerNames = new WeakMap<WebGLProgram, Set<string>>();
	const samplers = new WeakSet<WebGLUniformLocation>();

	const activeTexture = gl.activeTexture.bind(gl);
	const uniform1i = gl.uniform1i.bind(gl);
	const uniform1iv = gl.uniform1iv.bind(gl) as (
		at: WebGLUniformLocation | null,
		values: Iterable<number>,
	) => void;
	const getUniformLocation = gl.getUniformLocation.bind(gl);

	function namesOf(program: WebGLProgram): Set<string> {
		let names = samplerNames.get(program);
		if (names) return names;
		names = new Set();
		const count = gl.getProgramParameter(
			program,
			gl.ACTIVE_UNIFORMS,
		) as number;
		for (let at = 0; at < count; at++) {
			const info = gl.getActiveUniform(program, at);
			if (info && samplerTypes.has(info.type)) names.add(info.name);
		}
		samplerNames.set(program, names);
		return names;
	}

	gl.activeTexture = (unit) =>
		activeTexture(unit === gl.TEXTURE0 ? gl.TEXTURE0 + spare : unit);
	gl.getUniformLocation = (program, name) => {
		const at = getUniformLocation(program, name);
		if (at && namesOf(program).has(name)) samplers.add(at);
		return at;
	};
	gl.uniform1i = (at, value) =>
		uniform1i(at, at && samplers.has(at) && value === 0 ? spare : value);
	gl.uniform1iv = ((
		at: WebGLUniformLocation | null,
		values: Iterable<number>,
	) =>
		uniform1iv(
			at,
			at && samplers.has(at)
				? Array.from(values, (one) => (one === 0 ? spare : one))
				: values,
		)) as typeof gl.uniform1iv;
}

/**
 * Un lienzo con el contexto creado de antemano: skin3d no deja elegir sus atributos, pero
 * `getContext` devuelve el contexto que ya existe y Three lo adopta.
 */
function leanCanvas(antialias: boolean): HTMLCanvasElement {
	const canvas = document.createElement("canvas");
	const gl = canvas.getContext("webgl2", {
		alpha: true,
		// Con FXAA la figura se dibuja en el búfer del compositor y aquí solo llega la imagen.
		depth: !antialias,
		stencil: false,
		antialias: false,
		// En portátiles con dos GPU, una vista previa no despierta la dedicada.
		powerPreference: "low-power",
	});
	if (gl && isWebKitGtk(navigator.userAgent)) avoidTextureUnitZero(gl);
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
