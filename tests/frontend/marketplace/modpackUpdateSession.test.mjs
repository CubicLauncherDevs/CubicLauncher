import { expect, mock, test } from "bun:test";
import { createModpackUpdateSession } from "../../../src/lib/utils/modpackUpdateSession";

function deferred() {
	let resolve, reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

function fixture() {
	const api = {
		preview: mock(async () => ({
			token: "stage-a",
			conflicts: ["mods/custom.jar"],
		})),
		apply: mock(async () => {}),
		cancel: mock(async () => {}),
	};
	const cleanupError = mock();
	const session = createModpackUpdateSession("instance-a", api, cleanupError);
	return { api, session, cleanupError };
}

test("changing preview and closing releases each owned stage only once", async () => {
	const { api, session } = fixture();
	await session.preview("v2", null);
	session.clear();
	api.preview.mockResolvedValueOnce({ token: "stage-b", conflicts: [] });
	await session.preview(null, "/pack.zip");
	session.clear();
	session.dispose();
	expect(api.cancel.mock.calls).toEqual([
		["instance-a", "stage-a"],
		["instance-a", "stage-b"],
	]);
});

test("late preview responses are released after unmount and after a newer preview", async () => {
	for (const close of [false, true]) {
		const { api, session } = fixture();
		const late = deferred();
		api.preview.mockImplementationOnce(() => late.promise);
		const pending = session.preview("v2", null);
		if (close) session.dispose();
		else await session.preview("v3", null);
		late.resolve({ token: "late", conflicts: [] });
		expect(await pending).toBeNull();
		expect(api.cancel.mock.calls).toEqual([["instance-a", "late"]]);
		if (!close) {
			await session.apply("stage-a", {});
			expect(api.apply).toHaveBeenCalledWith("instance-a", "stage-a", {});
		}
	}
});

test("unmount does not cancel in-flight apply or its consumed stage", async () => {
	const { api, session } = fixture();
	const applied = deferred();
	api.apply.mockImplementationOnce(() => applied.promise);
	await session.preview("v2", null);
	const pending = session.apply("stage-a", { "mods/custom.jar": "keep" });
	session.dispose();
	expect(api.cancel).not.toHaveBeenCalled();
	applied.resolve();
	await pending;
	session.clear();
	expect(api.cancel).not.toHaveBeenCalled();
	expect(api.apply).toHaveBeenCalledWith("instance-a", "stage-a", {
		"mods/custom.jar": "keep",
	});
});

test("abandoned apply only releases staging once the backend rejects it", async () => {
	const { api, session } = fixture();
	const applied = deferred();
	api.apply.mockImplementationOnce(() => applied.promise);
	await session.preview("v2", null);
	const pending = session.apply("stage-a", {});
	const rejection = pending.catch((error) => error);
	session.dispose();
	expect(api.cancel).not.toHaveBeenCalled();
	applied.reject(Error("conflict"));
	expect((await rejection).message).toBe("conflict");
	expect(api.cancel.mock.calls).toEqual([["instance-a", "stage-a"]]);
});

test("visible apply errors retain staging for retry; success needs no cancellation", async () => {
	const { api, session } = fixture();
	await session.preview("v2", null);
	api.apply.mockRejectedValueOnce(Error("retryable"));
	await expect(session.apply("stage-a", {})).rejects.toThrow("retryable");
	expect(api.cancel).not.toHaveBeenCalled();
	await session.apply("stage-a", { "mods/custom.jar": "replace" });
	session.dispose();
	expect(api.cancel).not.toHaveBeenCalled();
});

test("cleanup failures are handled, and cleared tokens cannot be applied", async () => {
	const { api, session, cleanupError } = fixture();
	const error = Error("cleanup failed");
	api.cancel.mockRejectedValueOnce(error);
	await session.preview("v2", null);
	session.clear();
	await expect(session.apply("stage-a", {})).rejects.toThrow(
		"no longer available",
	);
	await Promise.resolve();
	expect(cleanupError).toHaveBeenCalledWith(error);
	expect(api.apply).not.toHaveBeenCalled();
});
