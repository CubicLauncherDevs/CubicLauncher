import { expect, test } from "bun:test";
import { compile, compileModule } from "svelte/compiler";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const root = resolve(import.meta.dir, "../../..");
const browserPath = ["google-chrome", "chromium", "firefox"]
	.map((name) => Bun.which(name))
	.find(Boolean);

test.skipIf(!browserPath)(
	"tutorial and profile account setup preserve layout, selection and keyboard navigation",
	async () => {
		const strings = await readFile(
			join(root, "src/lib/i18n/en-US.json"),
			"utf8",
		);
		const mocks = {
			"$lib/state/state.svelte": compileModule(
				`export const launcherStore = $state({settings: {user:[],active_user_idx:0,license_accepted:false,show_tutorial:true,language:'en-US',reduce_animations:true}}); export const showError=()=>{};`,
				{ filename: "state.svelte.js", generate: "client" },
			).js.code,
			"$lib/i18n": `const strings=${strings}; export const t=(key,params={})=>Object.entries(params).reduce((text,[key,value])=>text.replaceAll('{'+key+'}',String(value)),key.split('.').reduce((v,k)=>v?.[k],strings)??key); export const locales=[]; export const downloadLocale=async()=>{}; export const loadAvailableLocales=async()=>{};`,
			"$lib/api/launcherService":
				"export const saves=[]; export const hooks={failSave:false}; export const markLocalSettingsChange=()=>{}; export const saveSettings=async()=>{if(hooks.failSave) throw Error('Save failed'); saves.push(true);};",
			"$lib/state/avatarCache.svelte": `export const DEFAULT_AVATAR_SVG='<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="gray"/></svg>'; export const fetchAvatarSvg=async()=>DEFAULT_AVATAR_SVG;`,
			"profile-skin-stub": compile("<div></div>", {
				filename: "SkinStub.svelte",
				generate: "client",
			}).js.code,
			"$lib/api/cubicApi": `
				export const calls=[];
				export const openUrl=async()=>{};
				export const logout=async()=>{};
				export const switchUser=async()=>null;
				export const removeUser=async()=>{};
				export const startWebviewAuth=async()=>{calls.push('microsoft');return {username:'PremiumPlayer',uuid:'premium-id',user_type:'Microsoft',access_token:'test',refresh_token:null};};
				export const getYggdrasilServerInfo=async url=>{calls.push(url);return {server_name:'Test server',non_email_login:true,skin_domains:[]};};
				export const yggdrasilAuthenticate=async (url,username)=>({username,uuid:url,user_type:'Yggdrasil',yggdrasil_server_url:url,access_token:'test',refresh_token:null});
			`,
		};
		const bundle = await Bun.build({
			entrypoints: [
				join(import.meta.dir, "fixtures/tutorialLayout.entry.mjs"),
			],
			target: "browser",
			conditions: ["browser"],
			plugins: [
				{
					name: "tutorial-layout",
					setup(build) {
						build.onResolve(
							{
								filter: /^\.\/(SkinCapeManager|ElySkinManager)\.svelte$/,
							},
							() => ({
								path: "profile-skin-stub",
								namespace: "fixture",
							}),
						);
						build.onResolve({ filter: /^\$lib\// }, ({ path }) =>
							path in mocks
								? { path, namespace: "fixture" }
								: {
										path: Bun.resolveSync(
											path.replace(
												"$lib",
												join(root, "src/lib"),
											),
											root,
										),
									},
						);
						build.onLoad(
							{ filter: /.*/, namespace: "fixture" },
							({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents: mocks[path],
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
					},
				},
			],
		});
		expect(bundle.success, bundle.logs.join("\n")).toBe(true);
		const js = await bundle.outputs[0].text();
		const css = (
			await Promise.all(
				["reset", "welcome", "components", "modal"].map((name) =>
					readFile(
						join(root, `src/styles/shared/${name}.css`),
						"utf8",
					),
				),
			)
		).join("\n");
		const profile = await mkdtemp(join(tmpdir(), "tutorial-layout-"));
		let server, browser, watchdog;
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
					if (path === "/fixture")
						return new Response(
							`<!doctype html><meta charset="utf-8"><style>@layer cubic {${css}} :root {--space-lg:16px;--space-md:12px;--space-xl:24px;--border-width:1px;--border:#333;} body {margin:0}</style><script>window.addEventListener('error',event=>fetch('/result',{method:'POST',body:JSON.stringify({error:event.message,stack:event.error?.stack})}));window.addEventListener('unhandledrejection',event=>fetch('/result',{method:'POST',body:JSON.stringify({error:String(event.reason),stack:event.reason?.stack})}));</script><script type="module" src="/script.js"></script>`,
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
			expect(payload.ok).toBe(true);
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
