<script lang="ts">
	import type { SvelteHTMLElements } from "svelte/elements";
	import { themeIcons } from "$lib/api/themeManager";
	import { getIconPath, isRasterIcon } from "$lib/icons/registry";

	type SpanProps = SvelteHTMLElements["span"];

	function coerceSize(
		value: string | number | undefined,
	): string | undefined {
		if (value === undefined) return undefined;
		if (typeof value === "number") return `${value}px`;
		if (/^\d+(\.\d+)?$/.test(value)) return `${value}px`;
		return value;
	}

	let {
		class: className = "",
		src,
		name,
		size = 16,
		width,
		height,
		color,
		...rest
	}: {
		class?: string;
		src?: string;
		name?: string;
		size?: number;
		width?: string | number;
		height?: string | number;
		color?: string | null;
	} & SpanProps = $props();

	const w = $derived(coerceSize(width ?? size));
	const h = $derived(coerceSize(height ?? size));

	const customIcon = $derived.by(() => {
		if (!name) return null;
		return themeIcons.get(name) ?? null;
	});

	const resolvedSrc = $derived.by(() => {
		if (customIcon) return customIcon;
		if (src) return src;
		if (name) return getIconPath(name) ?? "";
		return "";
	});
	const raster = $derived(isRasterIcon(resolvedSrc));

	const style = $derived.by(() => {
		let base = `width: ${w}; height: ${h};`;
		if (color) base += ` color: ${color};`;
		if (raster) {
			return `${base} background-image: url("${resolvedSrc}");`;
		}
		return `${base} mask-image: url("${resolvedSrc}"); -webkit-mask-image: url("${resolvedSrc}");`;
	});
</script>

<span
	class="icon-svg {className}"
	class:icon-raster={raster}
	class:icon-mask={!raster}
	{style}
	aria-hidden="true"
	{...rest}
></span>
