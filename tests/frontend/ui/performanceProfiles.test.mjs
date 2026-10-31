import { expect, test } from "bun:test";
import {
	detectJvmPreset,
	detectLauncherPreset,
	hasAnyPerformanceOptimization,
	jvmPresetArgs,
	launcherPresetFlags,
	JVM_PRESETS,
	LAUNCHER_PERFORMANCE_PRESETS,
	PERFORMANCE_FLAG_KEYS,
} from "../../../src/lib/utils/performanceProfiles.ts";

test("every launcher preset is detected from its own flags", () => {
	for (const preset of LAUNCHER_PERFORMANCE_PRESETS) {
		expect(detectLauncherPreset(launcherPresetFlags(preset))).toBe(preset);
	}
});

test("partial or mixed flags are reported as custom", () => {
	expect(detectLauncherPreset({})).toBe("custom");
	expect(
		detectLauncherPreset({
			...launcherPresetFlags("balanced"),
			disable_blur_effects: true,
		}),
	).toBe("custom");
	expect(
		detectLauncherPreset({
			...launcherPresetFlags("quality"),
			reduce_animations: true,
		}),
	).toBe("custom");
});

test("performance preset enables every optimization and quality disables them", () => {
	const performance = launcherPresetFlags("performance");
	const quality = launcherPresetFlags("quality");
	for (const key of PERFORMANCE_FLAG_KEYS) {
		expect(performance[key]).toBe(true);
		expect(quality[key]).toBe(false);
	}
	expect(hasAnyPerformanceOptimization(quality)).toBe(false);
	expect(hasAnyPerformanceOptimization(performance)).toBe(true);
});

test("JVM presets are detected ignoring whitespace", () => {
	expect(detectJvmPreset("")).toBe("vanilla");
	expect(detectJvmPreset("   ")).toBe("vanilla");
	for (const preset of JVM_PRESETS) {
		const args = jvmPresetArgs(preset);
		expect(detectJvmPreset(args)).toBe(preset);
		expect(detectJvmPreset(`  ${args.replaceAll(" ", "   ")}  `)).toBe(
			preset,
		);
	}
	expect(detectJvmPreset("-Xmx4G")).toBe("custom");
});
