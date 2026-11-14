// 브라우저 개발 모드(pnpm dev, Tauri 없이)에서 "@tauri-apps/api/core" 대신 쓰는 가짜 모듈.
// vite.config.ts의 alias로 연결된다. Tauri 실행·빌드에서는 쓰이지 않음.
import { handlers } from "./handlers";

export interface InvokeOptions {
	headers?: Record<string, string>;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>, _options?: InvokeOptions): Promise<T> {
	const h = handlers[cmd];
	await new Promise((r) => setTimeout(r, 60)); // 실제 IPC처럼 비동기
	if (!h) {
		console.warn(`[dev-mock] 미구현 커맨드: ${cmd}`, args);
		// 실제 백엔드와 같은 모양의 오류
		throw { code: "INTERNAL_ERROR", category: "INTERNAL", message: `[mock] ${cmd} 미구현`, retryable: false };
	}
	console.info(`[dev-mock] ${cmd}`, args);
	return (await h(args ?? {})) as T;
}

/** tauri Channel 흉내: onmessage만 있으면 됨 */
export class Channel<T = unknown> {
	onmessage: (msg: T) => void = () => {};
	send(msg: T) {
		this.onmessage(msg);
	}
	toJSON() {
		return "__CHANNEL__";
	}
}

export function convertFileSrc(path: string): string {
	return path;
}

export function isTauri(): boolean {
	return false;
}
