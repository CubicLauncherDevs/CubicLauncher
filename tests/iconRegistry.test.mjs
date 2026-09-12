import { expect, test } from "bun:test";
import { isRasterIcon } from "../src/lib/icons/registry.ts";

for (const extension of ["png", "webp", "jpg", "jpeg", "PNG"]) {
	for (const suffix of [
		"",
		"?theme-revision=123-1",
		"#preview",
		"?theme-revision=123-1#preview",
	]) {
		test(`recognizes raster icon ${extension}${suffix}`, () => {
			expect(
				isRasterIcon(`asset://localhost/icons/play.${extension}${suffix}`),
			).toBe(true);
		});
	}
}

for (const path of [
	"/icons/play.svg?theme-revision=123-1",
	"/icons/play.svg?fallback=icon.png",
	"/icons/play.svg#icon.png",
	"/icons/play",
]) {
	test(`keeps non-raster icon ${path} as a mask`, () => {
		expect(isRasterIcon(path)).toBe(false);
	});
}
