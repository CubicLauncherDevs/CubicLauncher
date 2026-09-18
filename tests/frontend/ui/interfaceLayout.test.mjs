import { expect, test } from "bun:test";
import { compile, compileModule } from "svelte/compiler";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const root = resolve(import.meta.dir, "../../..");
const browserPath = [
	"google-chrome",
	"google-chrome-stable",
	"chromium",
	"firefox",
]
	.map((name) => Bun.which(name))
	.find(Boolean);
const state = `
export const launcherStore = $state({ notifications: [], settings: { reduce_animations: true,
 interface_preferences: {scale:100,density:'theme'},
 notification_preferences: {enabled:false,position:'top-right',size:'compact',title_size:13,message_size:11,uppercase_title:false,bold_title:false,duration_seconds:5}
}});
export const errors=[];
export const showError=(...args)=>errors.push(args);
export const showInfo=()=>'';
export const removeNotification=id=>{launcherStore.notifications=launcherStore.notifications.filter(item=>item.id!==id)};
`;

test.skipIf(!browserPath)(
	"interface preferences preserve themes, virtual rows and popup bounds",
	async () => {
		const bundle = await Bun.build({
			entrypoints: [
				join(import.meta.dir, "fixtures/interfaceLayout.entry.mjs"),
			],
			target: "browser",
			conditions: ["browser"],
			plugins: [
				{
					name: "interface-layout",
					setup(build) {
						build.onResolve(
							{ filter: /^\$lib\/state\/state.svelte$/ },
							() => ({ path: "state", namespace: "fixture" }),
						);
						build.onResolve({ filter: /^\$lib\/i18n$/ }, () => ({
							path: "i18n",
							namespace: "fixture",
						}));
						build.onResolve({ filter: /^\$lib\// }, ({ path }) => ({
							path: Bun.resolveSync(
								path.replace("$lib", join(root, "src/lib")),
								root,
							),
						}));
						build.onLoad(
							{ filter: /.*/, namespace: "fixture" },
							async ({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents:
									path === "state"
										? compileModule(state, {
												filename: "state.svelte.js",
												generate: "client",
											}).js.code
										: `const strings=${await readFile(join(root, "src/lib/i18n/en-US.json"), "utf8")}; export const t=key=>key.split('.').reduce((value,part)=>value?.[part],strings)??key;`,
							}),
						);
						build.onLoad(
							{ filter: /\.svelte$/ },
							async ({ path }) => ({
								loader: "js",
								contents: compile(
									await readFile(path, "utf8"),
									{
										filename: path,
										generate: "client",
										css: "injected",
									},
								).js.code,
							}),
						);
						build.onLoad(
							{ filter: /\.svelte\.ts$/ },
							async ({ path }) => ({
								loader: "js",
								contents: compileModule(
									new Bun.Transpiler({
										loader: "ts",
									}).transformSync(
										await readFile(path, "utf8"),
									),
									{ filename: path, generate: "client" },
								).js.code,
							}),
						);
						build.onLoad(
							{ filter: /\.css$/ },
							async ({ path }) => ({
								loader: "js",
								contents: `const style=document.createElement('style');style.textContent=${JSON.stringify(await readFile(path, "utf8"))};document.head.appendChild(style);`,
							}),
						);
					},
				},
			],
		});
		expect(bundle.success, bundle.logs.join("\n")).toBe(true);
		const js = await bundle.outputs[0].text();
		const css = (
			await Promise.all(
				["reset", "components", "interface", "modal", "perf"].map(
					(name) =>
						readFile(
							join(root, `src/styles/shared/${name}.css`),
							"utf8",
						),
				),
			)
		).join("\n");
		let server, browser, watchdog;
		const profile = await mkdtemp(join(tmpdir(), "interface-layout-"));
		try {
			let report;
			const result = new Promise((resolve) => {
				report = resolve;
			});
			server = Bun.serve({
				hostname: "127.0.0.1",
				port: 0,
				async fetch(request) {
					const path = new URL(request.url).pathname;
					if (path === "/result") {
						report(await request.json());
						return new Response(null, { status: 204 });
					}
					if (path === "/script.js")
						return new Response(js, {
							headers: {
								"Content-Type": "application/javascript",
							},
						});
					if (
						path.startsWith("/themes/") ||
						path.startsWith("/images/") ||
						path.startsWith("/fonts/")
					) {
						const file = Bun.file(join(root, "static", path));
						return (await file.exists())
							? new Response(file)
							: new Response(null, { status: 404 });
					}
					if (path === "/fixture")
						return new Response(
							`<!doctype html><meta charset="utf-8"><style>@layer cubic {${css}}</style><main></main><script type="module" src="/script.js"></script>`,
							{ headers: { "Content-Type": "text/html" } },
						);
					return new Response(
						'<!doctype html><iframe src="/fixture" style="width:800px;height:600px;border:0"></iframe>',
						{ headers: { "Content-Type": "text/html" } },
					);
				},
			});
			const url = `http://127.0.0.1:${server.port}`;
			browser = Bun.spawn(
				browserPath.includes("firefox")
					? [
							browserPath,
							"--headless",
							"--no-remote",
							"--profile",
							profile,
							url,
						]
					: [
							browserPath,
							"--headless",
							"--no-sandbox",
							"--disable-gpu",
							"--disable-dev-shm-usage",
							"--no-first-run",
							`--user-data-dir=${profile}`,
							url,
						],
				{ stdout: "ignore", stderr: "pipe" },
			);
			const stderr = new Response(browser.stderr).text();
			watchdog = setTimeout(() => browser.kill("SIGKILL"), 45000);
			const payload = await Promise.race([
				result,
				browser.exited.then(async (code) => {
					throw Error(`Browser exited (${code}): ${await stderr}`);
				}),
			]);
			expect(payload.error, payload.stack).toBeUndefined();
			expect(payload.cases).toBe(72);
		} finally {
			clearTimeout(watchdog);
			if (browser) {
				browser.kill();
				await browser.exited;
			}
			server?.stop(true);
			await rm(profile, { recursive: true, force: true });
		}
	},
	60000,
);
