// 플러그인 설정 탭(번역·음성·이미지·폰)이 함께 쓰는 도우미: 지금 값 + 한 부분만 고쳐 저장
import type { PluginSetting } from "$ipc/bindings";
import { library } from "$features/character/library.svelte";
import { pluginSetting } from "$shared/stores/aiTools.svelte";

export type Part = "translate" | "tts" | "imageGen" | "phone" | "prompts" | "sync" | "lint" | "memory" | "summary" | "rewrite";
export type Full = Required<{ [K in keyof PluginSetting]-?: Required<NonNullable<PluginSetting[K]>> }>;

export function ensureLoaded() {
	if (!pluginSetting.value) void pluginSetting.load();
	if (!library.loaded) void library.load();
}

export function current(): Full | null {
	return pluginSetting.value as Full | null;
}

export function save<K extends Part>(part: K, patch: Partial<Full[K]>) {
	const cur = current();
	if (!cur) return;
	void pluginSetting.save({ ...cur, [part]: { ...cur[part], ...patch } } as PluginSetting);
}

/** API 고르기 목록. kinds를 주면 그 종류(formatType)만 */
export function apiOptions(firstLabel = "앱에서 고른 API", kinds?: string[]) {
	const list = library.apis.filter((a) => !kinds || kinds.includes(a.formatType));
	return [{ value: "", label: firstLabel }, ...list.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))];
}

/** 채팅에 쓸 수 있는 API (음성·이미지 전용 연결은 뺌) */
export function chatApiOptions(firstLabel = "앱에서 고른 API") {
	const list = library.apis.filter((a) => a.formatType !== "elevenlabs" && a.formatType !== "novelai");
	return [{ value: "", label: firstLabel }, ...list.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))];
}

export const orNull = (v: string) => (v ? v : null);
