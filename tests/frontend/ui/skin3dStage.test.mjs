import { expect, test } from "bun:test";
import {
	avoidTextureUnitZero,
	isWebKitGtk,
} from "../../../src/lib/utils/skin3dStage.ts";

const TEXTURE0 = 33984;
const SAMPLER_2D = 35678;
const INT = 5124;
const SPARE = 31;

function fakeGl() {
	const calls = { active: [], one: [], many: [] };
	const uniforms = [
		{ name: "map", type: SAMPLER_2D },
		{ name: "maps[0]", type: SAMPLER_2D },
		{ name: "count", type: INT },
	];
	const gl = {
		TEXTURE0,
		SAMPLER_2D,
		SAMPLER_CUBE: 35680,
		SAMPLER_3D: 35679,
		SAMPLER_2D_ARRAY: 36289,
		SAMPLER_2D_SHADOW: 35682,
		SAMPLER_CUBE_SHADOW: 36293,
		SAMPLER_2D_ARRAY_SHADOW: 36292,
		MAX_TEXTURE_IMAGE_UNITS: 34930,
		ACTIVE_UNIFORMS: 35718,
		getParameter: () => SPARE + 1,
		getProgramParameter: () => uniforms.length,
		getActiveUniform: (_program, at) => uniforms[at],
		getUniformLocation: (_program, name) => ({ name }),
		activeTexture: (unit) => calls.active.push(unit),
		uniform1i: (at, value) => calls.one.push(value),
		uniform1iv: (at, values) => calls.many.push([...values]),
	};
	avoidTextureUnitZero(gl);
	return { gl, calls, program: {} };
}

test("lo que se pide para la unidad 0 va a la última", () => {
	const { gl, calls } = fakeGl();
	gl.activeTexture(TEXTURE0);
	expect(calls.active).toEqual([TEXTURE0 + SPARE]);
});

test("las demás unidades no cambian", () => {
	const { gl, calls } = fakeGl();
	gl.activeTexture(TEXTURE0 + 1);
	gl.activeTexture(TEXTURE0 + 63);
	expect(calls.active).toEqual([TEXTURE0 + 1, TEXTURE0 + 63]);
});

test("un sampler en la unidad 0 lee de la última", () => {
	const { gl, calls, program } = fakeGl();
	gl.uniform1i(gl.getUniformLocation(program, "map"), 0);
	expect(calls.one).toEqual([SPARE]);
});

test("un sampler en otra unidad no cambia", () => {
	const { gl, calls, program } = fakeGl();
	gl.uniform1i(gl.getUniformLocation(program, "map"), 1);
	expect(calls.one).toEqual([1]);
});

test("un entero que vale 0 sigue valiendo 0", () => {
	const { gl, calls, program } = fakeGl();
	gl.uniform1i(gl.getUniformLocation(program, "count"), 0);
	expect(calls.one).toEqual([0]);
});

test("una lista de samplers cambia solo su unidad 0", () => {
	const { gl, calls, program } = fakeGl();
	gl.uniform1iv(gl.getUniformLocation(program, "maps[0]"), [0, 1, 2]);
	expect(calls.many).toEqual([[SPARE, 1, 2]]);
});

test("una lista de enteros no cambia", () => {
	const { gl, calls, program } = fakeGl();
	gl.uniform1iv(gl.getUniformLocation(program, "count"), [0, 1]);
	expect(calls.many).toEqual([[0, 1]]);
});

test("el arreglo es solo para la ventana de Linux", () => {
	const webkitGtk =
		"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15";
	const webview2 =
		"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0";
	const chromeOnLinux =
		"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
	const wkWebView =
		"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";
	expect(isWebKitGtk(webkitGtk)).toBe(true);
	expect(isWebKitGtk(webview2)).toBe(false);
	expect(isWebKitGtk(chromeOnLinux)).toBe(false);
	expect(isWebKitGtk(wkWebView)).toBe(false);
});
