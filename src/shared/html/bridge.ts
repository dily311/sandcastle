// HTML 블록(iframe) → 채팅 입력칸. 허용된 요청은 fill(입력칸에 넣기)·send(보내기) 둘뿐.
// 입력칸을 가진 화면(대화·폰)이 열려 있을 때 register로 받는 쪽을 정한다. 마지막에 연 화면이 받는다.

export type HtmlAction = { action: "fill" | "send"; text: string };
type Composer = (req: HtmlAction) => void;

let composer: Composer | null = null;

/** 받는 쪽 등록. 돌려준 함수로 해제 ($effect의 정리 함수로 쓰기 좋다) */
export function registerComposer(fn: Composer): () => void {
	composer = fn;
	return () => {
		if (composer === fn) composer = null;
	};
}

/** iframe이 보낸 메시지를 확인해 요청으로 (모르는 모양이면 null) */
export function parseAction(data: unknown): HtmlAction | null {
	const d = data as { type?: unknown; action?: unknown; text?: unknown } | null;
	if (!d || d.type !== "sc-action") return null;
	if (d.action !== "fill" && d.action !== "send") return null;
	const text = typeof d.text === "string" ? d.text.slice(0, 4000).trim() : "";
	return text ? { action: d.action, text } : null;
}

/** 요청 전달. 받는 쪽이 없으면 false */
export function dispatchAction(req: HtmlAction): boolean {
	if (!composer) return false;
	composer(req);
	return true;
}
