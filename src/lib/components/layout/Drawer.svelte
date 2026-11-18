<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import type { Snippet } from "svelte";
	import { shouldReduceAnimations } from "#lib/utils/animations.js";
	import { t } from "#lib/i18n/index.js";

	type Direction = "bottom" | "top" | "left" | "right";

	interface Props {
		open?: boolean;
		onclose?: () => void;
		dismissible?: boolean;
		direction?: Direction;
		closeThreshold?: number;
		resizable?: boolean;
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
		resizable = false,
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
		resizePointer = null;
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
	let drawerWidth = $state<number | null>(null);
	let resizePointer: number | null = null;
	let resizeStartX = 0;
	let resizeStartWidth = 0;
	let resizeNextWidth = 0;

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

	function clampDrawerWidth(width: number) {
		const max = Math.min(760, window.innerWidth * 0.9);
		const min = Math.min(300, max);
		return Math.min(max, Math.max(min, width));
	}

	function onResizePointerDown(e: PointerEvent) {
		if (!resizable || isVertical || !open || resizePointer !== null) return;
		e.preventDefault();
		e.stopPropagation();
		const handle = e.currentTarget as HTMLElement;
		resizePointer = e.pointerId;
		resizeStartX = e.clientX;
		resizeStartWidth = drawerEl.getBoundingClientRect().width;
		resizeNextWidth = resizeStartWidth;
		transitionStyle = "none";
		handle.setPointerCapture(e.pointerId);
	}

	function onResizePointerMove(e: PointerEvent) {
		if (e.pointerId !== resizePointer) return;
		e.preventDefault();
		const directionSign = direction === "right" ? -1 : 1;
		resizeNextWidth = clampDrawerWidth(
			resizeStartWidth + (e.clientX - resizeStartX) * directionSign,
		);
		drawerEl.style.setProperty("--drawer-width", `${resizeNextWidth}px`);
	}

	function endResize(e: PointerEvent) {
		if (e.pointerId !== resizePointer) return;
		resizePointer = null;
		drawerWidth = resizeNextWidth;
		transitionStyle = transformTransition(openDuration);
	}

	function onResizeKeydown(e: KeyboardEvent) {
		if (!resizable || isVertical) return;
		if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
		e.preventDefault();
		const amount = e.key === "ArrowLeft" ? -24 : 24;
		const directionSign = direction === "right" ? -1 : 1;
		drawerWidth = clampDrawerWidth(
			drawerEl.getBoundingClientRect().width + amount * directionSign,
		);
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
		style="transform: {transformStyle}; transition: {transitionStyle}; {style}; {resizable &&
		drawerWidth !== null
			? `--drawer-width: ${drawerWidth}px;`
			: ''}"
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
		{#if resizable && !isVertical}
			<div
				class="drawer-resize-handle drawer-resize-handle--{direction}"
				data-drawer-no-drag
				role="slider"
				aria-label={t("common.resizePanel")}
				aria-orientation="horizontal"
				aria-valuemin="300"
				aria-valuemax="760"
				aria-valuenow={Math.round(drawerWidth ?? 340)}
				tabindex="0"
				onpointerdown={onResizePointerDown}
				onpointermove={onResizePointerMove}
				onpointerup={endResize}
				onpointercancel={endResize}
				onkeydown={onResizeKeydown}
			></div>
		{/if}
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

	.drawer-resize-handle {
		position: absolute;
		top: 0;
		bottom: 0;
		z-index: 2;
		display: flex;
		width: 12px;
		align-items: center;
		justify-content: center;
		cursor: col-resize;
		touch-action: none;
	}

	.drawer-resize-handle::before {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 44px;
		content: "";
	}

	.drawer-resize-handle::after {
		position: absolute;
		top: 50%;
		width: 3px;
		height: 40px;
		border-radius: var(--border-radius-pill);
		background: var(--border-color);
		content: "";
		opacity: 0.6;
		transform: translateY(-50%);
		transition:
			background 0.15s,
			opacity 0.15s;
	}

	.drawer-resize-handle:hover::after,
	.drawer-resize-handle:focus-visible::after {
		background: var(--accent);
		opacity: 1;
	}

	.drawer-resize-handle--right {
		left: 0;
	}

	.drawer-resize-handle--right::before {
		left: -16px;
	}

	.drawer-resize-handle--left {
		right: 0;
	}

	.drawer-resize-handle--left::before {
		right: -16px;
	}

	.drawer-resize-handle:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
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
