// 동기화 화면에서 기기를 부르는 이름 (5-E-16-1): 사용자가 붙인 이름, 없으면 id 앞 8자리
type Named = { deviceId: string; name?: string | null };

export const shortId = (id: string) => id.slice(0, 8);

/** 이름을 아는 기기 목록(동기화 상태의 peers)에서 찾고, 없으면 id 앞자리 */
export function deviceLabel(id: string, peers: readonly Named[] = [], name?: string | null): string {
	const n = name?.trim() || peers.find((p) => p.deviceId === id)?.name?.trim();
	return n || shortId(id);
}
