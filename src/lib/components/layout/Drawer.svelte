<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import type { Snippet } from "svelte";

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

	function getClosedTranslate(): number {
		return direction === "bottom" || direction === "right" ? 100 : -100;
	}

	$effect(() => {
		if (open) {
			dismissed = false;
			transitionStyle = "none";
			translatePct = getClosedTranslate();
			// Wait for next frame so browser paints the closed position first,
			// then CSS transitions animate it open
			requestAnimationFrame(() => {
				transitionStyle =
					"transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
				translatePct = 0;
			});
		} else {
			transitionStyle = "transform 0.25s cubic-bezier(0.32, 0.72, 0, 1)";
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

	let isDragging = false;
	let dragStart = 0;
	let drawerEl: HTMLDivElement = $state() as HTMLDivElement;

	function onPointerDown(e: PointerEvent) {
		if (!dismissible) return;
		isDragging = true;
		dragStart = isVertical ? e.clientY : e.clientX;
		transitionStyle = "none";
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
	}

	function onPointerMove(e: PointerEvent) {
		if (!isDragging) return;
		const current = isVertical ? e.clientY : e.clientX;
		const delta = current - dragStart;
		const sign = direction === "bottom" || direction === "right" ? 1 : -1;
		const dragged = delta * sign;
		const size = isVertical
			? drawerEl.getBoundingClientRect().height
			: drawerEl.getBoundingClientRect().width;

		translatePct = Math.max(0, (dragged / size) * 100);
	}

	function onPointerUp(e: PointerEvent) {
		if (!isDragging) return;
		isDragging = false;

		const current = isVertical ? e.clientY : e.clientX;
		const delta = current - dragStart;
		const sign = direction === "bottom" || direction === "right" ? 1 : -1;
		const size = isVertical
			? drawerEl.getBoundingClientRect().height
			: drawerEl.getBoundingClientRect().width;

		if ((delta * sign) / size > closeThreshold) {
			close();
		} else {
			transitionStyle = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
			translatePct = 0;
		}
	}

	function onTransitionEnd(e: TransitionEvent) {
		if (e.propertyName === "transform" && !open) {
			dismissed = true;
		}
	}

	function close() {
		if (!dismissible) return;
		open = false;
		onclose?.();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Escape" && open && dismissible) close();
	}

	onMount(() => window.addEventListener("keydown", onKeydown));
	onDestroy(() => {
		window.removeEventListener("keydown", onKeydown);
	});
</script>

{#if !dismissed}
	<div
		class="drawer-overlay"
		style="opacity: {overlayOpacity}; transition: opacity 0.25s ease;"
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
		onpointercancel={onPointerUp}
		ontransitionend={onTransitionEnd}
	>
		{@render children?.()}
	</div>
{/if}

<style>
	.drawer-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		z-index: 100;
		backdrop-filter: blur(var(--backdrop-blur-overlay, 2px));
		-webkit-backdrop-filter: blur(var(--backdrop-blur-overlay, 2px));
	}

	.drawer {
		position: fixed;
		z-index: 101;
		background: var(--bg-sidebar);
		border-left: 1px dotted var(--border);
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
		width: min(340px, 90vw);
		border-radius: var(--border-radius-sm) 0 0 var(--border-radius-sm);
		border-right: none;
		box-shadow: -8px 0 32px rgba(0, 0, 0, 0.6);
	}

	.drawer--left {
		top: 0;
		left: 0;
		height: 100%;
		width: min(340px, 90vw);
		border-radius: 0 var(--border-radius-sm) var(--border-radius-sm) 0;
		border-left: none;
		box-shadow: 8px 0 32px rgba(0, 0, 0, 0.6);
	}

	.drawer--bottom {
		bottom: 0;
		left: 0;
		right: 0;
		height: auto;
		max-height: 85vh;
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
		border-bottom: none;
		box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.6);
	}

	.drawer--top {
		top: 0;
		left: 0;
		right: 0;
		height: auto;
		max-height: 85vh;
		border-radius: 0 0 var(--border-radius-sm) var(--border-radius-sm);
		border-top: none;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
	}
</style>
