import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("./package.json", "utf-8"));

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
	plugins: [
		sveltekit({
			adapter: adapter({ fallback: "index.html" }),
			preprocess: [
				vitePreprocess(),
				{
					name: "cubic-theme-layer",
					// Permite que los temas sin capa sobrescriban los estilos de los componentes.
					style: ({ content }) => ({ code: `@layer cubic {\n${content}\n}` }),
				},
			],
		}),
	],

	define: {
		__APP_VERSION__: JSON.stringify(pkg.version),
	},

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent Vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
					protocol: "ws",
					host,
					port: 1421,
				}
			: undefined,
		watch: {
			// 3. tell Vite to ignore watching `src-tauri`
			ignored: ["**/src-tauri/**"],
		},
	},

	resolve: {
		alias: {
			"@static": new URL("./static", import.meta.url).pathname,
		},
	},

	build: {},
}));
