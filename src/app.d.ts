declare const __APP_VERSION__: string;

declare module "@static/images/*.svg?raw" {
	const content: string;
	export default content;
}
