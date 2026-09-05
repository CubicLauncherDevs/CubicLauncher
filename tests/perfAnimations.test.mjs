import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const chromium = [
	"chromium",
	"chromium-browser",
	"google-chrome",
	"google-chrome-stable",
]
	.map((name) => Bun.which(name))
	.find(Boolean);
const name =
	"perf.css stops infinite repetition without freezing finite animations";

if (!chromium) {
	test.skip(`${name} (no Chromium executable available on PATH)`, () => {});
} else {
	test(
		name,
		async () => {
			const css = await readFile(
				new URL("../src/styles/shared/perf.css", import.meta.url),
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
</style><style>${css}</style><main></main><pre id="result"></pre>
<img hidden src="/hold-load" alt="">
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
  try {
    for (const [noInfinite, reduceMotion] of [[false, false], [true, false], [true, true], [false, false]]) {
      for (const [attr, enabled] of [['data-no-infinite-animations', noInfinite], ['data-reduce-motion', reduceMotion]]) {
        if (enabled) root.setAttribute(attr, 'true'); else root.removeAttribute(attr);
      }
      main.innerHTML = '<div class="fade">finite</div><div class="pseudo"></div>' +
        '<div class="preserve-motion fade">preserved</div>' +
        '<div class="preserve-motion"><div class="fade">child</div></div><div class="spinner">spin</div>';
      const initial = sample(); // Flush styles and start real CSS animations.
      await new Promise(resolve => setTimeout(resolve, 300));
      results.push({ noInfinite, reduceMotion, initial, final: sample() });
    }
    document.querySelector('#result').textContent = btoa(JSON.stringify({ results }));
  } catch (error) {
    document.querySelector('#result').textContent = btoa(JSON.stringify({ error: String(error) }));
  } finally {
    await fetch('/done', { method: 'POST' });
  }
})();
</script>`;
				// Hold page load until sampling finishes: virtual time does not reliably tick CSS animations.
				let releaseLoad;
				const loadGate = new Promise((resolve) => {
					releaseLoad = resolve;
				});
				server = Bun.serve({
					hostname: "127.0.0.1",
					port: 0,
					async fetch(request) {
						const path = new URL(request.url).pathname;
						if (path === "/done") {
							releaseLoad();
							return new Response(null, { status: 204 });
						}
						if (path === "/hold-load") {
							await loadGate;
							return new Response(null, { status: 204 });
						}
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
						"--disable-background-networking",
						"--disable-component-update",
						"--disable-sync",
						"--disable-extensions",
						"--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1",
						"--run-all-compositor-stages-before-draw",
						"--dump-dom",
						`--user-data-dir=${profile}`,
						`http://127.0.0.1:${server.port}/`,
					],
					{ stdout: "pipe", stderr: "pipe" },
				);
				watchdog = setTimeout(() => browser.kill("SIGKILL"), 20_000);
				const [stdout, stderr, exitCode] = await Promise.all([
					new Response(browser.stdout).text(),
					new Response(browser.stderr).text(),
					browser.exited,
				]);
				expect(exitCode, `Chromium failed: ${stderr}`).toBe(0);
				const encoded = stdout.match(
					/<pre\b[^>]*\bid="result"[^>]*>\s*([A-Za-z0-9+/=]+)\s*<\/pre>/,
				)?.[1];
				expect(
					encoded,
					`Missing fixture result: ${stdout}\n${stderr}`,
				).toBeTruthy();
				const payload = JSON.parse(
					Buffer.from(encoded, "base64").toString("utf8"),
				);
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
