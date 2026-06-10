<script lang="ts">
	import { onMount, onDestroy, tick } from "svelte";
	import { t } from "$lib/i18n";

	interface Step {
		sel: string;
		key: string;
		pos: "right" | "left";
		onEnter?: () => void;
	}

	let {
		open = $bindable(false),
		onclose,
		onopensettings,
	}: {
		open: boolean;
		onclose?: () => void;
		onopensettings?: () => void;
	} = $props();

	const steps: Step[] = [
		{ sel: "[data-tutorial='sidebar-header']", key: "slide1", pos: "right" },
		{ sel: "[data-tutorial='instance-list']", key: "slide2", pos: "right" },
		{ sel: "[data-tutorial='create-instance']", key: "slide3", pos: "right" },
		{ sel: "[data-tutorial='download-versions']", key: "slide4", pos: "right" },
		{
			sel: "[data-tutorial='settings']", key: "slide5", pos: "right",
			onEnter: () => { onopensettings?.(); },
		},
		{ sel: "[data-tutorial='settings-tabs']", key: "slide6", pos: "right" },
		{
			sel: "[data-tutorial='settings-scroll']", key: "slide7", pos: "right",
			onEnter: () => {
				const javaTab = document.querySelector("[data-tutorial='tab-java']") as HTMLElement;
				if (javaTab) javaTab.click();
			},
		},
	];

	let currentStep = $state(0);
	let active = $state(false);
	let sx = $state(0);
	let sy = $state(0);
	let sw = $state(0);
	let sh = $state(0);
	let tx = $state(0);
	let ty = $state(0);

	const isInside = $derived(steps[currentStep].pos === "left");

	function close() {
		active = false;
		setTimeout(() => { open = false; onclose?.(); }, 150);
	}

	function next() { if (currentStep < steps.length - 1) currentStep++; }
	function prev() { if (currentStep > 0) currentStep--; }

	function expandTools() {
		try {
			if (localStorage.getItem("sidebar-tools") !== "true") {
				localStorage.setItem("sidebar-tools", "true");
			}
		} catch { /* localStorage not available */ }
	}

	function updatePosition() {
		const step = steps[currentStep];
		const el = document.querySelector(step.sel);
		if (!el) return;

		const r = el.getBoundingClientRect();
		const gap = 12;
		const tipW = 260;
		const tipH = 150;
		const m = 10;
		const left = step.pos === "left";

		sx = r.left - 4;
		sy = r.top - 4;
		sw = r.width + 8;
		sh = r.height + 8;

		let x = left ? r.left - tipW - gap : r.right + gap;
		let y = r.top + r.height / 2;

		y = Math.max(m, Math.min(y - tipH / 2, window.innerHeight - tipH - m));

		if (left && x < m) x = r.right + gap;
		else if (!left && x + tipW + m > window.innerWidth) x = r.left - tipW - gap;

		tx = x;
		ty = y;
	}

	$effect(() => {
		if (!open || active) return;
		expandTools();
		requestAnimationFrame(() => { active = true; });
	});

	$effect(() => {
		if (!active) return;
		const step = steps[currentStep];
		if (step.onEnter) step.onEnter();
		tick().then(updatePosition);
	});

	function onResize() { if (active) updatePosition(); }

	onMount(() => window.addEventListener("resize", onResize));
	onDestroy(() => window.removeEventListener("resize", onResize));
</script>

{#if open}
	<div
		class="tut-overlay"
		class:visible={active}
		class:dim={!isInside}
		onclick={close}
		role="presentation"
	>
		<div class="tut-spotlight" style="--sx:{sx}px;--sy:{sy}px;--sw:{sw}px;--sh:{sh}px"></div>
	</div>

	<div
		class="tut-tip"
		class:visible={active}
		class:left={isInside}
		style="--tx:{tx}px;--ty:{ty}px"
		role="dialog"
	>
		<div class="tut-arrow"></div>

		<button type="button" class="tut-close" onclick={close} aria-label={t("tutorial.skip")}>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
				<line x1="18" y1="6" x2="6" y2="18"></line>
				<line x1="6" y1="6" x2="18" y2="18"></line>
			</svg>
		</button>

		<div class="tut-body">
			<h3 class="tut-title">{t(`tutorial.${steps[currentStep].key}.title`)}</h3>
			<p class="tut-desc">{t(`tutorial.${steps[currentStep].key}.desc`)}</p>
		</div>

		<div class="tut-footer">
			<div class="tut-dots">
				{#each steps as _, i (i)}
					<button type="button" class="tut-dot" class:active={i === currentStep} onclick={() => (currentStep = i)} aria-label="Step {i + 1}"></button>
				{/each}
			</div>
			<div class="tut-nav">
				{#if currentStep > 0}
					<button type="button" class="btn-secondary tut-btn" onclick={prev}>{t("tutorial.prev")}</button>
				{/if}
				{#if currentStep < steps.length - 1}
					<button type="button" class="btn-primary tut-btn" onclick={next}>{t("tutorial.next")}</button>
				{:else}
					<button type="button" class="btn-primary tut-btn" onclick={close}>{t("tutorial.finish")}</button>
				{/if}
			</div>
		</div>
	</div>
{/if}
