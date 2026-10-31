export const PERFORMANCE_FLAG_KEYS = [
	"reduce_animations",
	"disable_blur_effects",
	"disable_infinite_animations",
	"disable_skin3d_animations",
	"reduce_log_animations",
	"disable_background_image",
	"reduce_skin3d_quality",
] as const;

export type PerformanceFlagKey = (typeof PERFORMANCE_FLAG_KEYS)[number];
export type PerformanceFlags = Record<PerformanceFlagKey, boolean>;

export const LAUNCHER_PERFORMANCE_PRESETS = [
	"quality",
	"balanced",
	"performance",
] as const;

export type LauncherPerformancePreset =
	(typeof LAUNCHER_PERFORMANCE_PRESETS)[number];
export type LauncherPerformanceSelection = LauncherPerformancePreset | "custom";

const PRESET_FLAGS: Record<LauncherPerformancePreset, PerformanceFlags> = {
	quality: {
		reduce_animations: false,
		disable_blur_effects: false,
		disable_infinite_animations: false,
		disable_skin3d_animations: false,
		reduce_log_animations: false,
		disable_background_image: false,
		reduce_skin3d_quality: false,
	},
	balanced: {
		reduce_animations: true,
		disable_blur_effects: false,
		disable_infinite_animations: true,
		disable_skin3d_animations: false,
		reduce_log_animations: true,
		disable_background_image: false,
		reduce_skin3d_quality: false,
	},
	performance: {
		reduce_animations: true,
		disable_blur_effects: true,
		disable_infinite_animations: true,
		disable_skin3d_animations: true,
		reduce_log_animations: true,
		disable_background_image: true,
		reduce_skin3d_quality: true,
	},
};

/** Copia inmutable de las opciones que aplica un preset del launcher. */
export function launcherPresetFlags(
	preset: LauncherPerformancePreset,
): Readonly<PerformanceFlags> {
	return PRESET_FLAGS[preset];
}

/** Detecta el preset activo o `custom` si la combinación no coincide. */
export function detectLauncherPreset(
	flags: Partial<PerformanceFlags>,
): LauncherPerformanceSelection {
	for (const preset of LAUNCHER_PERFORMANCE_PRESETS) {
		const expected = PRESET_FLAGS[preset];
		if (PERFORMANCE_FLAG_KEYS.every((key) => flags[key] === expected[key]))
			return preset;
	}
	return "custom";
}

/** `true` si el usuario no tiene ninguna optimización del launcher activa. */
export function hasAnyPerformanceOptimization(
	flags: Partial<PerformanceFlags>,
): boolean {
	return PERFORMANCE_FLAG_KEYS.some((key) => flags[key] === true);
}

export const JVM_PRESETS = ["vanilla", "recommended", "aikar"] as const;

export type JvmPreset = (typeof JVM_PRESETS)[number];
export type JvmPresetSelection = JvmPreset | "custom";

/**
 * Los presets no incluyen `-Xmx`/`-Xms`: la memoria se controla desde los
 * controles de RAM para que ambos ajustes no se contradigan.
 */
export const JVM_PRESET_ARGS: Record<JvmPreset, string> = {
	vanilla: "",
	recommended:
		"-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M",
	aikar: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1",
};

function normalizeArgs(args: string): string {
	return args.trim().split(/\s+/).filter(Boolean).join(" ");
}

/** Detecta el preset JVM activo o `custom` si hay argumentos propios. */
export function detectJvmPreset(args: string): JvmPresetSelection {
	const normalized = normalizeArgs(args);
	for (const preset of JVM_PRESETS) {
		if (normalized === normalizeArgs(JVM_PRESET_ARGS[preset]))
			return preset;
	}
	return "custom";
}

export function jvmPresetArgs(preset: JvmPreset): string {
	return JVM_PRESET_ARGS[preset];
}
