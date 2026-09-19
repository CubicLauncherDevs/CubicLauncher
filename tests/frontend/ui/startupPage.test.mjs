import { expect, test } from "bun:test";
import { compile, compileModule } from "svelte/compiler";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const root = resolve(import.meta.dir, "../../..");
const page = join(root, "src/routes/+page.svelte");
const browserPath = [
	"google-chrome",
	"google-chrome-stable",
	"chromium",
	"firefox",
]
	.map((name) => Bun.which(name))
	.find(Boolean);

// Mount the real page and panel containers. Native calls and leaf panels are
// instrumented so imports/mounts can be counted separately from visibility.
const common = `
export const calls={};
export const count=key=>{calls[key]=(calls[key]??0)+1};
export const hooks={settings:null};
export const launcherStore=$state({loadedInstances:[],settings:{language:'en-US',theme:'test',show_tutorial:false,license_accepted:true,discord_presence:true,auto_updates:true,interface_preferences:{scale:100,density:'theme'}}});
export const syncSettings=async()=>{count('settings');await hooks.settings};
export const getVersions=async()=>count('instances');
export const loadInstalledVersions=async()=>count('installed');
export const initEventListeners=mode=>count(mode);
export const destroyEventListeners=()=>count('destroy');
export const initDiscordPresence=()=>count('discord');
export const autoUpdate=()=>count('updates');
export const getCurrentWebview=()=>({onDragDropEvent:async()=>{count('drag');return ()=>count('unlistenDrag')}});
export const applyTheme=()=>count('theme');
export const applyInterfaceScale=async()=>count('scale');
export const applyInterfaceDensity=()=>{};
export const interfacePreferences=value=>value;
export const animDuration=normal=>hooks.animate?normal:0;
export const shouldReduceAnimations=()=>true;
export const animateHeight=()=>({destroy(){}});
export const t=key=>key;
export const showError=(...args)=>{throw Error(args.join(':'))};
export const showSuccess=()=>{};
export const importThemeZip=async()=>{};
export const import_theme_cbth=async()=>{};
export const saveSettings=async()=>{};
export const getLoaderLogo=()=>'';
export const getDisplayIconSrc=()=>'';
export const subscribeLogPreview=()=>()=>{};
export const invoke=async()=>{};
`;

