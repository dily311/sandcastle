// 시각 표시: 목록·메시지에는 상대 시각, 툴팁에는 전체 날짜

/** ms 타임스탬프 → "방금", "3분 전", "2시간 전", "4일 전", "2달 전", "1년 전" */
export function relative(at: number, now = Date.now()): string {
	const min = Math.floor((now - at) / 60_000);
	if (min < 1) return "방금";
	if (min < 60) return `${min}분 전`;
	const h = Math.floor(min / 60);
	if (h < 24) return `${h}시간 전`;
	const d = Math.floor(h / 24);
	if (d < 30) return `${d}일 전`;
	const mo = Math.floor(d / 30);
	if (mo < 12) return `${mo}달 전`;
	return `${Math.floor(mo / 12)}년 전`;
}

/** 툴팁용 전체 날짜·시각 */
export function absolute(at: number): string {
	return new Date(at).toLocaleString("ko-KR", { dateStyle: "full", timeStyle: "medium" });
}
