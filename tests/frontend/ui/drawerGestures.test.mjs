import { expect, test } from "bun:test";
import { compile, compileModule } from "svelte/compiler";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const root = resolve(import.meta.dir, "../../..");
const lib = join(root, "src/lib");
const browserPath = [
	"google-chrome",
	"google-chrome-stable",
	"chromium",
	"firefox",
]
	.map((name) => Bun.which(name))
	.find(Boolean);

// Mount the real notification controls inside the real Drawer. Only backend
// state, translations and icon artwork are replaced by browser-local fixtures.
const state = `
export const launcherStore = $state({settings:{reduce_animations:true,
 notification_preferences:{enabled:true,position:'top-right',size:'compact',
 title_size:13,message_size:11,uppercase_title:false,bold_title:false,duration_seconds:5}}});
export const showInfo=()=>'';
export const removeNotification=()=>{};
`;
const wrapper = `
<script>
 import Drawer from '$lib/components/layout/Drawer.svelte';
 import Controls from '$lib/components/settings/NotificationSettings.svelte';
 let { direction, onsave, onclose } = $props();
 let open=$state(true);
 export function isOpen(){return open}
 export function setOpen(value){open=value}
</script>
<Drawer bind:open {direction} {onclose}>
 <div id="free-space" style="height:40px;flex-shrink:0">Free space</div>
 <div style="overflow:auto">
  <Controls {onsave}/>
  <button id="button"><span>Button</span></button>
  <label id="label"><span>Label</span><input type="text"/></label>
  <textarea id="textarea"></textarea>
  <select id="native-select"><option>Option</option></select>
  <a id="link" href="#"><span>Link</span></a>
  <div id="editable" contenteditable="true"><span>Editable</span></div>
  <div id="slider-role" role="slider" tabindex="0" aria-valuenow="0"><span>Slider</span></div>
  <div id="no-drag" data-drawer-no-drag><span>No drag</span></div>
 </div>
</Drawer>`;
const entry = `
import {mount,unmount,flushSync,tick} from 'svelte';
import Wrapper from 'drawer-wrapper';
import {launcherStore} from '$lib/state/state.svelte';
const assert=(value,message)=>{if(!value)throw Error(message)};
const render=async()=>{flushSync();await tick()};
const frame=()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));

// Synthetic PointerEvents have no native active pointer. Record capture instead
// so we can assert that the Drawer never claims a slider's pointer stream.
const captured=new WeakMap();let captureCalls=0;
Element.prototype.setPointerCapture=function(id){captured.set(this,id);captureCalls++};
Element.prototype.hasPointerCapture=function(id){return captured.get(this)===id};
Element.prototype.releasePointerCapture=function(id){if(captured.get(this)===id)captured.delete(this)};
async function pointer(target,type,point,extra={}){
 target.dispatchEvent(new PointerEvent(type,{bubbles:true,pointerId:1,isPrimary:true,button:0,clientX:point.x,clientY:point.y,...extra}));
 await render();
}
const atRest=drawer=>drawer.style.transform.replaceAll(' ','').match(/^translate3d\\(0(?:%|px)?,0(?:%|px)?,0(?:%|px)?\\)$/);
let cases=0;
try {
 for(const direction of ['right','left','bottom','top']){
  let closes=0;const saves=[];
  const component=mount(Wrapper,{target:document.querySelector('main'),props:{direction,
   onsave:async()=>saves.push({...launcherStore.settings.notification_preferences}),onclose:()=>closes++}});
  await render();await frame();await render();
  const drawer=document.querySelector('.drawer');
  const blank=document.querySelector('#free-space');
  const start={x:100,y:100};const end={...start};
  const axis=['top','bottom'].includes(direction)?'y':'x';
  end[axis]+= ['top','left'].includes(direction)?-400:400;

  for(const key of ['title_size','message_size','duration_seconds']){
   const input=document.querySelector('#notification-'+key);const before=captureCalls;
   launcherStore.settings.notification_preferences[key]=Number(input.min);await render();
   await pointer(input,'pointerdown',start);
   await pointer(input,'pointermove',end);
   input.value=input.max;input.dispatchEvent(new Event('input',{bubbles:true}));await render();
   await pointer(input,'pointerup',end);
   input.dispatchEvent(new Event('change',{bubbles:true}));await render();
   assert(component.isOpen() && atRest(drawer),direction+': slider moved/closed Drawer');
   assert(captureCalls===before,direction+': Drawer captured slider');
   assert(saves.at(-1)[key]===Number(input.max),direction+': final slider value not saved');
   cases++;
  }
  for(const selector of ['#button span','#label span','#textarea','#native-select','#link span',
    '#editable span','#slider-role span','#no-drag span','#notification-position .selected-value',
    '#notification-customization']){
   const target=document.querySelector(selector);const before=captureCalls;
   await pointer(target,'pointerdown',start);await pointer(target,'pointermove',end);await pointer(target,'pointerup',end);
   assert(captureCalls===before && component.isOpen() && atRest(drawer),direction+': control stolen '+selector);
   cases++;
  }
  for(const extra of [{button:2},{pointerId:2,isPrimary:false}]){
   const before=captureCalls;
   await pointer(blank,'pointerdown',start,extra);await pointer(blank,'pointermove',end,extra);await pointer(blank,'pointerup',end,extra);
   assert(captureCalls===before && atRest(drawer),'non-primary drag');cases++;
  }
  await pointer(blank,'pointerdown',start);
  assert(drawer.hasPointerCapture(1),'Drawer should capture its own drag');
  await pointer(blank,'lostpointercapture',start);
  assert(drawer.hasPointerCapture(1),'child capture transfer cancelled Drawer gesture');
  await pointer(drawer,'pointermove',end,{pointerId:2,isPrimary:false});
  await pointer(drawer,'pointerup',end,{pointerId:2,isPrimary:false});
  assert(atRest(drawer) && drawer.hasPointerCapture(1),'second pointer hijacked drag');
  await pointer(drawer,'pointermove',end);assert(!atRest(drawer),'free-space drag not working');
  await pointer(drawer,'pointercancel',end);
  assert(component.isOpen() && atRest(drawer) && !drawer.hasPointerCapture(1) && closes===0,'cancel closed panel');cases++;

  await pointer(blank,'pointerdown',start);await pointer(drawer,'pointermove',end);
  captured.delete(drawer);await pointer(drawer,'lostpointercapture',end);
  await pointer(drawer,'pointerup',end);
  assert(component.isOpen() && atRest(drawer) && closes===0,'lost capture closed panel');cases++;

  await pointer(blank,'pointerdown',start);await pointer(drawer,'pointermove',end);
  component.setOpen(false);await render();
  assert(!drawer.hasPointerCapture(1),'external close leaked capture');
  component.setOpen(true);await render();await frame();await render();
  await pointer(drawer,'pointermove',end);
  assert(atRest(drawer),'stale drag after reopening');cases++;

  await pointer(blank,'pointerdown',start);
  await pointer(drawer,'pointerup',{x:101,y:101});
  assert(component.isOpen() && atRest(drawer),'short gesture should restore');cases++;
  await pointer(blank,'pointerdown',start);await pointer(drawer,'pointermove',end);await pointer(drawer,'pointerup',end);
  assert(!component.isOpen() && closes===1 && !drawer.hasPointerCapture(1),'free-space close failed');cases++;
  await unmount(component);
 }
 await fetch('/result',{method:'POST',body:JSON.stringify({cases})});
}catch(error){await fetch('/result',{method:'POST',body:JSON.stringify({error:String(error),stack:error.stack,cases})});}
`;

