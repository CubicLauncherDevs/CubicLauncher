import { plugin, Transpiler, type BunPlugin } from "bun";
import { readFile } from "node:fs/promises";
import { compileModule } from "svelte/compiler";

const transpiler = new Transpiler({ loader: "ts", target: "browser" });

// Bun copies runes verbatim and `bun test` does not run module plugins, so every
// `.svelte.ts` a fixture evaluates has to be compiled explicitly. Otherwise the
// runtime throws `rune_outside_svelte` or `$state is not defined`.
export function svelteModules(): BunPlugin {
	return {
		name: "svelte-modules",
		setup(build) {
			build.onLoad({ filter: /\.svelte\.ts$/ }, async ({ path }) => {
				const { js } = compileModule(
					transpiler.transformSync(await readFile(path, "utf8")),
					{ filename: path, generate: "client" },
				);
				return { loader: "js", contents: js.code };
			});
		},
	};
}

// Module-level plugin for tests that import the real runes modules directly
// instead of bundling them.
plugin(svelteModules());
