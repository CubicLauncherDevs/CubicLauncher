/** Measures theme lengths using CSS itself (including rem, calc() and var()).
 * ResizeObserver also catches font-size changes and asynchronously loaded Inject.css.
 * The clipped probes never participate in layout or enlarge a scroll container.
 */
export function observeThemeMetrics<K extends string>(
	target: HTMLElement,
	metrics: Record<
		K,
		{
			variable: string;
			fallback: number;
			allowZero?: boolean;
			expression?: string;
		}
	>,
	onChange: (values: Record<K, number>) => void,
): () => void {
	const host = document.createElement("div");
	host.setAttribute("aria-hidden", "true");
	host.style.cssText =
		"position:absolute;width:0;height:0;overflow:hidden;visibility:hidden;pointer-events:none;contain:strict;";
	const keys = Object.keys(metrics) as K[];
	const probes = keys.map((key) => {
		const probe = document.createElement("div");
		const { variable, fallback, expression } = metrics[key];
		probe.style.cssText = `display:block;box-sizing:content-box;min-width:0;max-width:none;height:0;padding:0;border:0;margin:0;width:${expression ?? `var(${variable}, ${fallback}px)`};`;
		host.appendChild(probe);
		return probe;
	});
	target.appendChild(host);
	let disposed = false;
	const normalize = (key: K, width: number) => {
		const { fallback, allowZero } = metrics[key];
		return Number.isFinite(width) && (allowZero ? width >= 0 : width > 0)
			? width
			: fallback;
	};
	let values = {} as Record<K, number>;
	const probeKeys = new Map<Element, K>();
	keys.forEach((key, index) => {
		probeKeys.set(probes[index], key);
		// One synchronous measurement on mount; subsequent sizes come directly
		// from the observer, including rem/calc() and locally scoped theme CSS.
		values[key] = normalize(
			key,
			Number.parseFloat(getComputedStyle(probes[index]).width),
		);
	});
	const observer = new ResizeObserver((entries) => {
		if (disposed) return;
		let next: Record<K, number> | undefined;
		for (const entry of entries) {
			const key = probeKeys.get(entry.target);
			if (key === undefined) continue;
			const width = normalize(key, entry.contentRect.width);
			if (width === values[key]) continue;
			next ??= { ...values };
			next[key] = width;
		}
		if (next) {
			values = next;
			onChange(next);
		}
	});
	probes.forEach((probe) => observer.observe(probe));
	onChange(values);
	return () => {
		disposed = true;
		observer.disconnect();
		host.remove();
	};
}
