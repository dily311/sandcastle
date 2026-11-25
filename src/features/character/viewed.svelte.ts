// 대화를 마지막으로 연 시각 (5-E-8 "최근 본" 정렬). 이 기기의 화면 기록이라 브라우저 저장소에만 둔다 (동기화하지 않음)
const KEY = "sc.convs.viewed";
const MAX = 500;

class Viewed {
	at = $state<Record<string, number>>(read());

	mark(convId: string) {
		const next = { ...this.at, [convId]: Date.now() };
		// 오래된 것부터 버려 크기를 묶는다
		const keep = Object.entries(next)
			.sort((a, b) => b[1] - a[1])
			.slice(0, MAX);
		this.at = Object.fromEntries(keep);
		try {
			localStorage.setItem(KEY, JSON.stringify(this.at));
		} catch {
			/* 못 쓰면 이번 실행 동안만 */
		}
	}
}

function read(): Record<string, number> {
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? "{}") ?? {};
	} catch {
		return {};
	}
}

export const viewed = new Viewed();
