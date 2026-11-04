import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";

export const isDesktop = ["windows", "macos", "linux"].includes(platform());
