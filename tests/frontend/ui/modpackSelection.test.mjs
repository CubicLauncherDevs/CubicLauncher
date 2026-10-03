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
const directory = join(
	root,
	"src/lib/components/instances/CreateInstanceModal",
);

const common = `
export const hooks={view:null,requests:[],downloads:[],installs:[],searches:0,catalogs:0};
export const launcherStore={loadedInstances:[]};
export const t=key=>key;
export const showErrorParsed=error=>{throw error};
export const openUrl=()=>{};
export const getAvailableVersions=async()=>{hooks.catalogs++;return []};
const request=(kind,id,signal)=>new Promise((resolve,reject)=>hooks.requests.push({kind,id,signal,resolve,reject}));
const projects=[1,2].map(id=>({id,project_id:String(id),title:'Pack '+id,name:'Pack '+id,description:'',summary:'',downloads:1,downloadCount:1,authors:[]}));
export const searchModrinth=async()=>{hooks.searches++;return {hits:projects,total_hits:2,limit:10}};
export const searchCurseForgeModpacks=async()=>{hooks.searches++;return {data:projects,pagination:{totalCount:2,pageSize:10}}};
export const getModrinthProjectVersions=(id,a,b,signal)=>request('versions',id,signal);
export const getModrinthProject=(id,signal)=>request('modrinth-description',id,signal);
export const getCurseForgeProjectFiles=(id,a,b,signal)=>request('versions',id,signal);
export const getCurseForgeProjectDescription=(id,signal)=>request('curseforge-description',id,signal);
export const getCurseForgeFileDownloadUrl=()=>{throw Error('unexpected fallback')};
export const downloadMrpack=async(...args)=>{hooks.downloads.push(args);return '/pack'};
export const downloadCurseForgeModpack=downloadMrpack;
export const installMrpackWithUpstream=async(...args)=>{hooks.installs.push(args);return {}};
export const installCurseForgeModpack=installMrpackWithUpstream;
`;

const browserStub = `
<script>
import {hooks} from 'modpack-common';
let {query=$bindable(''),selectedItem=$bindable(null),selectedVersion=$bindable(''),customName=$bindable(''),customNameError=$bindable(null),filters=$bindable(),items,versionOptions,loadingVersions,installError,installing,onSelect,onBack,onSearch,onFilterChange,onInstall,onConfirmCustomName,detailExtra}=$props();
$effect(()=>{hooks.view={items,selectedItem,selectedVersion,versionOptions,loadingVersions,installError,installing,onSelect,onBack,onSearch,onFilterChange,onInstall,onConfirmCustomName}});
</script>
<input id="query" bind:value={query}/>
{#if selectedItem}{@render detailExtra(selectedItem)}{/if}
`;