test.skipIf(!browserPath)(
	"Drawer excludes controls from gestures and cancels safely",
	async () => {
		const bundle = await Bun.build({
			entrypoints: ["drawer-entry"],
			target: "browser",
			conditions: ["browser"],
			plugins: [
				{
					name: "drawer-fixture",
					setup(build) {
						build.onResolve(
							{ filter: /^drawer-(entry|wrapper)$/ },
							({ path }) => ({ path, namespace: "fixture" }),
						);
						build.onResolve(
							{ filter: /^\$lib\/state\/state.svelte$/ },
							() => ({ path: "state", namespace: "fixture" }),
						);
						build.onResolve({ filter: /^\$lib\/i18n$/ }, () => ({
							path: "i18n",
							namespace: "fixture",
						}));
						build.onResolve(
							{ filter: /^\$lib\/icons\/registry$/ },
							() => ({ path: "registry", namespace: "fixture" }),
						);
						build.onResolve(
							{ filter: /(^|\/)Icon.svelte$/ },
							() => ({ path: "icon", namespace: "fixture" }),
						);
						build.onResolve({ filter: /^\$lib\// }, ({ path }) => ({
							path: Bun.resolveSync(
								path.replace("$lib", lib),
								root,
							),
						}));
						build.onResolve(
							{ filter: /^svelte(\/.*)?$/ },
							({ path }) => ({
								path: Bun.resolveSync(path, root),
							}),
						);
						build.onLoad(
							{ filter: /.*/, namespace: "fixture" },
							({ path }) => ({
								loader: "js",
								resolveDir: root,
								contents:
									path === "drawer-entry"
										? entry
										: path === "state"
											? compileModule(state, {
													filename:
														"fixture.svelte.js",
													generate: "client",
												}).js.code
											: path === "i18n"
												? "export const t=key=>key;"
												: path === "registry"
													? "export const getIconPath=()=>'';"
													: compile(
															path ===
																"drawer-wrapper"
																? wrapper
																: '<svg aria-hidden="true" width="16" height="16"></svg>',
															{
																filename:
																	path +
																	".svelte",
																generate:
																	"client",
																css: "injected",
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
		const profile = await mkdtemp(join(tmpdir(), "drawer-gestures-"));
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
						'<!doctype html><style>html{font-size:14px}body{margin:0;height:100vh;overflow:hidden}.drawer{--bg-sidebar:#222;--bg-input:#333;--text-secondary:#ddd;--border-color:#555;--accent:#a9f}</style><main></main><script type="module" src="/script.js"></script>',
						{ headers: { "Content-Type": "text/html" } },
					);
				},
			});
			const url = `http://127.0.0.1:${server.port}`;
			const firefox = browserPath.includes("firefox");
			browser = Bun.spawn(
				firefox
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
			watchdog = setTimeout(() => browser.kill("SIGKILL"), 20000);
			const payload = await Promise.race([
				result,
				browser.exited.then(async (code) => {
					throw Error(`Browser exited (${code}): ${await stderr}`);
				}),
			]);
			expect(payload.error, payload.stack).toBeUndefined();
			expect(payload.cases).toBe(80);
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
	30000,
);
