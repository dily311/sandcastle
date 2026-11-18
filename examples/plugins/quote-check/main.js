// 따옴표 점검 — AI 응답이 저장된 직후(정규식을 거친 글)에 불린다. 스트리밍 중에는 불리지 않는다.
//
// sandcastle.on("message.process", ({ message }) => ({ content?, notes? }))
//   - notes: 응답 아래에 붙는 메모. 사용자가 고른 방식이 '표시만'이어도 붙는다
//   - content: 고친 글. 방식이 '자동 고침'일 때만 받아들여지고 원문은 남는다(원문 보기·되돌리기)
//   - 아무것도 돌려주지 않으면 그대로 둔다. 5초 안에 답해야 한다
//
// 주의: 응답에 ```html 블록을 새로 넣는 고침은 앱이 거절한다 (화면에서 실행될 수 있어서)

const FENCE = "`".repeat(3);

/** 코드 블록 안은 세지 않는다 */
function withoutCode(text) {
	return text.split(FENCE).filter((_, i) => i % 2 === 0).join("");
}

const count = (text, ch) => text.split(ch).length - 1;

sandcastle.on("message.process", ({ message }) => {
	if (message.role !== "assistant") return;
	const text = withoutCode(message.content);
	const notes = [];
	let content = message.content;

	// 곧은 큰따옴표가 홀수면 어딘가 안 닫힌 것
	if (count(text, '"') % 2 === 1) {
		notes.push({ level: "warn", text: '큰따옴표(")가 닫히지 않은 것 같습니다' });
		// 마지막이 대사로 끝나는 흔한 경우: 닫는 따옴표를 붙인다 (방식이 '자동 고침'일 때만 반영됨)
		content = content.replace(/\s*$/, '"');
	}
	// 롤플레이 지문의 별표(*)도 짝이 맞아야 기울임이 풀린다
	if (count(text, "*") % 2 === 1) {
		notes.push({ level: "info", text: "별표(*)가 홀수 개입니다 — 지문이 끝까지 기울어져 보일 수 있습니다" });
	}
	if (!notes.length) return; // 문제 없음: 아무것도 안 붙인다
	return { content, notes };
});