const entry = `
import {mount,unmount,flushSync,tick} from 'svelte';
import Modrinth from ${JSON.stringify(join(directory, "ModrinthModpackBrowser.svelte"))};
import CurseForge from ${JSON.stringify(join(directory, "CurseForgeModpackBrowser.svelte"))};
import {hooks} from 'modpack-common';
const assert=(ok,msg)=>{if(!ok)throw Error(msg)};
const settle=async()=>{for(let i=0;i<4;i++){flushSync();await tick()}};
const resolvePair=pair=>{for(const request of pair){
 const id=String(request.id);
 request.resolve(request.kind==='versions'?[{id:'v'+id,project_id:id,version_number:'v'+id,game_versions:[],gameVersions:[],fileName:'pack.zip',files:[{url:'https://pack/'+id,primary:true}],downloadUrl:'https://pack/'+id}]:request.kind==='modrinth-description'?{body:'Description '+id}:'Description '+id);
}};
async function run(){
 for(const Component of [Modrinth,CurseForge]){
  hooks.requests=[];hooks.downloads=[];hooks.installs=[];hooks.searches=0;hooks.catalogs=0;
  let app=mount(Component,{target:document.querySelector('main')});await settle();
  assert(hooks.searches===1&&hooks.catalogs===1,'startup must load once');
  const input=document.querySelector('#query');input.value='test';input.dispatchEvent(new Event('input',{bubbles:true}));await settle();
  assert(hooks.searches===1&&hooks.catalogs===1,'typing repeated initialization');
  hooks.view.onFilterChange();await settle();
  assert(hooks.searches===2&&hooks.catalogs===1,'filter duplicated search/catalog');
  const [a,b]=hooks.view.items;
  let old=hooks.view.onSelect(a);await settle();const pairA=hooks.requests.slice(-2);
  hooks.view.onBack();await settle();
  let current=hooks.view.onSelect(b);await settle();const pairB=hooks.requests.slice(-2);
  assert(pairA.every(r=>r.signal.aborted),'back did not cancel detail');
  resolvePair(pairA);await old;await settle();
  assert(hooks.view.loadingVersions&&hooks.view.versionOptions.length===0,'old response cleared current loading');
  resolvePair(pairB);await current;await settle();
  assert(hooks.view.selectedVersion==='v2','wrong selected version');
  assert(document.querySelector('[data-description]').textContent==='Description 2','wrong description');
  // Resolve the current selection first, then an older request that ignores abort.
  old=hooks.view.onSelect(a);const late=hooks.requests.slice(-2);await settle();
  current=hooks.view.onSelect(b);const latest=hooks.requests.slice(-2);await settle();
  resolvePair(latest);await current;resolvePair(late);await old;await settle();
  assert(hooks.view.selectedVersion==='v2','late response replaced selected version');
  hooks.view.onInstall();await settle();hooks.view.onConfirmCustomName();await settle();
  assert(hooks.downloads[0][0]==='https://pack/2','downloaded another modpack');
  assert(String(hooks.installs[0][2])==='2'&&hooks.installs[0][3]==='v2','mixed install metadata');
  // An obsolete rejection must not overwrite the new selection or its loading flag.
  old=hooks.view.onSelect(a);const errors=hooks.requests.slice(-2);await settle();
  current=hooks.view.onSelect(b);const remaining=hooks.requests.slice(-2);await settle();
  errors[0].reject(Error('obsolete failure'));errors[1].resolve(null);await old;await settle();
  assert(!hooks.view.installError&&hooks.view.loadingVersions,'obsolete error affected selection');
  resolvePair(remaining);await current;await settle();
  // Current errors are handled, and leaving during another read aborts both requests.
  const failing=hooks.view.onSelect(a);const failed=hooks.requests.slice(-2);
  failed[0].reject(Error('current failure'));failed[1].resolve(null);await failing;await settle();
  assert(hooks.view.installError.includes('current failure')&&!hooks.view.loadingVersions,'current error not surfaced');
  const pending=hooks.view.onSelect(b);const closing=hooks.requests.slice(-2);await settle();
  await unmount(app);assert(closing.every(r=>r.signal.aborted),'unmount did not cancel');
  resolvePair(closing);await pending;await settle();
  assert(document.querySelector('main').textContent==='','unmounted detail reappeared');
 }
}
run().then(()=>fetch('/result',{method:'POST',body:JSON.stringify({ok:true})})).catch(error=>fetch('/result',{method:'POST',body:JSON.stringify({error:String(error),stack:error.stack})}));
`;

test.skipIf(!browserPath)(
	"modpack selection ignores obsolete results and installs the selected project",
	async () => {
		const bundle = await Bun.build({
			entrypoints: ["modpack-entry"],
			target: "browser",
			conditions: ["browser"],
			plugins: [
				{
					name: "modpack-selection",
					setup(build) {
						build.onResolve(
							{ filter: /^modpack-(entry|common)$/ },
							({ path }) => ({ path, namespace: "fixture" }),
						);
						build.onResolve({ filter: /.*/ }, ({ path }) => {
							if (path === "$lib/utils/instanceName")
								return {
									path: join(
										root,
										"src/lib/utils/instanceName.ts",
									),
								};
							if (
								path === "./ModpackBrowser.svelte" ||
								path.startsWith("$lib/components/") ||
								path.startsWith("$lib/icons/")
							)
								return { path, namespace: "component" };
							if (path.startsWith("$lib/"))
								return {
									path: "modpack-common",
									namespace: "fixture",
								};
						});
						build.onLoad(
							{ filter: /.*/, namespace: "fixture" },
							({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents: compileModule(
									path === "modpack-entry" ? entry : common,
									{
										filename: path + ".svelte.js",
										generate: "client",
									},
								).js.code,
							}),
						);
						build.onLoad(
							{ filter: /.*/, namespace: "component" },
							({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents: compile(
									path === "./ModpackBrowser.svelte"
										? browserStub
										: path.includes("Renderer")
											? "<script>let {source}=$props()</script><p data-description>{source}</p>"
											: "",
									{ filename: path, generate: "client" },
								).js.code,
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
					},
				},
			],
		});
		expect(bundle.success, bundle.logs.join("\n")).toBe(true);
		const js = await bundle.outputs[0].text();
		const profile = await mkdtemp(join(tmpdir(), "modpack-selection-"));
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
					return new Response(
						'<!doctype html><main></main><script type="module" src="/script.js"></script>',
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
