import { expect, test } from "bun:test";
import { compile, compileModule } from "svelte/compiler";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const root = resolve(import.meta.dir, "../../..");
const view = join(
	root,
	"src/lib/components/instances/InstanceView/InstanceView.svelte",
);
const browserPath = [
	"google-chrome",
	"google-chrome-stable",
	"chromium",
	"firefox",
]
	.map((name) => Bun.which(name))
	.find(Boolean);
const common = `
export const props=$state({selectedInstance:{uuid:'A',name:'A',loader:'Vanilla',status:'off',version:'1.21'}});
export const lists={A:Array.from({length:1000},(_,i)=>'/A/'+i+'.png'),B:['/B/one.png'],Empty:[]};
export const calls={thumbnails:[],originals:[],deletions:[],active:0,max:0,restores:[],refreshes:[]};
export const hooks={list:null,deletion:null,pack:null,restore:null};
export const packFor=id=>['A','B'].includes(id)?{schema_version:1,source:'local',project_id:null,version_id:null,name:'Pack '+id,version:'1',game_version:'fabric-loader-0.16.0-1.21',locked:true,files:{}}:null;
const pixel='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==';
export const invoke=async(command,args)=>{
	if(command==='get_instance_modpack')return hooks.pack?hooks.pack(args.id):packFor(args.id);
	if(command==='restore_modpack_inventory'){
	 calls.restores.push(args);
	 return hooks.restore?hooks.restore(args.id):{...packFor(args.id),needs_inventory:false};
	}
 if(command==='get_all_instance_screenshots')return hooks.list?hooks.list(args.instanceName):lists[args.instanceName]??[];
 if(command==='get_screenshot_thumbnail'){
  calls.thumbnails.push(args);calls.max=Math.max(calls.max,++calls.active);
  await new Promise(r=>setTimeout(r,2));calls.active--;return pixel;
 }
 throw Error(command);
};
export const convertFileSrc=path=>{calls.originals.push(path);return pixel};
export const deleteInstanceFile=async(id,dir,filename)=>{
 calls.deletions.push({id,dir,filename});
 if(hooks.deletion)await hooks.deletion();
 lists[id]=lists[id].filter(path=>!path.endsWith('/'+filename));
};
export const t=key=>key;
export const launcherStore={settings:{reduce_animations:true}};
export const launchInstance=()=>{};
export const killInst=()=>{};
const refreshCallbacks=new Map();
export const refreshInstanceMods=id=>{calls.refreshes.push(id);for(const callback of refreshCallbacks.get(id)??[])callback()};
export const registerModsRefreshCallback=(id,callback)=>{
 if(!refreshCallbacks.has(id))refreshCallbacks.set(id,new Set());
 refreshCallbacks.get(id).add(callback);
 return ()=>{refreshCallbacks.get(id)?.delete(callback)};
};
export const isVersionDownloading=()=>false;
export const downloads=new Map();
export const getOverallPct=()=>0;
`;
const entry = `
import {mount,unmount,flushSync,tick} from 'svelte';
import View from ${JSON.stringify(view)};
import {props,calls,hooks,lists,packFor,refreshInstanceMods} from 'gallery-common';
const assert=(ok,message)=>{if(!ok)throw Error(message)};
const settle=async()=>{flushSync();await tick();await new Promise(r=>setTimeout(r,40));flushSync();await tick()};
const wait=async(fn,message)=>{for(let i=0;i<100&&!fn();i++)await settle();assert(fn(),message)};
const cards=()=>[...document.querySelectorAll('.screenshot-card')];
const change=id=>{props.selectedInstance={...props.selectedInstance,uuid:id,name:id}};
async function run(){
 window.confirm=()=>true;
 const app=mount(View,{target:document.querySelector('main'),props});
 await settle();
 assert(document.querySelector('.pack-summary strong')?.textContent==='Pack A','pack summary did not mount');
 hooks.pack=id=>({...packFor(id),name:'Edited '+id});
 refreshInstanceMods('A');await settle();
 assert(document.querySelector('.pack-summary strong')?.textContent==='Edited A','edited pack metadata did not refresh');
 let legacy={...packFor('A'),needs_inventory:true};
 hooks.pack=id=>id==='A'?legacy:packFor(id);
 refreshInstanceMods('A');await settle();
 const packButton=key=>[...document.querySelectorAll('.pack-summary button')].find(button=>button.textContent.trim()===key);
 const inventoryButton=()=>packButton('modpack.restoreInventory');
 assert(document.querySelector('.inventory-notice')?.textContent.includes('modpack.needsInventory'),'legacy inventory hint missing');
 assert(inventoryButton()&&!inventoryButton().disabled,'legacy inventory recovery unavailable');
 assert(!packButton('modpack.unlock').disabled&&!packButton('modpack.update').disabled,'legacy state blocks unlocking or update');
 hooks.restore=async()=>{throw Error('Inventory offline')};
 inventoryButton().click();await settle();
 assert(document.querySelector('.pack-summary [role="alert"]')?.textContent.includes('Inventory offline'),'inventory error hidden');
 assert(inventoryButton()&&!inventoryButton().disabled,'inventory failure prevented retry');
 let finishInventory;
 hooks.restore=()=>new Promise(resolve=>finishInventory=resolve);
 inventoryButton().click();await settle();
 assert(inventoryButton().disabled&&packButton('modpack.unlock').disabled,'inventory restore allows overlapping mutations');
 refreshInstanceMods('A');await settle();
 const refreshes=calls.refreshes.length;
 legacy={...legacy,needs_inventory:false,name:'Recovered A'};
 finishInventory(legacy);await settle();
 assert(!document.querySelector('.inventory-notice'),'successful inventory recovery retained legacy hint');
 assert(document.querySelector('.pack-summary strong')?.textContent==='Recovered A','inventory recovery lost returned metadata');
 assert(calls.refreshes.length>refreshes&&calls.refreshes.at(-1)==='A','inventory recovery did not refresh mods');
 assert(calls.restores.length===2&&calls.restores.every(args=>args.id==='A'),'inventory command targeted wrong instance');
 legacy={...legacy,needs_inventory:true};
 refreshInstanceMods('A');await settle();
 inventoryButton().click();await settle();
 change('B');await settle();
 finishInventory({...legacy,needs_inventory:false,name:'Late recovery'});await settle();
 assert(document.querySelector('.pack-summary strong')?.textContent==='Pack B','late inventory recovery replaced current instance');
 assert(!document.querySelector('.inventory-notice')&&!document.querySelector('.pack-summary [role="alert"]'),'old inventory state leaked into new instance');
 hooks.pack=null;
 hooks.restore=null;
 change('A');await settle();
 assert(!document.querySelector('.inventory-notice'),'optional needs_inventory missing from mock treated as legacy');
 document.querySelector('#screenshots').click();
 await wait(()=>cards().length>0,'gallery did not mount');
 const grid=document.querySelector('.screenshots-grid');
 assert(grid.clientHeight>100&&grid.clientHeight<600,'gallery has no bounded viewport');
 assert(cards().length<100,'1000 screenshots created unbounded DOM');
 await wait(()=>document.querySelector('.screenshot-card img'),'no thumbnail rendered');
 assert(calls.originals.length===0,'gallery loaded original images');
 assert(calls.thumbnails.length<100,'offscreen thumbnails requested');
 assert(calls.max===1,'concurrent thumbnail IPC');
 document.querySelector('.open-screenshot').click();
 await settle();
 assert(document.querySelector('.viewer-container img')&&calls.originals[0]==='/A/0.png','original viewer missing');
 window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape'}));
 await settle();
 assert(!document.querySelector('.viewer-container'),'Escape did not close viewer');
 grid.scrollTop=grid.scrollHeight;grid.dispatchEvent(new Event('scroll'));
 await wait(()=>document.querySelector('[data-index="999"]'),'last screenshot unreachable');
 assert(cards().length<100,'scroll grew DOM');
 const last=document.querySelector('[data-index="999"] .open-screenshot');
 last.focus();last.dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}));
 await wait(()=>document.activeElement?.closest('[data-index="0"]'),'keyboard Home failed');
 document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true}));
 await wait(()=>document.activeElement?.closest('[data-index="999"]'),'keyboard End failed');
 for(const width of [360,700,1100]){
  document.querySelector('main').style.width=width+'px';
  grid.style.setProperty('--screenshot-min-width','calc(9rem + 12px)');
  grid.style.setProperty('--screenshot-gap','1.5rem');
  await settle();
  grid.scrollTop=grid.scrollHeight;grid.dispatchEvent(new Event('scroll'));
  await wait(()=>document.querySelector('[data-index="999"]'),'resize lost final row');
  const final=document.querySelector('[data-index="999"]').getBoundingClientRect();
  const bounds=grid.getBoundingClientRect();
  assert(final.bottom<=bounds.bottom+2&&final.top>=bounds.top-2,'resized row outside viewport');
  const rows=[...document.querySelectorAll('.screenshot-row')].map(el=>el.getBoundingClientRect());
  for(let i=1;i<rows.length;i++)assert(rows[i].top>=rows[i-1].bottom,'rows overlap after resize');
 }
  let releaseList;
  let releasePack;
  hooks.pack=id=>id==='Slow'?new Promise(r=>releasePack=r):packFor(id);
 hooks.list=name=>name==='Slow'?new Promise(r=>releaseList=r):lists[name]??[];
 change('Slow');await settle();change('B');
 await wait(()=>cards().length===1,'new instance did not load');
  releaseList(['/Slow/late.png']);await settle();
  releasePack({...packFor('A'),name:'Stale pack'});await settle();
  assert(document.querySelector('.pack-summary strong')?.textContent==='Pack B','late pack response replaced selected instance');
  hooks.pack=null;
 assert(document.querySelector('.open-screenshot').getAttribute('aria-label')==='one.png','stale list replaced current instance');
 document.querySelector('.open-screenshot').click();await settle();
 let releaseDelete;
 hooks.deletion=()=>new Promise(r=>releaseDelete=r);
 document.querySelector('.delete-btn').click();await settle();
 change('Empty');await settle();
 assert(!document.querySelector('.viewer-container'),'instance switch retained viewer');
 releaseDelete();await settle();
 assert(cards().length===0&&document.querySelector('.empty-state'),'late deletion changed current instance');
 assert(calls.deletions[0].id==='B','deleted from wrong instance');
 hooks.list=null;hooks.deletion=null;
 lists.B=['/B/new.png'];change('B');await wait(()=>cards().length===1,'reload failed');
 document.querySelector('.delete-btn').click();
 await wait(()=>cards().length===0,'deleting last screenshot left a stale row');
  change('A');await settle();
  assert(document.querySelector('.pack-summary strong')?.textContent==='Pack A','returning to an instance lost pack metadata');
 await unmount(app);
 const after=calls.thumbnails.length;await new Promise(r=>setTimeout(r,150));
 assert(calls.thumbnails.length===after,'thumbnail work continued after unmount');
 return {ok:true,maxConcurrent:calls.max};
}
run().then(result=>fetch('/result',{method:'POST',body:JSON.stringify(result)})).catch(error=>fetch('/result',{method:'POST',body:JSON.stringify({error:String(error),stack:error.stack})}));
`;

