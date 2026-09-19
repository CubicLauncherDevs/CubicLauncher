<script lang="ts">
	import { onMount, onDestroy, tick } from "svelte";
	import { t } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import TutorialTipContent from "./TutorialTipContent.svelte";
	import TutorialTipFooter from "./TutorialTipFooter.svelte";
	import ModalBase from "../ModalBase.svelte";

	interface Step {
		sel: string;
		key: string;
		pos?: "right" | "left" | "center";
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
		{ sel: "body", key: "slide9", pos: "center" },
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
	let missingTarget = $state(false);
	let showSkipConfirmation = $state(false);
	let skipping = $state(false);

	let closeTimer: ReturnType<typeof setTimeout> | undefined;
	let stepTimer: ReturnType<typeof setTimeout> | undefined;
	let rAFId: number | undefined;
	let measureTimer: ReturnType<typeof setTimeout> | undefined;

	const isLicenseStep = $derived(currentStep === steps.length - 1);
	const isCentered = $derived(
		steps[currentStep]?.pos === "center" || missingTarget,
	);
	const canFinish = $derived(
		!isLicenseStep || launcherStore.settings.license_accepted,
	);
	const canSkip = $derived(
		!isLicenseStep || launcherStore.settings.license_accepted,
	);

	function closeTutorial() {
		active = false;
		closeTimer = setTimeout(() => {
			open = false;
			onclose?.();
		}, 150);
	}

	function requestSkip() {
		if (!canSkip || skipping) return;
		showSkipConfirmation = true;
	}

	async function skipTutorial() {
		if (!showSkipConfirmation || skipping) return;
		showSkipConfirmation = false;
		if (!launcherStore.settings.license_accepted) {
			goToStep(steps.length - 1);
			return;
		}
		skipping = true;
		try {
			launcherStore.settings.show_tutorial = false;
			await saveSettings();
			closeTutorial();
		} finally {
			skipping = false;
		}
	}

	async function finishTutorial() {
		if (!canFinish) return;
		if (isLicenseStep) {
			launcherStore.settings.license_accepted = true;
		}
		launcherStore.settings.show_tutorial = false;
		await saveSettings();
		closeTutorial();
	}

	function goToStep(i: number) {
		if (i === currentStep) return;
		clearTimeout(stepTimer);
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
		const r = el?.getBoundingClientRect();
		const gap = 12;
		const m = 10;
		const tipBounds = tipEl?.getBoundingClientRect();
		const tipW = tipBounds?.width || 280;
		const tipH = tipBounds?.height || 150;
		missingTarget = !r || r.width === 0 || r.height === 0;

		if (step.pos === "center" || missingTarget || !r) {
			sx = 0;
			sy = 0;
			sw = window.innerWidth;
			sh = window.innerHeight;
			tipLeft = false;
			tx = Math.max(m, (window.innerWidth - tipW) / 2);
			ty = Math.max(m, (window.innerHeight - tipH) / 2);
			return;
		}

		sx = r.left - 4;
		sy = r.top - 4;
		sw = r.width + 8;
		sh = r.height + 8;

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

		tx = Math.max(m, Math.min(x, window.innerWidth - tipW - m));
		ty = y;
	}

	$effect(() => {
		if (!tipEl) return;
		const observer = new ResizeObserver(() => {
			if (active) updatePosition();
		});
		observer.observe(tipEl);
		return () => observer.disconnect();
	});

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
			.then(updatePosition)
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
		onclick={requestSkip}
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
		class:center={isCentered}
		style="--tx:{tx}px;--ty:{ty}px"
		bind:this={tipEl}
		role="dialog"
		inert={showSkipConfirmation || skipping}
	>
		<div class="tut-arrow"></div>

		{#if canSkip}
			<button
				type="button"
				class="tut-close"
				onclick={requestSkip}
				aria-label={t("tutorial.skip")}
			>
				<CloseIcon size={20} />
			</button>
		{/if}

		<TutorialTipContent
			stepKey={steps[currentStep].key}
			isFirstStep={currentStep === 0}
			{isLicenseStep}
		/>

		<TutorialTipFooter
			{currentStep}
			totalSteps={steps.length}
			onprev={prev}
			onnext={next}
			ongoToStep={goToStep}
			onfinish={finishTutorial}
			{canFinish}
		/>
		{#if canSkip}
			<button
				type="button"
				class="btn-secondary tut-btn tut-skip"
				onclick={requestSkip}
			>
				{t("tutorial.skipTutorial")}
			</button>
		{/if}
	</div>

	<ModalBase
		bind:open={showSkipConfirmation}
		title={t("tutorial.skipConfirmTitle")}
	>
		<p class="confirmation-message">
			{t("tutorial.skipConfirmDesc")}
		</p>
		{#if !launcherStore.settings.license_accepted}
			<p class="confirmation-message">
				{t("tutorial.skipLicenseNotice")}
			</p>
		{/if}
		{#snippet footer()}
			<button
				type="button"
				class="btn-secondary"
				onclick={() => (showSkipConfirmation = false)}
			>
				{t("tutorial.continueTutorial")}
			</button>
			<button type="button" class="btn-primary" onclick={skipTutorial}>
				{t("tutorial.confirmSkip")}
			</button>
		{/snippet}
	</ModalBase>
{/if}

<style>
	.tut-skip {
		align-self: flex-end;
	}

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
