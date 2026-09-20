import { expect, test } from "bun:test";
import {
	suggestMigrationNames,
	validMigrationNames,
} from "../../../src/lib/utils/migrationNames.ts";

test("migration names avoid existing and batch collisions within the length limit", () => {
	const long = "A".repeat(24);
	const names = suggestMigrationNames(
		[long, long, "Survival", "Survival"],
		[long, "Survival", "Survival (2)"],
	);
	expect(names).toEqual([
		"A".repeat(20) + " (2)",
		"A".repeat(20) + " (3)",
		"Survival (3)",
		"Survival (4)",
	]);
	expect(validMigrationNames(names, [long, "Survival", "Survival (2)"])).toBe(
		true,
	);
});

test("migration validates edited names and normalized duplicates", () => {
	for (const names of [
		[],
		[""],
		["../world"],
		["world", " world "],
		["Existing"],
	]) {
		expect(validMigrationNames(names, ["Existing"])).toBe(false);
	}
	expect(validMigrationNames(["World", "Modded"], ["Existing"])).toBe(true);
});