const entry = `
import {mount,unmount,flushSync,tick} from 'svelte';
import Page from ${JSON.stringify(page)};
import Header from ${JSON.stringify(join(root, "src/lib/components/instances/InstanceView/InstanceHeader.svelte"))};
import {calls,count,hooks,launcherStore} from 'startup-common';
const assert=(ok,message)=>{if(!ok)throw Error(message)};
const settle=async()=>{flushSync();await tick();await new Promise(r=>setTimeout(r,30));flushSync();await tick()};
const click=async selector=>{const el=document.querySelector(selector);assert(el,'missing '+selector);el.click();await settle()};
const reset=()=>{for(const key of Object.keys(calls))delete calls[key]};
async function run(){
 let app=mount(Page,{target:document.querySelector('main')});
 await settle();
 assert(calls.main===1 && calls.instances===1 && calls.installed===1,'main startup missing');
 assert(!calls.settingsImport && !calls.downloaderImport && !calls.catalog,'closed panels did work');
 await click('#open-settings');
 assert(calls.settingsImport===1 && calls.settingsMount===1,'settings did not lazy load');
 await click('#close-settings');
 await click('#open-settings');
 assert(calls.settingsImport===1,'settings imported twice');
 await click('#close-settings');
 await click('#open-downloader');
 assert(calls.downloaderImport===1 && calls.catalog===1,'downloader did not lazy load');
 await click('#close-downloader');
 await click('#open-downloader');
 assert(calls.catalog===1,'reopening downloader repeated startup');
 hooks.startModpack('My pack');
 launcherStore.loadedInstances.push({uuid:'other',name:'Other'});
 await settle();
 assert(!document.querySelector('#selected-instance'),'unrelated instance was selected');
 launcherStore.loadedInstances.push({uuid:'pack',name:'My pack'});
 await settle();
 assert(document.querySelector('#selected-instance')?.textContent==='pack','new modpack was not selected while installing');
 hooks.startModpack('Failed pack');
 hooks.failModpack('Failed pack');
 launcherStore.loadedInstances.push({uuid:'failed',name:'Failed pack'});
 await settle();
 assert(document.querySelector('#selected-instance')?.textContent==='pack','failed install left a pending selection');
 hooks.startModpack('Older pack');
 hooks.startModpack('Latest pack');
 hooks.failModpack('Older pack');
 launcherStore.loadedInstances.push({uuid:'latest',name:'Latest pack'});
 await settle();
 assert(document.querySelector('#selected-instance')?.textContent==='latest','earlier failure cancelled the latest install');
 await unmount(app);
 assert(calls.unlistenDrag===1,'drag listener not released');
 document.documentElement.style.setProperty('--border-radius-sm','6px');
 const instance={uuid:'pack',name:'My pack',loader:'Fabric',version:'1.21',status:'off',last_played:0,path:'/pack'};
 hooks.animate=true;
 let headerProps=$state({instance,bannerState:'Idle',activeSection:'detalles',downloadKind:null,downloadProgress:0,onPlay:()=>count('play')});
 app=mount(Header,{target:document.querySelector('main'),props:headerProps});
 await settle();
 const originalButton=document.querySelector('.launch-btn');
 for(const [kind,progress] of [['mods',0],['mods',37],['mods',100],['version',62]]){
  const previousWidth=originalButton.getBoundingClientRect().width;
  headerProps.downloadKind=kind;
  headerProps.downloadProgress=progress;
  await settle();
  const buttons=[...document.querySelectorAll('.launch-btn')];
  assert(buttons.length===2,'missing expanded or compact download button');
  assert(buttons[0]===originalButton,'download changes replaced the animated button');
  if(kind==='mods' && progress===0 && !matchMedia('(prefers-reduced-motion: reduce)').matches){
   assert(originalButton.getAnimations().length>0,'button width did not animate');
   assert(originalButton.getBoundingClientRect().width>=previousWidth,'button expanded in the wrong direction');
  }
  for(const button of buttons){
   assert(button.disabled,'download button allowed launching');
   assert(button.textContent.includes(kind==='mods'?'instanceView.downloadingMods':'instanceView.downloadingVersion'),'wrong download phase');
   assert(button.textContent.includes(progress+'%'),'download percentage missing');
   const border=button.querySelector('rect');
   assert(border?.getAttribute('stroke-dasharray')===progress+' 100','border does not match progress');
   assert(border.getTotalLength()>0,'download border has no geometry');
   button.click();
  }
  assert(!calls.play,'disabled button launched instance');
  await new Promise(r=>setTimeout(r,250));
 }
 headerProps.downloadKind=null;
 await settle();
 assert(document.querySelector('.launch-btn')===originalButton,'completion replaced the button');
 assert(!originalButton.disabled && !originalButton.querySelector('rect'),'completed download did not restore play');
 originalButton.click();
 await settle();
 assert(calls.play===1,'restored play button did not launch: '+calls.play);
 await unmount(app);
 hooks.animate=false;
 reset();
 history.replaceState(null,'','/?log=test&name=Test');
 localStorage.setItem('sidebarMode','compact');
 app=mount(Page,{target:document.querySelector('main')});
 await settle();
 assert(calls.logs===1 && calls.settings===1 && calls.logMount===1,'log startup missing');
 assert(calls.theme>=1 && calls.scale>=1,'log appearance missing');
 assert(!calls.instances && !calls.installed && !calls.discord && !calls.drag && !calls.catalog,'logs initialized main-only work');
 assert(localStorage.getItem('sidebarMode')==='compact','logs changed sidebar preference');
 await new Promise(r=>setTimeout(r,2100));
 assert(!calls.updates,'logs scheduled an update');
 await unmount(app);
 reset();
 history.replaceState(null,'','/');
 let release;
 hooks.settings=new Promise(r=>release=r);
 app=mount(Page,{target:document.querySelector('main')});
 await settle();
 await unmount(app);
 release();
 await settle();
 assert(!calls.discord && !calls.drag,'destroyed page continued startup');
}
run().then(()=>fetch('/result',{method:'POST',body:JSON.stringify({ok:true})})).catch(error=>fetch('/result',{method:'POST',body:JSON.stringify({error:String(error),stack:error.stack})}));
`;

