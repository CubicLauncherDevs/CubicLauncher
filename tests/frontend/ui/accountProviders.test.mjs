import { expect, test } from "bun:test";
import { isElyByAccount } from "../../../src/lib/utils/accountProviders.ts";

test("Ely.by skin panel is available only for official Ely.by accounts", () => {
	for (const server of [
		"ely.by",
		"https://ely.by/",
		"https://www.ely.by/",
		"https://authserver.ely.by/",
		"http://authserver.ely.by/",
		"https://AUTHSERVER.ELY.BY/",
		"https://account.ely.by/api/authlib-injector",
	]) {
		expect(
			isElyByAccount({
				user_type: "Yggdrasil",
				yggdrasil_server_url: server,
			}),
		).toBe(true);
	}
	for (const server of [
		null,
		"",
		"https://littlesk.in/",
		"https://authserver.ely.by.example.com/",
		"https://ely.by@example.com/",
		"https://example.com/ely.by",
		"https://authserver.ely.by:8080/",
		"ftp://ely.by/",
	]) {
		expect(
			isElyByAccount({
				user_type: "Yggdrasil",
				yggdrasil_server_url: server,
			}),
		).toBe(false);
	}
});

test("Microsoft and offline accounts do not use the Ely.by skin panel", () => {
	for (const user_type of ["Microsoft", "Cracked"]) {
		expect(
			isElyByAccount({
				user_type,
				yggdrasil_server_url: "https://ely.by",
			}),
		).toBe(false);
	}
});
