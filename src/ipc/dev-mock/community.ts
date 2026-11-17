// 커뮤니티 플러그인 가짜 응답 (6-D-1). 브라우저 미리보기에는 파일이 없으므로 고른 경로와 상관없이
// 예제 플러그인 하나를 미리보기로 보여 주고, 설치·켜기·지우기는 메모리 목록만 바꾼다
import type { PluginManifest_Serialize as PluginManifest, PluginPreview_Serialize as PluginPreview, TrashItem, UserPlugin_Serialize as UserPlugin } from "../bindings";

type Args = Record<string, unknown>;
type Handler = (args: Args) => unknown;

const sample: PluginManifest = {
	id: "word-count",
	name: "글자 수",
	version: "1.0.0",
	apiVersion: 1,
	author: "예제",
	description: "오른쪽 패널에 이 대화의 글자 수를 보여줍니다",
	main: "main.js",
	permissions: ["data.read:chat", "ui.panel"],
	settings: [],
};

const plugins: UserPlugin[] = [
	{
		id: "tidy-quotes",
		manifest: {
			id: "tidy-quotes",
			name: "따옴표 정리",
			version: "0.3.1",
			apiVersion: 1,
			author: "예제",
			description: "AI 응답의 짝이 안 맞는 따옴표를 표시합니다",
			main: "main.js",
			permissions: ["messages.process"],
			settings: [{ key: "fix", type: "toggle", label: "자동으로 고치기", default: false }],
		},
		error: null,
		enabled: false,
		granted: [],
		installedAt: Date.now() - 3 * 86_400_000,
		size: 8_200,
	},
];

const find = (id: unknown) => {
	const p = plugins.find((x) => x.id === id);
	if (!p) throw { code: "RESOURCE_NOT_FOUND", message: `설치된 플러그인이 아님: ${String(id)}` };
	return p;
};

export const communityHandlers: Record<string, Handler> = {
	list_user_plugins: () => plugins.map((p) => ({ ...p })),
	preview_user_plugin: (): PluginPreview => {
		const existing = plugins.find((p) => p.id === sample.id);
		return {
			manifest: sample,
			files: 3,
			size: 5_400,
			hash: "mock-hash",
			existing: existing ? { version: existing.manifest?.version ?? "", enabled: existing.enabled } : null,
			addedPermissions: existing ? sample.permissions.filter((x) => !existing.granted.includes(x)) : [...sample.permissions],
			downgrade: false,
		};
	},
	install_user_plugin: () => {
		const old = plugins.findIndex((p) => p.id === sample.id);
		const p: UserPlugin = { id: sample.id, manifest: sample, error: null, enabled: false, granted: [], installedAt: Date.now(), size: 5_400 };
		if (old >= 0) plugins.splice(old, 1, p);
		else plugins.push(p);
		return { ...p };
	},
	set_user_plugin_enabled: ({ pluginId, enabled, grant }) => {
		const p = find(pluginId);
		const want = [...(p.manifest?.permissions ?? [])].sort().join();
		if (enabled && [...((grant as string[]) ?? [])].sort().join() !== want) {
			throw { code: "INVALID_INPUT", message: "플러그인이 요청한 권한 전부에 동의해야 켤 수 있음" };
		}
		p.enabled = Boolean(enabled);
		if (enabled) p.granted = [...(p.manifest?.permissions ?? [])];
		return { ...p };
	},
	uninstall_user_plugin: ({ pluginId }): TrashItem => {
		const p = find(pluginId);
		plugins.splice(plugins.indexOf(p), 1);
		return { id: `t-plugin-${p.id}`, kind: "user_plugin", itemId: p.id, label: p.manifest?.name ?? p.id, deletedAt: Date.now(), rows: 1, files: 1, size: p.size };
	},
};
