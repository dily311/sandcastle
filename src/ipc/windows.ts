
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";

export const isDesktop = ["windows", "macos", "linux"].includes(platform());

// 전체화면 토글 함수
export async function toggleFullscreen() {
	const appWindow = getCurrentWindow();
	const isFullscreen = await appWindow.isFullscreen();
	await appWindow.setFullscreen(!isFullscreen);

	return isFullscreen;
}
