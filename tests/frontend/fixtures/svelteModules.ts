import { readFile } from "node:fs/promises";
import { compileModule } from "svelte/compiler";

// Minimal shapes of the Bun runtime used by the fixtures. `bun` is not a
// dependency of the launcher, so its type declarations are not available to
// `svelte-check`; importing it dynamically keeps these files type-checkable.
type OnLoadArgs = { path: string };
type OnLoadResult = { loader: string; contents: string } | undefined;
type Build = {
	onLoad: (
		filter: { filter: RegExp },
		callback: (args: OnLoadArgs) => Promise<OnLoadResult>,
	) => void;
};
export type BunPlugin = {
	name: string;
	setup: (build: Build) => void;
};
type BunRuntime = {
	Transpiler: new (options: { loader: string; target: string }) => {
		transformSync: (source: string) => string;
	};
};

// `bun` is not a dependency, so it is resolved at runtime and never type-checked.
const importBun = new Function(
	'return import("bun");',
) as () => Promise<unknown>;

export async function svelteModules(): Promise<BunPlugin> {
	const bun = (await importBun().catch(() => undefined)) as
		BunRuntime | undefined;
	if (!bun) throw new Error("Las fixtures de runas requieren Bun.");
	const transpiler = new bun.Transpiler({
		loader: "ts",
		target: "browser",
	});

	// Bun copies runes verbatim and `bun test` does not run module plugins, so
	// every `.svelte.ts` a fixture evaluates has to be compiled explicitly.
	// Otherwise the runtime throws `rune_outside_svelte` or `$state is not
	// defined`.
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
