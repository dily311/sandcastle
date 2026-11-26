// 권한 이름 → 사용자에게 보여줄 설명 (CLAUDE.md 6-D-8, ADR-012). 켜기 전 동의 창과 설치 미리보기가 쓴다.
// risk: 높을수록 앞에 보이고 색이 진하다. 모르는 권한은 가장 높게(새 앱이 모르는 권한이 조용히 지나가지 않게)
import { get } from "svelte/store";
import { _ } from "svelte-i18n";

const tr = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

export type Risk = "low" | "mid" | "high";
export interface PermissionInfo {
	label: string;
	desc: string;
	risk: Risk;
}

const KINDS = new Set(["character", "persona", "lorebook", "chat"]);

const SIMPLE_RISK: Record<string, Risk> = {
	"messages.process": "mid",
	"prompt.modify": "mid",
	"ui.panel": "low",
	"ui.command": "low",
	ai: "high",
	storage: "low",
};

export function permissionInfo(p: string): PermissionInfo {
	if (Object.hasOwn(SIMPLE_RISK, p)) {
		const slug = p.replace(".", "_");
		return {
			label: tr(`communityUi.permSimple.${slug}_label`),
			desc: tr(`communityUi.permSimple.${slug}_desc`),
			risk: SIMPLE_RISK[p],
		};
	}
	const m = /^data\.(read|write):(\w+)$/.exec(p);
	if (m && KINDS.has(m[2])) {
		const [, mode, kind] = m;
		const k = tr(`communityUi.permKinds.${kind}`);
		if (mode === "read") {
			return {
				label: tr("communityUi.permReadLabel", { kind: k }),
				desc: kind === "chat" ? tr("communityUi.permReadChatDesc") : tr("communityUi.permReadOtherDesc", { kind: k }),
				risk: kind === "chat" ? "high" : "mid",
			};
		}
		return {
			label: tr("communityUi.permWriteLabel", { kind: k }),
			desc: tr("communityUi.permWriteDesc", { kind: k }),
			risk: "mid",
		};
	}
	return { label: p, desc: tr("communityUi.permUnknownDesc"), risk: "high" };
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
