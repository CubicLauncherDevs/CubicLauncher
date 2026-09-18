export const INTERFACE_SCALES = [90, 100, 110, 125] as const;
export const INTERFACE_DENSITIES = ["theme", "compact", "comfortable"] as const;

export interface InterfacePreferences {
	scale: (typeof INTERFACE_SCALES)[number];
	density: (typeof INTERFACE_DENSITIES)[number];
}

export const DEFAULT_INTERFACE_PREFERENCES: Readonly<InterfacePreferences> =
	Object.freeze({ scale: 100, density: "theme" });

export function interfacePreferences(
	value?: { scale?: number; density?: string } | null,
): InterfacePreferences {
	const { scale, density } = value ?? {};
	return {
		scale: INTERFACE_SCALES.includes(scale as InterfacePreferences["scale"])
			? (scale as InterfacePreferences["scale"])
			: 100,
		density: INTERFACE_DENSITIES.includes(
			density as InterfacePreferences["density"],
		)
			? (density as InterfacePreferences["density"])
			: "theme",
	};
}

// Change only whitespace. Subtract the original gap before adding the adjusted
// gap so a virtual row's content keeps exactly the height provided by its theme.
export function densityGap(variable: string, fallback: number): string {
	return `calc(var(${variable}, ${fallback}px) * var(--cubic-interface-gap-factor, 1))`;
}

export function densityRowHeight(
	variable: string,
	fallback: number,
	gapVariable: string,
	gapFallback: number,
): string {
	return `calc(var(${variable}, ${fallback}px) - var(${gapVariable}, ${gapFallback}px) + ${densityGap(gapVariable, gapFallback)})`;
}
