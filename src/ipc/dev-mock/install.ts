// 브라우저 개발 모드(pnpm dev, Tauri 없이)용 가짜 백엔드.
// 진짜 @tauri-apps/api가 부르는 window.__TAURI_INTERNALS__를 흉내 낸다 → 생성된 바인딩·Channel·convertFileSrc가 그대로 동작.
// main.ts가 개발 모드이고 Tauri가 아닐 때만 불러온다 (빌드에는 들어가지 않음).
import { handlers } from "./handlers";

const callbacks = new Map<number, (v: unknown) => void>();
let nextId = 1;

async function invoke(cmd: string, args: Record<string, unknown> = {}) {
	const h = handlers[cmd];
	await new Promise((r) => setTimeout(r, 60)); // 실제 IPC처럼 비동기
	if (!h) {
		console.warn(`[dev-mock] 미구현 커맨드: ${cmd}`, args);
		// 실제 백엔드와 같은 모양의 오류
		throw { code: "INTERNAL_ERROR", category: "INTERNAL", message: `[mock] ${cmd} 미구현`, retryable: false };
	}
	console.info(`[dev-mock] ${cmd}`, args);
	return h(args);
}

(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
	invoke,
	transformCallback(cb: (v: unknown) => void) {
		const id = nextId++;
		callbacks.set(id, cb);
		return id;
	},
	unregisterCallback(id: number) {
		callbacks.delete(id);
	},
	convertFileSrc(path: string) {
		return path;
	},
	metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
};
