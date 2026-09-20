import { expect, test } from "bun:test";
import {
	isElyByAccount,
	getAccountProvider,
} from "../../../src/lib/utils/accountProviders.ts";

test("saved accounts identify their provider without confusing custom servers", () => {
	for (const [user_type, server, provider] of [
		["Microsoft", null, "premium"],
		["Cracked", "https://auth.cubiclauncher.org", "offline"],
		["Yggdrasil", "https://account.ely.by/api/authlib-injector", "ely"],
		["Yggdrasil", "auth.cubiclauncher.org", "cubicAuth"],
		[
			"Yggdrasil",
			"https://AUTH.CUBICLAUNCHER.ORG/api/yggdrasil",
			"cubicAuth",
		],
		[
			"Yggdrasil",
			"https://auth.cubiclauncher.org.example.com",
			"authinject",
		],
		[
			"Yggdrasil",
			"https://auth.cubiclauncher.org@other.example.com",
			"authinject",
		],
		["Yggdrasil", "https://auth.cubiclauncher.org:8443", "authinject"],
		["Yggdrasil", "ftp://auth.cubiclauncher.org", "authinject"],
		["Yggdrasil", "invalid server", "authinject"],
		["Yggdrasil", null, "authinject"],
	]) {
		expect(
			getAccountProvider({ user_type, yggdrasil_server_url: server }),
		).toBe(provider);
	}
});

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
