<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import type { Snippet } from "svelte";
	import { shouldReduceAnimations } from "$lib/utils/animations";

	type Direction = "bottom" | "top" | "left" | "right";

	interface Props {
		open?: boolean;
		onclose?: () => void;
		dismissible?: boolean;
		direction?: Direction;
		closeThreshold?: number;
		class?: string;
		style?: string;
		children?: Snippet;
	}

	let {
		open = $bindable(false),
		onclose,
		dismissible = true,
		direction = "bottom",
		closeThreshold = 0.25,
		class: className = "",
		style = "",
		children,
	}: Props = $props();

	const isVertical = $derived(direction === "bottom" || direction === "top");

	let translatePct = $state(getClosedTranslate());
	let transitionStyle = $state("");
	let dismissed = $state(true);
	let openRAF: number | undefined;

	const reduceAnimations = $derived(shouldReduceAnimations());
	const openDuration = $derived(reduceAnimations ? 0.05 : 0.35);
	const closeDuration = $derived(reduceAnimations ? 0.05 : 0.25);
	const overlayDuration = $derived(reduceAnimations ? 0.05 : 0.25);

	function getClosedTranslate(): number {
		return direction === "bottom" || direction === "right" ? 100 : -100;
	}

	function transformTransition(duration: number): string {
		return `transform ${duration}s cubic-bezier(0.32, 0.72, 0, 1)`;
	}

	$effect(() => {
		releaseDrag();
		if (open) {
			dismissed = false;
			transitionStyle = "none";
			translatePct = getClosedTranslate();
			// Wait for next frame so browser paints the closed position first,
			// then CSS transitions animate it open
			openRAF = requestAnimationFrame(() => {
				transitionStyle = transformTransition(openDuration);
				translatePct = 0;
			});
			return () => cancelAnimationFrame(openRAF!);
		} else {
			transitionStyle = transformTransition(closeDuration);
			translatePct = getClosedTranslate();
		}
	});

	const transformStyle = $derived(
		isVertical
			? `translate3d(0, ${translatePct}%, 0)`
			: `translate3d(${translatePct}%, 0, 0)`,
	);

	const overlayOpacity = $derived(
		Math.max(0, 1 - Math.abs(translatePct) / 100),
	);

	let dragPointer: number | null = null;
	let dragStart = 0;
	let dragSize = 0;
	let drawerEl: HTMLDivElement = $state() as HTMLDivElement;

	// Let controls own their pointer interaction, including native range dragging.
	const interactiveSelector = [
		"input",
		"select",
		"textarea",
		"button",
		"a",
		"label",
		'[contenteditable]:not([contenteditable="false"])',
		'[role="slider"]',
		'[role="spinbutton"]',
		'[role="combobox"]',
		'[role="listbox"]',
		'[role="button"]',
		'[role="textbox"]',
		"[data-drawer-no-drag]",
	].join(",");

	function onPointerDown(e: PointerEvent) {
		if (
			!open ||
			!dismissible ||
			!e.isPrimary ||
			e.button !== 0 ||
			dragPointer !== null
		)
			return;
		if (
			e.target instanceof Element &&
			e.target.closest(interactiveSelector)
		)
			return;
		const rect = drawerEl.getBoundingClientRect();
		dragSize = isVertical ? rect.height : rect.width;
		if (dragSize <= 0) return;
		drawerEl.setPointerCapture(e.pointerId);
		dragPointer = e.pointerId;
		dragStart = isVertical ? e.clientY : e.clientX;
		transitionStyle = "none";
	}

	function onPointerMove(e: PointerEvent) {
		if (e.pointerId !== dragPointer) return;
		const current = isVertical ? e.clientY : e.clientX;
		const delta = current - dragStart;
		const sign = direction === "bottom" || direction === "right" ? 1 : -1;
		const dragged = delta * sign;
		translatePct = Math.max(0, (dragged / dragSize) * 100);
	}

	function onPointerUp(e: PointerEvent) {
		if (e.pointerId !== dragPointer) return;
		releaseDrag();

		const current = isVertical ? e.clientY : e.clientX;
		const delta = current - dragStart;
		const sign = direction === "bottom" || direction === "right" ? 1 : -1;
		if ((delta * sign) / dragSize > closeThreshold) {
			close();
		} else {
			transitionStyle = transformTransition(openDuration);
			translatePct = 0;
		}
	}

	function releaseDrag() {
		const pointer = dragPointer;
		dragPointer = null;
		if (pointer !== null && drawerEl?.hasPointerCapture(pointer)) {
			drawerEl.releasePointerCapture(pointer);
		}
	}

	function onPointerCancel(e: PointerEvent) {
		if (e.pointerId !== dragPointer) return;
		releaseDrag();
		if (open) {
			transitionStyle = transformTransition(openDuration);
			translatePct = 0;
		}
	}

	function onLostPointerCapture(e: PointerEvent) {
		// A child's implicit touch capture can be released when the Drawer takes
		// ownership. Only losing the Drawer's own capture cancels its gesture.
		if (e.target === drawerEl) onPointerCancel(e);
	}

	function onTransitionEnd(e: TransitionEvent) {
		if (
			e.target === e.currentTarget &&
			e.propertyName === "transform" &&
			!open
		) {
			dismissed = true;
		}
	}

	function close() {
		if (!dismissible) return;
		releaseDrag();
		open = false;
		onclose?.();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Escape" && open && dismissible) close();
	}

	onMount(() => window.addEventListener("keydown", onKeydown));
	onDestroy(() => {
		releaseDrag();
		window.removeEventListener("keydown", onKeydown);
	});
