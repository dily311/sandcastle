// 고정 자리에 그릴 상태창들 (5-F-4-2·5, 5-F-5-2): 캐릭터의 상태창 틀(변수로 채움)이 맨 위, 그 아래 출처마다 가장 최근 메시지의 상태창
import type { Block } from "$features/character/format";
import { library, type Conversation } from "$features/character/library.svelte";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { plugins } from "$shared/stores/plugins.svelte";
import { latestStatusBlocks } from "./pin";
import { fillTemplate } from "./template";

type Msg = { id: string; role: string; content: string };

/** vars: 틀을 채울 변수 (없으면 대화의 지금 값 — 읽기 화면은 지난 시점의 값을 줌, 5-E-6-1)
 *  allowedPinKeys: 표시할 출처 집합. 없으면 모두 (기존 동작). 있으면 그 출처만. */
export function currentStatusBlocks(conv: Conversation | undefined, messages: readonly Msg[], display: Record<string, string>, safeHtml: boolean, vars?: Record<string, unknown>, allowedPinKeys?: ReadonlySet<string>): Block[] {
	const fromMessages: Block[] = latestStatusBlocks(messages, display, safeHtml, allowedPinKeys);
	const template = conv ? library.statusTemplates[conv.characterId] : null;
	if (conv && template) {
		// 틀은 사용자가 쓴 HTML: HTML 렌더링이 켜졌으면 ```html 블록처럼 격리된 iframe(JS·triggerSlash 됨),
		// 꺼졌으면 메시지 속 HTML처럼 거른 HTML로 바로 (스크립트 없이) — 틀이 그 플러그인에 매이지 않게
		const filled: Block = {
			kind: "html",
			...(plugins.on("html") ? {} : { scripts: false as const }),
			code: fillTemplate(template, {
				vars: vars ?? (conv.meta as { variables?: Record<string, unknown> } | null)?.variables ?? {},
				globals: pluginSetting.value?.quickReply?.globals ?? {},
				defs: library.varDefs[conv.characterId],
				char: library.character(conv.characterId)?.name,
				user: library.persona(library.convPersonaId(conv))?.name,
			}),
		};
		return [filled, ...fromMessages];
	}
	return fromMessages;
}
