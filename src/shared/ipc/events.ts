// 백엔드 이벤트 구독. Tauri 밖(브라우저 개발 모드)에서는 아무것도 안 하고 빈 해제 함수를 돌려준다
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export async function onEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
	if (!__IS_TAURI__) return () => {};
	try {
		return await listen<T>(name, (e) => handler(e.payload));
	} catch (e) {
		console.error(`[event] ${name} 구독 실패`, e);
		return () => {};
	}
}
