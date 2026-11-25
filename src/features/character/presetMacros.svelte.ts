// 대화 유틸 지시문은 프리셋 블록의 매크로 자리에만 들어간다 (5-F-7-1, ADR-030 추가).
// 기능 스위치 밑에 "지금 프리셋에 {{key}}가 없음"을 알리려고, 프리셋마다 켜진 블록 글을 한 번 읽어 둔다.
import { call, commands } from "$shared/ipc/client";
import { appSetting } from "$shared/stores/appSetting.svelte";
import { library, type Conversation } from "./library.svelte";

/** 유틸 지시문 매크로 (백엔드 prompt_manager의 img_inprompt + UTIL_MACROS) */
export type UtilMacro = "img_inprompt" | "multi_char" | "auto_image" | "vars";

/** 이 대화에 쓰이는 프리셋 (대화 → 캐릭터 기본 → 앱 기본) */
export function presetOf(conv: Conversation): string | null {
	return conv.promptPresetId || library.character(conv.characterId)?.defaultPromptPresetId || appSetting.value?.selectedPreset || null;
}

class PresetMacros {
	/** 프리셋 id → 켜진 블록 글 (소문자) */
	private texts = $state<Record<string, string>>({});
	private loading = new Set<string>();

	private load(id: string) {
		if (this.loading.has(id)) return;
		this.loading.add(id);
		call(commands.getPromptPreset(id), { source: "preset", silent: true })
			.then((d) => {
				const text = d.blocks
					.filter((b) => b.enabled)
					.map((b) => b.content)
					.join("\n")
					.toLowerCase();
				this.texts = { ...this.texts, [id]: text };
			})
			.catch(() => {})
			.finally(() => this.loading.delete(id));
	}

	/** 프리셋을 저장하면 다시 읽게 */
	forget(id: string) {
		const { [id]: _, ...rest } = this.texts;
		this.texts = rest;
	}

	/** 그 프리셋에 매크로가 없다고 확인됐을 때만 true (아직 못 읽었거나 프리셋이 없으면 false — 앱 기본 배치에는 자리가 있음) */
	missing(presetId: string | null, key: UtilMacro): boolean {
		if (!presetId) return false;
		const text = this.texts[presetId];
		if (text === undefined) {
			this.load(presetId);
			return false;
		}
		return !text.includes(`{{${key}}}`);
	}
}
export const presetMacros = new PresetMacros();