</script>

{#if !dismissed}
	<div
		class="drawer-overlay"
		style="opacity: {overlayOpacity}; transition: opacity {overlayDuration}s ease;"
		role="presentation"
		onclick={() => close()}
	></div>

	<div
		bind:this={drawerEl}
		class="drawer drawer--{direction} {className}"
		style="transform: {transformStyle}; transition: {transitionStyle}; {style}"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		onpointercancel={onPointerCancel}
		onlostpointercapture={onLostPointerCapture}
		ontransitionend={onTransitionEnd}
	>
		{@render children?.()}
	</div>
{/if}

<style>
	.drawer-overlay {
		position: fixed;
		inset: 0;
		background: var(--drawer-overlay, var(--bg-overlay));
		z-index: 100;
		backdrop-filter: blur(var(--backdrop-blur-overlay, 2px));
		-webkit-backdrop-filter: blur(var(--backdrop-blur-overlay, 2px));
		will-change: opacity;
	}

	.drawer {
		position: fixed;
		z-index: 101;
		background: var(--bg-sidebar);
		border-left: var(--border-width) dotted var(--border);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		will-change: transform;
		-webkit-user-select: none;
		user-select: none;
	}

	.drawer--right {
		top: 0;
		right: 0;
		height: 100%;
		width: min(var(--drawer-width, 340px), 90vw);
		border-radius: var(--border-radius-sm) 0 0 var(--border-radius-sm);
		border-right: none;
		box-shadow: var(--shadow-drawer-left);
	}

	.drawer--left {
		top: 0;
		left: 0;
		height: 100%;
		width: min(var(--drawer-width, 340px), 90vw);
		border-radius: 0 var(--border-radius-sm) var(--border-radius-sm) 0;
		border-left: none;
		box-shadow: var(--shadow-drawer-right);
	}

	.drawer--bottom {
		bottom: 0;
		left: 0;
		right: 0;
		height: auto;
		max-height: var(--drawer-max-height, 85vh);
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
		border-bottom: none;
		box-shadow: var(--shadow-drawer-top);
	}

	.drawer--top {
		top: 0;
		left: 0;
		right: 0;
		height: auto;
		max-height: var(--drawer-max-height, 85vh);
		border-radius: 0 0 var(--border-radius-sm) var(--border-radius-sm);
		border-top: none;
		box-shadow: var(--shadow-drawer-bottom);
	}
</style>
