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
	let previous: Record<K, number> | undefined;
	const measure = () => {
		if (disposed) return;
		const values = {} as Record<K, number>;
		keys.forEach((key, index) => {
			const width = Number.parseFloat(
				getComputedStyle(probes[index]).width,
			);
			const { fallback, allowZero } = metrics[key];
			values[key] =
				Number.isFinite(width) && (allowZero ? width >= 0 : width > 0)
					? width
					: fallback;
		});
		if (!previous || keys.some((key) => previous![key] !== values[key])) {
			previous = values;
			onChange(values);
		}
	};
	const observer = new ResizeObserver(measure);
	probes.forEach((probe) => observer.observe(probe));
	measure();
	return () => {
		disposed = true;
		observer.disconnect();
		host.remove();
	};
}
