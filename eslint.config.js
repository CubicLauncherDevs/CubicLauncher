import js from "@eslint/js";
import tseslint from "typescript-eslint";
import sveltePlugin from "eslint-plugin-svelte";
import globals from "globals";

export default [
	{
		ignores: [
			"target/**",
			"crates/**",
			"src-tauri/**",
			"**/node_modules/**",
			"build/**",
			".svelte-kit",
		],
	},
	js.configs.recommended,
	...tseslint.configs.recommended,
	...sveltePlugin.configs["flat/recommended"],

	{
		files: ["src/**/*.{ts,svelte}"],
		languageOptions: {
			globals: { ...globals.browser, ...globals.node, __APP_VERSION__: "readonly" },
			parser: sveltePlugin.parser,
			parserOptions: {
				parser: tseslint.parser,
				extraFileExtensions: [".svelte"],
				project: "./tsconfig.json",
			},
		},
		rules: {
			"@typescript-eslint/no-explicit-any": "warn",
			"@typescript-eslint/no-unused-vars": [
				"warn",
				{ argsIgnorePattern: "^_" },
			],
			"@typescript-eslint/consistent-type-imports": "warn",
			"svelte/no-at-html-tags": "off",
			"svelte/require-each-key": "warn",
			"svelte/button-has-type": "warn",
		},
	},

	{
		files: ["src/**/*.ts"],
		rules: {
			"@typescript-eslint/explicit-function-return-type": "off",
			"@typescript-eslint/no-require-imports": "error",
		},
	},

	{
		files: ["*.config.{js,ts}", ".*.cjs"],
		languageOptions: {
			globals: { ...globals.node },
		},
		rules: {
			"@typescript-eslint/no-require-imports": "off",
		},
	},
];
