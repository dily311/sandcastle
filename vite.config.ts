import { svelte } from "@sveltejs/vite-plugin-svelte";
import process from "node:process";
import path from "node:path";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(({ command }) => {
	const isTauri = process.env.TAURI_ENV_DEBUG === 'true';
	const isDev = command === 'serve';
	const baseAlias = {
		$: path.resolve(import.meta.dirname, "./src"),
		$shared: path.resolve(import.meta.dirname, "./src/shared"),
		$app: path.resolve(import.meta.dirname, "./src/app"),
		$ipc: path.resolve(import.meta.dirname, "./src/ipc"),
		$lib: path.resolve(import.meta.dirname, "./src/lib"),
		$features: path.resolve(import.meta.dirname, "./src/features"),
		$type: path.resolve(import.meta.dirname, "./src/lib/type"),
	};

	return {
		plugins: [svelte()],
		build: {
			cssCodeSplit: false,
			rollupOptions: {
				input: {
					main: path.resolve(import.meta.dirname, "index.html"),
					widget: path.resolve(import.meta.dirname, "widget.html"),
				},
			},
		},
		test: {
			environment: "jsdom",
			setupFiles: ["./src/test/setup.ts"],
			exclude: ["**/node_modules/**", "**/dist/**", "**/.claude/worktrees/**"],
		},
		resolve: {
			alias: (!isTauri && isDev)
				? {
						...baseAlias,
						"@tauri-apps/plugin-os": path.resolve(import.meta.dirname, "./src/ipc/tauri-mock.ts"),
						"@tauri-apps/api/window": path.resolve(import.meta.dirname, "./src/ipc/tauri-mock.ts"),
					}
				: baseAlias,
		},
		define: {
			__IS_TAURI__: isTauri || !isDev,
		},
		// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
		//
		// 1. prevent Vite from obscuring rust errors
		clearScreen: false,
		// 2. tauri expects a fixed port, fail if that port is not available
		server: {
			cors: true,
			port: 1420,
			strictPort: true,
			host: host || false,
			hmr: host
				? {
						protocol: "ws",
						host,
						port: 1421,
					}
				: undefined,
			watch: {
				// 3. tell Vite to ignore watching `src-tauri`
				ignored: ["**/src-tauri/**"],
			},
		},
	};
});
