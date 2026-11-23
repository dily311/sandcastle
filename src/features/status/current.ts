// 고정 자리에 그릴 상태창 하나 (5-F-4-2·5): 캐릭터에 상태창 틀이 있으면 변수로 채운 틀, 없으면 가장 최근 메시지의 상태창
import type { Block } from "$features/character/format";
import { library, type Conversation } from "$features/character/library.svelte";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { plugins } from "$shared/stores/plugins.svelte";
import { latestStatusBlock } from "./pin";
import { fillTemplate } from "./template";

type Msg = { id: string; role: string; content: string };

export function currentStatusBlock(conv: Conversation | undefined, messages: readonly Msg[], display: Record<string, string>, safeHtml: boolean): Block | null {
	const template = conv ? library.statusTemplates[conv.characterId] : null;
	if (conv && template) {
		// 틀은 사용자가 쓴 HTML: HTML 렌더링이 켜졌으면 ```html 블록처럼 격리된 iframe(JS·triggerSlash 됨),
		// 꺼졌으면 메시지 속 HTML처럼 거른 HTML로 바로 (스크립트 없이) — 틀이 그 플러그인에 매이지 않게
		return {
			kind: "html",
			...(plugins.on("html") ? {} : { scripts: false as const }),
			code: fillTemplate(template, {
				vars: (conv.meta as { variables?: Record<string, unknown> } | null)?.variables ?? {},
				globals: pluginSetting.value?.quickReply?.globals ?? {},
				defs: library.varDefs[conv.characterId],
				char: library.character(conv.characterId)?.name,
				user: library.persona(library.convPersonaId(conv))?.name,
			}),
		};
	}
	return latestStatusBlock(messages, display, safeHtml);
}
