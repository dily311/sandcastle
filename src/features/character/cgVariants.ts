// 대화 CG 같은 키 묶기 (5-E-25): `smile.png`·`smile-1.png`…를 키 `smile` 하나로. 보일 때는 메시지마다 하나를 고른다 (다시 그려도 같은 것)

/** 끝의 `-숫자`를 뗀 묶음 키 (백엔드 chat_images::base_key와 같은 규칙) */
export function baseKey(key: string): string {
	const m = /^(.+)-(\d+)$/.exec(key);
	return m ? m[1] : key;
}

/** 키 → 그림 주소들. group이면 묶음 키로 모으고 원래 키로도 찾을 수 있게 둔다 */
export function cgVariants(items: { key: string; src: string | null }[], group: boolean): Record<string, string[]> {
	const out: Record<string, string[]> = {};
	for (const { key, src } of items) {
		if (!src) continue;
		(out[key] ??= []).push(src);
		const b = baseKey(key);
		if (group && b !== key) (out[b] ??= []).push(src);
	}
	return out;
}

function hash(s: string): number {
	let h = 2166136261;
	for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
	return h >>> 0;
}

/** 이 메시지에서 보일 그림: 묶음이면 메시지 id·키로 하나를 고른다 */
export function pickVariants(variants: Record<string, string[]>, seed: string): Record<string, string> {
	return Object.fromEntries(Object.entries(variants).map(([k, list]) => [k, list.length > 1 ? list[hash(`${seed}:${k}`) % list.length] : list[0]]));
}
