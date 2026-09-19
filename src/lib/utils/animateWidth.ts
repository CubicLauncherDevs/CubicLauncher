import type { Action } from "svelte/action";

// The first child must keep its natural width while the outer element resizes.
export const animateWidth: Action<HTMLElement, number> = (
	node,
	duration = 0,
) => {
	const content = node.firstElementChild;
	if (!(content instanceof HTMLElement)) return;

	const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
	const originalWidth = node.style.width;
	let animation: Animation | null = null;
	let previousWidth = 0;

	function resize() {
		if (!duration || reducedMotion.matches) return;
		const style = getComputedStyle(node);
		const extra =
			parseFloat(style.paddingLeft) +
			parseFloat(style.paddingRight) +
			parseFloat(style.borderLeftWidth) +
			parseFloat(style.borderRightWidth);
		const width = Math.max(
			content!.getBoundingClientRect().width + extra,
			parseFloat(style.minWidth) || 0,
		);
		if (Math.abs(width - previousWidth) < 0.5) return;

		const from = node.getBoundingClientRect().width;
		animation?.cancel();
		animation = null;
		previousWidth = width;
		node.style.width = `${width}px`;
		if (Math.abs(width - from) < 0.5) return;

		animation = node.animate(
			{ width: [`${from}px`, `${width}px`] },
			{ duration, easing: "cubic-bezier(0.25, 0.8, 0.25, 1)" },
		);
		animation.onfinish = () => {
			animation = null;
		};
	}

	function reset() {
		animation?.cancel();
		animation = null;
		node.style.width = originalWidth;
		previousWidth = node.getBoundingClientRect().width;
		if (duration && !reducedMotion.matches) {
			node.style.width = `${previousWidth}px`;
		}
	}

	reset();
	const observer = new ResizeObserver(resize);
	observer.observe(content);
	window.addEventListener("resize", resize);
	reducedMotion.addEventListener("change", reset);

	return {
		update(value) {
			if (duration === value) return;
			duration = value;
			reset();
		},
		destroy() {
			observer.disconnect();
			window.removeEventListener("resize", resize);
			reducedMotion.removeEventListener("change", reset);
			animation?.cancel();
			node.style.width = originalWidth;
		},
	};
};
