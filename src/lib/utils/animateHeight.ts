import type { Action } from "svelte/action";

// The first child must wrap all content and keep its natural height.
export const animateHeight: Action<HTMLElement, number> = (
	node,
	duration = 0,
) => {
	const content = node.firstElementChild;
	if (!(content instanceof HTMLElement)) return;

	const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
	const originalHeight = node.style.height;
	let animation: Animation | null = null;
	let frame = 0;
	let previousHeight = node.getBoundingClientRect().height;

	function resize() {
		if (!duration || reducedMotion.matches) return;
		const style = getComputedStyle(node);
		const extra =
			parseFloat(style.paddingTop) +
			parseFloat(style.paddingBottom) +
			parseFloat(style.borderTopWidth) +
			parseFloat(style.borderBottomWidth);
		const maxHeight = parseFloat(style.maxHeight);
		const height = Math.min(
			content!.getBoundingClientRect().height + extra,
			Number.isFinite(maxHeight) ? maxHeight : Infinity,
		);
		if (Math.abs(height - previousHeight) < 0.5) return;

		const from = node.getBoundingClientRect().height;
		animation?.cancel();
		animation = null;
		previousHeight = height;
		node.style.height = `${height}px`;

		if (Math.abs(height - from) < 0.5) return;

		animation = node.animate(
			{ height: [`${from}px`, `${height}px`] },
			{ duration, easing: "cubic-bezier(0.25, 0.8, 0.25, 1)" },
		);
		animation.onfinish = () => {
			animation = null;
		};
	}

	function schedule() {
		if (frame) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			resize();
		});
	}

	function stop() {
		cancelAnimationFrame(frame);
		frame = 0;
		animation?.cancel();
		animation = null;
		node.style.height = originalHeight;
		previousHeight = node.getBoundingClientRect().height;
		// Hold the last measured height so content cannot jump before the next frame.
		if (duration && !reducedMotion.matches) {
			node.style.height = `${previousHeight}px`;
		}
	}

	stop();
	const observer = new ResizeObserver(schedule);
	observer.observe(content);
	window.addEventListener("resize", schedule);
	reducedMotion.addEventListener("change", stop);

	return {
		update(value) {
			if (duration === value) return;
			duration = value;
			stop();
		},
		destroy() {
			observer.disconnect();
			window.removeEventListener("resize", schedule);
			reducedMotion.removeEventListener("change", stop);
			animation?.cancel();
			cancelAnimationFrame(frame);
			node.style.height = originalHeight;
		},
	};
};
