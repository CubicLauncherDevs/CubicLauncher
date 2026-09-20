import type { MinecraftUser } from "$lib/types/types";

export type AccountProvider =
	"offline" | "premium" | "ely" | "cubicAuth" | "authinject";

export const ACCOUNT_PROVIDERS: AccountProvider[] = [
	"offline",
	"premium",
	"ely",
	"cubicAuth",
	"authinject",
];

export const ACCOUNT_SERVER_PRESETS: Partial<Record<AccountProvider, string>> =
	{
		ely: "https://account.ely.by/api/authlib-injector",
		cubicAuth: "https://auth.cubiclauncher.org",
	};

export function getAccountProvider(
	user: Pick<MinecraftUser, "user_type" | "yggdrasil_server_url">,
): AccountProvider {
	if (user.user_type === "Microsoft") return "premium";
	if (user.user_type !== "Yggdrasil") return "offline";
	if (isElyByAccount(user)) return "ely";
	try {
		const server = user.yggdrasil_server_url?.trim() ?? "";
		const url = new URL(
			server.includes("://") ? server : `https://${server}`,
		);
		if (
			["http:", "https:"].includes(url.protocol) &&
			!url.port &&
			url.hostname === "auth.cubiclauncher.org"
		)
			return "cubicAuth";
	} catch {
		// Unknown servers keep the generic provider label.
	}
	return "authinject";
}

export function isElyByAccount(
	user: Pick<MinecraftUser, "user_type" | "yggdrasil_server_url">,
): boolean {
	if (user.user_type !== "Yggdrasil" || !user.yggdrasil_server_url) {
		return false;
	}
	try {
		const server = user.yggdrasil_server_url.trim();
		const url = new URL(
			server.includes("://") ? server : `https://${server}`,
		);
		return (
			["http:", "https:"].includes(url.protocol) &&
			!url.port &&
			[
				"ely.by",
				"www.ely.by",
				"authserver.ely.by",
				"account.ely.by",
			].includes(url.hostname)
		);
	} catch {
		return false;
	}
}