test.skipIf(!browserPath)(
	"page handles startup, modpack navigation and animated download buttons",
	async () => {
		const bundle = await Bun.build({
			entrypoints: ["startup-entry"],
			target: "browser",
			conditions: ["browser"],
			plugins: [
				{
					name: "startup-page",
					setup(build) {
						build.onResolve(
							{ filter: /^startup-(entry|common)$/ },
							({ path }) => ({ path, namespace: "fixture" }),
						);
						build.onResolve(
							{ filter: /.*/ },
							({ path, importer }) => {
								if (path === "$lib/utils/animateWidth")
									return {
										path: join(
											root,
											"src/lib/utils/animateWidth.ts",
										),
									};
								if (path === "$lib/icons/logos")
									return {
										path: "startup-common",
										namespace: "fixture",
									};
								if (path.endsWith(".css"))
									return {
										path: "empty",
										namespace: "fixture",
									};
								if (
									path.startsWith("$lib/components/") ||
									path.startsWith("$lib/icons/")
								) {
									if (
										path.endsWith("/Drawer.svelte") ||
										path.endsWith("/ModalBase.svelte")
									)
										return {
											path: join(
												root,
												path.replace("$lib", "src/lib"),
											),
										};
									return { path, namespace: "component" };
								}
								if (
									path.startsWith("$lib/") ||
									path.startsWith("@tauri-apps/") ||
									(importer.endsWith("Drawer.svelte") &&
										path.includes("animations"))
								)
									return {
										path: "startup-common",
										namespace: "fixture",
									};
							},
						);
						build.onLoad(
							{ filter: /.*/, namespace: "fixture" },
							({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents:
									path === "startup-entry"
										? compileModule(entry, {
												filename: "entry.svelte.js",
												generate: "client",
											}).js.code
										: path === "startup-common"
											? compileModule(common, {
													filename:
														"common.svelte.js",
													generate: "client",
												}).js.code
											: "",
							}),
						);
						build.onLoad(
							{ filter: /.*/, namespace: "component" },
							({ path }) => {
								let source = "";
								if (/Sidebar(Compact)?\.svelte$/.test(path))
									source = `<script>let {onopenquickmenu,onopenversiondownloader}=$props()</script><button id="open-settings" onclick={onopenquickmenu}>Settings</button><button id="open-downloader" onclick={onopenversiondownloader}>Versions</button>`;
								if (path.endsWith("/Settings.svelte"))
									source = `<script module>import {count} from 'startup-common';count('settingsImport')</script><script>import {onMount} from 'svelte';let {onclose}=$props();onMount(()=>{count('settingsMount')})</script><button id="close-settings" onclick={onclose}>Close</button>`;
								if (path.endsWith("/VersionDownloader.svelte"))
									source = `<script module>import {count} from 'startup-common';count('downloaderImport')</script><script>import {onMount} from 'svelte';let {open=$bindable(false)}=$props();onMount(()=>{count('catalog')})</script>{#if open}<button id="close-downloader" onclick={()=>open=false}>Close</button>{/if}`;
								if (path.endsWith("/LogWindow.svelte"))
									source = `<script>import {onMount} from 'svelte';import {count} from 'startup-common';onMount(()=>{count('logMount')})</script>`;
								if (
									path.endsWith("/CreateInstanceModal.svelte")
								)
									source = `<script>import {onMount} from 'svelte';import {hooks} from 'startup-common';let {oninstallstarted,oninstallfailed}=$props();onMount(()=>{hooks.startModpack=oninstallstarted;hooks.failModpack=oninstallfailed})</script>`;
								if (path.endsWith("/InstanceView.svelte"))
									source = `<script>let {selectedInstance}=$props()</script><div id="selected-instance">{selectedInstance.uuid}</div>`;
								return {
									loader: "js",
									resolveDir: root,
									contents: compile(source, {
										filename: path,
										generate: "client",
										css: "injected",
									}).js.code,
								};
							},
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
					},
				},
			],
		});
		expect(bundle.success, bundle.logs.join("\n")).toBe(true);
		const js = await bundle.outputs[0].text();
		let server, browser, watchdog;
		const profile = await mkdtemp(join(tmpdir(), "startup-page-"));
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
					return new Response(
						'<!doctype html><meta charset="utf-8"><main></main><script type="module" src="/script.js"></script>',
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
			watchdog = setTimeout(() => browser.kill("SIGKILL"), 30000);
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
	45000,
);
