<script lang="ts">
	import { onMount, onDestroy, tick } from "svelte";
	import { t } from "$lib/i18n";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import TutorialTipContent from "./TutorialTipContent.svelte";
	import TutorialTipFooter from "./TutorialTipFooter.svelte";

	interface Step {
		sel: string;
		key: string;
		pos?: "right" | "left";
		onEnter?: () => void;
		measureDelay?: number;
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
		{ sel: "[data-tutorial='sidebar-header']", key: "slide1" },
		{
			sel: "[data-tutorial='user-profile']",
			key: "slide2",
			pos: "right",
		},
		{ sel: "[data-tutorial='instance-list']", key: "slide3" },
		{ sel: "[data-tutorial='create-instance']", key: "slide4" },
		{ sel: "[data-tutorial='download-versions']", key: "slide5" },
		{ sel: "[data-tutorial='settings']", key: "slide6" },
		{
			sel: "[data-tutorial='settings-tabs']",
			key: "slide7",
			measureDelay: 400,
			onEnter: () => {
				onopensettings?.();
			},
		},
		{
			sel: "[data-tutorial='settings-scroll']",
			key: "slide8",
			measureDelay: 400,
			onEnter: () => {
				const javaTab = document.querySelector(
					"[data-tutorial='tab-java']",
				) as HTMLElement;
				if (javaTab) javaTab.click();
			},
		},
	];

	let currentStep = $state(0);
	let active = $state(false);
	let positioning = $state(false);
	let sx = $state(0);
	let sy = $state(0);
	let sw = $state(0);
	let sh = $state(0);
	let tx = $state(0);
	let ty = $state(0);
	let tipEl: HTMLElement | undefined = $state();
	let tipLeft = $state(false);
	let measuredW = $state(0);
	let measuredH = $state(0);
	let hasMeasured = $state(false);

	let closeTimer: ReturnType<typeof setTimeout> | undefined;
	let stepTimer: ReturnType<typeof setTimeout> | undefined;
	let rAFId: number | undefined;
	let measureTimer: ReturnType<typeof setTimeout> | undefined;

	function close() {
		active = false;
		closeTimer = setTimeout(() => {
			open = false;
			onclose?.();
		}, 150);
	}

	function goToStep(i: number) {
		if (i === currentStep) return;
		positioning = true;
		stepTimer = setTimeout(() => {
			currentStep = i;
		}, 150);
	}

	function next() {
		if (currentStep < steps.length - 1) goToStep(currentStep + 1);
	}
	function prev() {
		if (currentStep > 0) goToStep(currentStep - 1);
	}

	function expandTools() {
		try {
			if (localStorage.getItem("sidebar-tools") !== "true") {
				localStorage.setItem("sidebar-tools", "true");
			}
		} catch {
			/* localStorage not available */
		}
	}

	function updatePosition() {
		const step = steps[currentStep];
		const el = document.querySelector(step.sel);
		if (!el) {
			console.warn(
				`[Tutorial] Element not found: "${step.sel}" (step ${currentStep + 1}: "${step.key}")`,
			);
			return;
		}

		const r = el.getBoundingClientRect();
		const gap = 12;
		const m = 10;

		sx = r.left - 4;
		sy = r.top - 4;
		sw = r.width + 8;
		sh = r.height + 8;

		const tipW = hasMeasured && measuredW > 0 ? measuredW : 280;
		const tipH = hasMeasured && measuredH > 0 ? measuredH : 150;

		const spaceRight = window.innerWidth - r.right;
		const spaceLeft = r.left;
		const needed = tipW + gap + m;
		const preferRight = spaceRight >= needed;
		const preferLeft = spaceLeft >= needed;
		let left: boolean;
		if (preferRight && !preferLeft) left = false;
		else if (preferLeft && !preferRight) left = true;
		else left = spaceRight >= spaceLeft ? false : true;

		if (step.pos === "left") left = true;
		else if (step.pos === "right") left = false;

		tipLeft = left;

		let x = left ? r.left - tipW - gap : r.right + gap;
		let y = r.top + r.height / 2;

		y = Math.max(m, Math.min(y - tipH / 2, window.innerHeight - tipH - m));

		if (left && x < m) {
			x = r.right + gap;
			tipLeft = false;
		} else if (!left && x + tipW + m > window.innerWidth) {
			x = r.left - tipW - gap;
			tipLeft = true;
		}

		tx = x;
		ty = y;
	}

	async function showTip() {
		updatePosition();
		if (tipEl) {
			const tr = tipEl.getBoundingClientRect();
			measuredW = tr.width;
			measuredH = tr.height;
			hasMeasured = true;
			await tick();
			updatePosition();
		}
	}

	$effect(() => {
		if (!open || active) return;
		expandTools();
		positioning = true;
		rAFId = requestAnimationFrame(() => {
			active = true;
		});
	});

	$effect(() => {
		if (!active) return;
		const step = steps[currentStep];
		if (step.onEnter) step.onEnter();
		const resume = step.measureDelay
			? new Promise((r) => {
					measureTimer = setTimeout(r, step.measureDelay);
				})
			: Promise.resolve();
		tick()
			.then(() => resume)
			.then(showTip)
			.then(() => {
				positioning = false;
			});
	});

	function onResize() {
		if (active && !positioning) updatePosition();
	}

	onMount(() => window.addEventListener("resize", onResize));
	onDestroy(() => {
		window.removeEventListener("resize", onResize);
		clearTimeout(closeTimer);
		clearTimeout(stepTimer);
		cancelAnimationFrame(rAFId!);
		clearTimeout(measureTimer);
	});
</script>

{#if open}
	<div
		class="tut-overlay"
		class:visible={active}
		class:dim={!tipLeft}
		onclick={close}
		role="presentation"
	>
		<div
			class="tut-spotlight"
			style="--sx:{sx}px;--sy:{sy}px;--sw:{sw}px;--sh:{sh}px"
		></div>
	</div>

	<div
		class="tut-tip"
		class:visible={active}
		class:fading={positioning}
		class:left={tipLeft}
		style="--tx:{tx}px;--ty:{ty}px"
		bind:this={tipEl}
		role="dialog"
	>
		<div class="tut-arrow"></div>

		<button
			type="button"
			class="tut-close"
			onclick={close}
			aria-label={t("tutorial.skip")}
		>
			<CloseIcon size={20} />
		</button>

		<TutorialTipContent
			stepKey={steps[currentStep].key}
			isFirstStep={currentStep === 0}
		/>

		<TutorialTipFooter
			{currentStep}
			totalSteps={steps.length}
			onprev={prev}
			onnext={next}
			ongoToStep={goToStep}
			onfinish={close}
		/>
	</div>
{/if}

<style>
	.tut-tip {
		opacity: 0;
		transform: translateY(8px);
		transition:
			opacity 150ms ease,
			transform 150ms ease;
		pointer-events: none;
	}
	.tut-tip.visible {
		opacity: 1;
		transform: translateY(0);
		pointer-events: auto;
	}
	.tut-tip.visible.fading {
		opacity: 0;
		transform: translateY(8px);
	}
</style>
