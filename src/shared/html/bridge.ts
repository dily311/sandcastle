// HTML 블록(iframe) → 채팅 입력칸. 허용된 요청은 fill(입력칸에 넣기)·send(보내기)·slash(슬래시 커맨드 실행, triggerSlash) 셋뿐.
// 입력칸을 가진 화면(대화·폰)이 열려 있을 때 register로 받는 쪽을 정한다. 마지막에 연 화면이 받고,
// 그 화면이 닫히면 앞에 열려 있던 화면이 다시 받는다 (예: 대화 화면 위에 폰을 열었다 닫음).

export type HtmlAction = { action: "fill" | "send" | "slash"; text: string };
type Composer = (req: HtmlAction) => void;

const composers: Composer[] = [];

/** 받는 쪽 등록. 돌려준 함수로 해제 ($effect의 정리 함수로 쓰기 좋다) */
export function registerComposer(fn: Composer): () => void {
	composers.push(fn);
	return () => {
		const i = composers.lastIndexOf(fn);
		if (i >= 0) composers.splice(i, 1);
	};
}

/** iframe이 보낸 메시지를 확인해 요청으로 (모르는 모양이면 null) */
export function parseAction(data: unknown): HtmlAction | null {
	const d = data as { type?: unknown; action?: unknown; text?: unknown } | null;
	if (!d || d.type !== "sc-action") return null;
	if (d.action !== "fill" && d.action !== "send" && d.action !== "slash") return null;
	const text = typeof d.text === "string" ? d.text.slice(0, d.action === "slash" ? 20000 : 4000).trim() : "";
	// slash는 커맨드로 시작해야 한다 (아니면 메시지로 보내질 수 있어 거른다)
	if (d.action === "slash" && !/^\/[A-Za-z:#*]/.test(text)) return null;
	return text ? { action: d.action, text } : null;
}

/** 요청 전달. 받는 쪽이 없으면 false */
export function dispatchAction(req: HtmlAction): boolean {
	const composer = composers.at(-1);
	if (!composer) return false;
	composer(req);
	return true;
}
