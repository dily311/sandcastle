// 권한 이름 → 사용자에게 보여줄 설명 (CLAUDE.md 6-D-8, ADR-012). 켜기 전 동의 창과 설치 미리보기가 쓴다.
// risk: 높을수록 앞에 보이고 색이 진하다. 모르는 권한은 가장 높게(새 앱이 모르는 권한이 조용히 지나가지 않게)
export type Risk = "low" | "mid" | "high";
export interface PermissionInfo {
	label: string;
	desc: string;
	risk: Risk;
}

const KIND: Record<string, string> = { character: "캐릭터", persona: "페르소나", lorebook: "로어북", chat: "대화" };

const SIMPLE: Record<string, PermissionInfo> = {
	"messages.process": { label: "AI 응답 후처리", desc: "새 AI 응답을 읽고 메모를 붙입니다. '자동 고침'을 고르면 글을 바꿀 수도 있습니다 (원문은 남고 되돌릴 수 있음)", risk: "mid" },
	"prompt.modify": { label: "보내는 프롬프트에 글 더하기", desc: "AI에게 보내기 전에 규칙이나 안내 글을 더합니다. AI의 답에 영향을 줍니다 (있는 프롬프트를 고치거나 빼지는 못함)", risk: "mid" },
	"ui.panel": { label: "오른쪽 패널에 화면 추가", desc: "오른쪽 패널에 이 플러그인의 화면을 보여줍니다", risk: "low" },
	"ui.command": { label: "명령 추가", desc: "타이틀바의 플러그인 명령 메뉴에 항목을 더합니다. 누를 때만 실행됩니다", risk: "low" },
	ai: { label: "AI 호출", desc: "연결된 AI API로 글을 만듭니다. 요금이 들 수 있습니다 (예산 한도 안에서, 실행 기록에 남음)", risk: "high" },
	storage: { label: "자기 저장소", desc: "이 플러그인만 쓰는 작은 저장 공간 (최대 1MB)", risk: "low" },
};

export function permissionInfo(p: string): PermissionInfo {
	const simple = SIMPLE[p];
	if (simple) return simple;
	const m = /^data\.(read|write):(\w+)$/.exec(p);
	if (m && KIND[m[2]]) {
		const [, mode, kind] = m;
		const k = KIND[kind];
		if (mode === "read") {
			return { label: `${k} 읽기`, desc: kind === "chat" ? "대화의 메시지 내용을 읽습니다" : `${k}의 글(이름·설명 등)을 읽습니다. 파일 경로는 보지 못합니다`, risk: kind === "chat" ? "high" : "mid" };
		}
		return { label: `${k} 고치기 제안`, desc: `${k}을(를) 고치자고 제안합니다. 사용자가 바뀌는 부분을 보고 승인해야만 바뀌고, 되돌릴 수 있습니다`, risk: "mid" };
	}
	return { label: p, desc: "이 앱이 설명을 모르는 권한입니다", risk: "high" };
}

const ORDER: Record<Risk, number> = { high: 0, mid: 1, low: 2 };

/** 위험한 것부터 */
export function sortPermissions(perms: readonly string[]): string[] {
	return [...perms].sort((a, b) => ORDER[permissionInfo(a).risk] - ORDER[permissionInfo(b).risk] || a.localeCompare(b));
}

export function highestRisk(perms: readonly string[]): Risk | null {
	if (!perms.length) return null;
	return perms.reduce<Risk>((r, p) => (ORDER[permissionInfo(p).risk] < ORDER[r] ? permissionInfo(p).risk : r), "low");
}
