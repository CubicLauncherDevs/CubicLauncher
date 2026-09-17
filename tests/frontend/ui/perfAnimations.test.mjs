import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Ubuntu runners also expose Snap launcher stubs under chromium-browser.
// Prefer the installed Chrome binary; CI provides an explicit path so a
// missing browser fails rather than silently skipping the integration test.
const configuredBrowser = process.env.CHROME_BIN;
const chromium = configuredBrowser
	? Bun.which(configuredBrowser)
	: ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]
			.map((name) => Bun.which(name))
			.find(Boolean);
if (configuredBrowser && !chromium) {
	throw new Error(`CHROME_BIN is not executable: ${configuredBrowser}`);
}
const name =
	"perf.css stops infinite repetition without freezing finite animations";

if (!chromium) {
	test.skip(`${name} (no Chromium executable available on PATH)`, () => {});
} else {
	test(
		name,
		async () => {
			const css = await readFile(
				new URL("../../../src/styles/shared/perf.css", import.meta.url),
				"utf8",
			);
			let server, browser, profile, watchdog;
			try {
				profile = await mkdtemp(join(tmpdir(), "perf-animations-"));
				const html = `<!doctype html><meta charset="utf-8">
<link rel="icon" href="data:,">
<style>
@keyframes fade { from { opacity: 0; } to { opacity: 1; } }
@keyframes spin { to { transform: rotate(360deg); } }
.fade, .pseudo::before, .pseudo::after {
  opacity: 0; animation: fade 80ms linear forwards;
}
.pseudo::before, .pseudo::after { content: "fade"; display: inline-block; }
.spinner, .spinner::before, .spinner::after {
  display: inline-block; animation: spin 80ms linear infinite;
}
.spinner::before, .spinner::after { content: "spin"; }
</style><style>${css}</style><main></main>
<script>
(async () => {
  const results = [];
  const root = document.documentElement;
  const main = document.querySelector('main');
  const sample = () => {
    const entries = [
      ['finite', '.fade'], ['before', '.pseudo', '::before'],
      ['after', '.pseudo', '::after'], ['preserved', '.preserve-motion.fade'],
      ['preservedChild', '.preserve-motion .fade'],
      ['spinner', '.spinner'], ['spinnerBefore', '.spinner', '::before'],
      ['spinnerAfter', '.spinner', '::after']
    ];
    return Object.fromEntries(entries.map(([name, selector, pseudo]) => {
      const style = getComputedStyle(main.querySelector(selector), pseudo);
      return [name, { opacity: Number(style.opacity), playState: style.animationPlayState,
        iterations: style.animationIterationCount }];
    }));
  };
  let payload;
  try {
    for (const [noInfinite, reduceMotion] of [[false, false], [true, false], [true, true], [false, false]]) {
      for (const [attr, enabled] of [['data-no-infinite-animations', noInfinite], ['data-reduce-motion', reduceMotion]]) {
        if (enabled) root.setAttribute(attr, 'true'); else root.removeAttribute(attr);
      }
      main.innerHTML = '<div class="fade">finite</div><div class="pseudo"></div>' +
        '<div class="preserve-motion fade">preserved</div>' +
        '<div class="preserve-motion"><div class="fade">child</div></div><div class="spinner">spin</div>';
      const initial = sample(); // Flush styles and start real CSS animations.
      // Wait for the browser's animation timeline, not an arbitrary wall-clock
      // delay that can expire before a busy CI renderer has painted a frame.
      const finite = main.getAnimations({ subtree: true }).filter(animation =>
        Number.isFinite(animation.effect.getComputedTiming().endTime));
      await Promise.all(finite.map(animation => animation.finished));
      results.push({ noInfinite, reduceMotion, initial, final: sample() });
    }
    payload = { results };
  } catch (error) {
    payload = { error: String(error) };
  }
  await fetch('/result', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
})();
</script>`;
				// Let the page load normally and report after real-time CSS sampling.
				// --dump-dom plus a pending load can stall headless rendering indefinitely.
				let reportResult, rejectResult;
				const result = new Promise((resolve, reject) => {
					reportResult = resolve;
					rejectResult = reject;
				});
				let pageRequested = false;
				server = Bun.serve({
					hostname: "127.0.0.1",
					port: 0,
					async fetch(request) {
						const path = new URL(request.url).pathname;
						if (path === "/result" && request.method === "POST") {
							try {
								reportResult(await request.json());
							} catch (error) {
								rejectResult(error);
								return new Response(null, { status: 400 });
							}
							return new Response(null, { status: 204 });
						}
						if (path !== "/") {
							return new Response(null, { status: 404 });
						}
						pageRequested = true;
						return new Response(html, {
							headers: {
								"Content-Type": "text/html; charset=utf-8",
								"Content-Security-Policy":
									"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src 'self' data:; connect-src 'self'",
							},
						});
					},
				});
				browser = Bun.spawn(
					[
						chromium,
						"--headless",
						"--no-sandbox",
						"--disable-gpu",
						"--disable-dev-shm-usage",
						"--no-first-run",
						"--no-default-browser-check",
						"--password-store=basic",
						"--disable-background-networking",
						"--disable-component-update",
						"--disable-sync",
						"--disable-extensions",
						"--disable-background-timer-throttling",
						"--disable-renderer-backgrounding",
						"--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1",
						"--run-all-compositor-stages-before-draw",
						"--remote-debugging-port=0",
						`--user-data-dir=${profile}`,
						`http://127.0.0.1:${server.port}/`,
					],
					{ stdout: "ignore", stderr: "pipe" },
				);
				const stderr = new Response(browser.stderr).text();
				let timedOut = false;
				watchdog = setTimeout(() => {
					timedOut = true;
					browser.kill("SIGKILL");
				}, 20_000);
				const payload = await Promise.race([
					result,
					browser.exited.then(async (exitCode) => {
						throw new Error(
							`${timedOut ? `Chromium timed out ${pageRequested ? "running animations" : "starting before loading the test page"}` : `Chromium exited before reporting animation results (code ${exitCode})`}; executable: ${chromium}; page requested: ${pageRequested}\n${await stderr}`,
						);
					}),
				]);
				clearTimeout(watchdog);
				expect(payload.error).toBeUndefined();
				expect(payload.results).toHaveLength(4);
				for (const phase of payload.results) {
					const context = JSON.stringify(phase);
					for (const key of [
						"finite",
						"before",
						"after",
						"preserved",
						"preservedChild",
					]) {
						if (!phase.reduceMotion)
							expect(
								phase.initial[key].opacity,
								context,
							).toBeLessThan(1);
						expect(phase.final[key].opacity, context).toBe(1);
						expect(phase.final[key].playState, context).toBe(
							"running",
						);
					}
					for (const key of [
						"spinner",
						"spinnerBefore",
						"spinnerAfter",
					]) {
						expect(phase.final[key].iterations, context).toBe(
							phase.noInfinite ? "1" : "infinite",
						);
						expect(phase.final[key].playState, context).toBe(
							"running",
						);
					}
				}
			} finally {
				clearTimeout(watchdog);
				if (browser) {
					if (browser.exitCode === null) browser.kill("SIGKILL");
					await browser.exited;
				}
				await server?.stop(true);
				if (profile)
					await rm(profile, {
						recursive: true,
						force: true,
						maxRetries: 5,
						retryDelay: 100,
					});
			}
		},
		30_000,
	);
}
