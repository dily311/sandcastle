// 제안 전·후 글의 줄 단위 차이 (6-D-3). 승인 화면이 바뀌는 줄만 눈에 띄게 보여준다.
export type DiffLine = { kind: "same" | "del" | "add"; text: string };
/** 접은 같은 줄 묶음 */
export type DiffItem = DiffLine | { kind: "skip"; count: number };

/** 이보다 크면 줄 비교를 하지 않고 통째로 지움/더함으로 (LCS 표 크기) */
const MAX_CELLS = 2_000_000;

export function diffLines(before: string, after: string): DiffLine[] {
	const a = before === "" ? [] : before.split("\n");
	const b = after === "" ? [] : after.split("\n");
	if (a.length * b.length > MAX_CELLS) {
		return [...a.map((text) => ({ kind: "del" as const, text })), ...b.map((text) => ({ kind: "add" as const, text }))];
	}
	// LCS 길이 표 (뒤에서부터)
	const n = a.length;
	const m = b.length;
	const t: Uint32Array[] = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
	for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) t[i][j] = a[i] === b[j] ? t[i + 1][j + 1] + 1 : Math.max(t[i + 1][j], t[i][j + 1]);
	const out: DiffLine[] = [];
	let i = 0;
	let j = 0;
	while (i < n && j < m) {
		if (a[i] === b[j]) {
			out.push({ kind: "same", text: a[i] });
			i++;
			j++;
		} else if (t[i + 1][j] >= t[i][j + 1]) out.push({ kind: "del", text: a[i++] });
		else out.push({ kind: "add", text: b[j++] });
	}
	while (i < n) out.push({ kind: "del", text: a[i++] });
	while (j < m) out.push({ kind: "add", text: b[j++] });
	return out;
}

/** 바뀐 줄 앞뒤 context줄만 남기고 같은 줄은 접는다 (긴 설명에서 바뀐 곳만 보이게) */
export function fold(lines: DiffLine[], context = 2): DiffItem[] {
	const keep = lines.map(() => false);
	lines.forEach((l, i) => {
		if (l.kind === "same") return;
		for (let k = Math.max(0, i - context); k <= Math.min(lines.length - 1, i + context); k++) keep[k] = true;
	});
	const out: DiffItem[] = [];
	let skipped = 0;
	lines.forEach((l, i) => {
		if (keep[i]) {
			if (skipped) out.push({ kind: "skip", count: skipped });
			skipped = 0;
			out.push(l);
		} else skipped++;
	});
	if (skipped) out.push({ kind: "skip", count: skipped });
	return out;
}

/** 칸 값을 보여줄 글로 (배열은 쉼표로, 참/거짓은 켜짐/꺼짐) */
export function show(v: unknown): string {
	if (v === null || v === undefined) return "";
	if (Array.isArray(v)) return v.map(String).join(", ");
	if (typeof v === "boolean") return v ? "켜짐" : "꺼짐";
	return String(v);
}