test.skipIf(!browserPath)(
	"gallery virtualizes 1000 screenshots, preserves navigation and rejects stale work",
	async () => {
		const bundle = await Bun.build({
			entrypoints: ["gallery-entry"],
			target: "browser",
			conditions: ["browser"],
			plugins: [
				{
					name: "screenshots-gallery",
					setup(build) {
						build.onResolve(
							{ filter: /^gallery-(entry|common)$/ },
							({ path }) => ({ path, namespace: "fixture" }),
						);
						build.onResolve({ filter: /.*/ }, ({ path }) => {
							if (path.endsWith("InstanceHeader.svelte"))
								return { path: "header", namespace: "fixture" };
							if (path.startsWith("$lib/icons/"))
								return { path: "icon", namespace: "fixture" };
							if (
								[
									"@tauri-apps/api/core",
									"$lib/api/cubicApi",
									"$lib/api/launcherService",
									"$lib/state/state.svelte",
									"$lib/state/downloadState.svelte",
									"$lib/state/downloadQueueState.svelte",
									"$lib/i18n",
								].includes(path)
							)
								return {
									path: "gallery-common",
									namespace: "fixture",
								};
							if (
								path.endsWith("/Market.svelte") ||
								path.endsWith("/WorldsTab.svelte") ||
								path.endsWith("/ServersTab.svelte")
							)
								return { path: "icon", namespace: "fixture" };
							if (path.startsWith("$lib/"))
								return {
									path: Bun.resolveSync(
										path.replace(
											"$lib",
											join(root, "src/lib"),
										),
										root,
									),
								};
						});
						build.onLoad(
							{ filter: /.*/, namespace: "fixture" },
							({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents:
									path === "gallery-entry"
										? entry
										: path === "gallery-common"
											? compileModule(common, {
													filename:
														"common.svelte.js",
													generate: "client",
												}).js.code
											: compile(
													path === "header"
														? '<script>let {activeSection=$bindable()}=$props()</script><button id="screenshots" onclick={()=>activeSection="screenshots"}>Screenshots</button>'
														: "",
													{
														filename:
															path + ".svelte",
														generate: "client",
													},
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
		const profile = await mkdtemp(join(tmpdir(), "screenshots-gallery-"));
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
						`<!doctype html><meta charset="utf-8"><style>*{box-sizing:border-box}body{margin:0}main{display:flex;width:800px;height:600px}</style><main></main><script>
						const reportError=error=>fetch('/result',{method:'POST',body:JSON.stringify({error:String(error),stack:error?.stack})});
						addEventListener('error',event=>reportError(event.error??event.message));
						addEventListener('unhandledrejection',event=>reportError(event.reason));
						</script><script type="module" src="/script.js"></script>`,
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
			expect(payload.maxConcurrent).toBe(1);
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
