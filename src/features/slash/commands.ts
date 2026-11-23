// 슬래시 커맨드 → 앱 기능 (5-E-20-5-2·5-E-20-6-2, ADR-025). 엔진(script.ts)은 앱을 모르고, 여기서 커맨드 표를 만든다.
// 스크립트는 사용자가 입력칸에서 보내거나 빠른 답장을 눌렀을 때만 돈다.
// AI를 부르는 커맨드는 한 번 실행에 설정의 상한(기본 5, 최대 50)번까지 — 넘으면 오류로 멈춘다.
// 변수는 실행 중에 들고 있다가 끝날 때 한 번 저장한다 (반복 안에서 여러 번 바꿔도 저장은 한 번).
import { call, commands } from "$shared/ipc/client";
import { chooseFrom, confirmAction, promptText } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { library } from "$features/character/library.svelte";
import type { ChatSession } from "$features/character/session.svelte";
import { quickReplies, toInput } from "./quickReply.svelte";
import { slashRuns } from "./running.svelte";
import { ghost } from "$features/ghost/ghost.svelte";
import { plugins } from "$shared/stores/plugins.svelte";
import { abortScript, addTo, getIndexed, runScript, ScriptError, setIndexed, type ArgValue, type Handler, type ScriptEnv } from "./script";

const src = { source: "slash" };
/** 한 번 실행 AI 호출 상한 (설정이 없을 때, ADR-025 5-E-20-6-0) */
export const DEFAULT_AI_LIMIT = 5;
export const MAX_AI_LIMIT = 50;
export function aiLimit(): number {
	const v = Number((pluginSetting.value?.quickReply as { aiLimit?: number } | undefined)?.aiLimit ?? DEFAULT_AI_LIMIT);
	return Math.min(MAX_AI_LIMIT, Math.max(1, Number.isFinite(v) ? Math.round(v) : DEFAULT_AI_LIMIT));
}
/** SillyTavern에 있지만 지원하지 않는 커맨드 (ADR-025) */
const UNSUPPORTED = ["import", "fetch", "js", "api", "bg", "theme", "css", "member-add", "member-remove", "member-get", "member-count", "group", "sd", "imagine"];

/** "세트.버튼" → 그 빠른 답장의 스크립트 (세트 이름에 점이 있어도 되게 점마다 나눠 본다) */
export function quickReplySource(name: string): string | undefined {
	const n = name.trim();
	for (let i = n.indexOf("."); i > 0; i = n.indexOf(".", i + 1)) {
		const [setName, label] = [n.slice(0, i).trim(), n.slice(i + 1).trim()];
		const item = quickReplies.sets.find((s) => s.name === setName)?.items.find((q) => q.label === label);
		if (item) return item.message;
	}
	return undefined;
}

const text = (v: ArgValue | undefined) => (typeof v === "string" ? v : "");

/** "5" 또는 "3-7" → 번호(seq) 목록. 비면 마지막 메시지 */
function range(arg: string, last: number): number[] {
	const m = /^(\d+)(?:\s*-\s*(\d+))?$/.exec(arg.trim());
	if (!m) return arg.trim() ? [] : [last];
	const [a, b] = [Number(m[1]), Number(m[2] ?? m[1])];
	return Array.from({ length: Math.max(0, b - a + 1) }, (_, i) => a + i);
}

/** 실행할 때 화면에서 넘기는 것: 입력칸 글 ({{input}}·/setinput·/impersonate) */
export interface SlashOpts {
	input?: string;
	setInput?: (v: string) => void;
	/** 다시 실행할 때 앞 실행의 /input 답 (질문 → 답) */
	answers?: Record<string, string>;
}

/** "3" · "2-5" · "" (마지막) → 메시지 자리(0부터) 목록. SillyTavern의 메시지 번호와 같은 0부터 */
function indexRange(arg: string, count: number): number[] {
	const t = arg.trim();
	if (!t) return count ? [count - 1] : [];
	const m = /^(-?\d+)(?:\s*-\s*(\d+))?$/.exec(t);
	if (!m) return [];
	const fix = (n: number) => (n < 0 ? count + n : n);
	const [a, b] = [fix(Number(m[1])), fix(Number(m[2] ?? m[1]))];
	const out: number[] = [];
	for (let i = Math.max(0, a); i <= Math.min(count - 1, b); i++) out.push(i);
	return out;
}

/** 이 대화에서 스크립트 실행 환경. flush: 바뀐 변수 저장 (실행이 끝나면 부름) */
export function slashEnv(session: ChatSession, opts: SlashOpts = {}): ScriptEnv & { flush: () => Promise<void>; answers: Record<string, string> } {
	const convId = session.convId;
	const conv = () => library.conversation(convId);
	const charName = () => library.character(conv()?.characterId)?.name ?? "";
	const userName = () => library.persona(conv()?.personaId)?.name ?? "";
	// 변수: 대화 meta.variables (SillyTavern chat_metadata.variables와 같은 범위). 실행 중엔 여기 들고 있다가 바뀔 때 저장
	const vars: Record<string, string> = Object.fromEntries(
		Object.entries((conv()?.meta as { variables?: Record<string, unknown> } | null)?.variables ?? {}).map(([k, v]) => [k, typeof v === "string" ? v : JSON.stringify(v)]),
	);
	const globals: Record<string, string> = { ...((pluginSetting.value?.quickReply as { globals?: Record<string, string> } | undefined)?.globals ?? {}) };
	/** 이번 실행의 /input 답 (다시 실행에 씀) */
	const answers: Record<string, string> = {};
	let varsDirty = false;
	let globalsDirty = false;
	const limit = aiLimit();
	let aiCalls = 0;
	const ai = (name: string) => {
		if (++aiCalls > limit) throw new ScriptError(`AI 호출이 한 번 실행 상한(${limit}번)을 넘어 /${name}에서 멈춤 — 설정 > 빠른 답장에서 바꿀 수 있음`);
	};
	/** 변수 커맨드 묶음 (대화 변수·전역 변수가 같은 모양) */
	const varCommands = (store: Record<string, string>, dirty: () => void, prefix: "" | "global"): Record<string, Handler> => {
		const keyOf = (named: Record<string, ArgValue>, unnamed: ArgValue) => (text(named.key) || text(unnamed)).trim();
		const set = (key: string, value: string, index?: string) => {
			store[key] = index !== undefined ? setIndexed(store[key] ?? "", index, value) : value;
			dirty();
			return store[key];
		};
		const step = (key: string, by: number) => {
			const n = Number(store[key] ?? 0);
			return set(key, String((Number.isFinite(n) ? n : 0) + by));
		};
		return {
			[`set${prefix}var`]: ({ named, unnamed }) => {
				const key = text(named.key).trim();
				if (!key) throw new ScriptError(`/set${prefix}var: key=이름 이 필요함`);
				set(key, text(unnamed), named.index === undefined ? undefined : text(named.index));
			},
			[`get${prefix}var`]: ({ named, unnamed }) => {
				const v = store[keyOf(named, unnamed)] ?? "";
				return named.index === undefined ? v : (getIndexed(v, text(named.index)) ?? "");
			},
			[`add${prefix}var`]: ({ named, unnamed }) => {
				const key = text(named.key).trim();
				if (!key) throw new ScriptError(`/add${prefix}var: key=이름 이 필요함`);
				return set(key, addTo(store[key] ?? "", text(unnamed)));
			},
			[`inc${prefix}var`]: ({ named, unnamed }) => step(keyOf(named, unnamed), 1),
			[`dec${prefix}var`]: ({ named, unnamed }) => step(keyOf(named, unnamed), -1),
			[`flush${prefix}var`]: ({ named, unnamed }) => {
				delete store[keyOf(named, unnamed)];
				dirty();
			},
		};
	};
	const nextSeq = () => (session.messages.at(-1)?.seq ?? 0) + 1;
	const append = async (role: "user" | "assistant" | "system", content: string, name: string | null = null, meta: Record<string, unknown> | null = null) => {
		if (!content.trim()) throw new ScriptError("보낼 글이 비어 있음");
		const character = role === "assistant" ? (conv()?.characterId ?? null) : null;
		const m = await call(commands.appendMessage({ conversationId: convId, seq: nextSeq(), role, name, content, characterId: character, meta: meta as never }), src);
		session.messages.push(m);
		// /gen 결과가 들어간 메시지면 그때 보낸 프롬프트를 남김 (메시지의 '보낸 프롬프트' 보기)
		if (lastGen && content.includes(lastGen.text)) await call(commands.saveMessagePrompt(m.id, lastGen.prompt as never), { ...src, silent: true }).catch(() => {});
		void library.refresh(convId);
		return content;
	};
	const bySeq = (seqs: number[]) => session.messages.filter((m) => seqs.includes(m.seq));
	/** 마지막 /gen 결과와 보낸 프롬프트 — 이 글이 들어간 메시지에 '보낸 프롬프트'로 남김 (5-E-20-6-10) */
	let lastGen: { text: string; prompt: unknown } | null = null;
	/** /gen 결과: 잘렸으면 알림 (5-E-20-6-6) */
	const generated = (name: string, r: { text: string; truncated: boolean; prompt?: unknown }) => {
		if (r.truncated) toasts.show({ tone: "warning", message: `/${name} 결과가 응답 길이 한도에서 잘림 — 프리셋의 '응답 길이'를 늘리면 됨`, source: "슬래시 커맨드" });
		if (r.text.trim() && r.prompt) lastGen = { text: r.text.trim(), prompt: r.prompt };
		return r.text;
	};
	const setOf = (name: string) => {
		const s = quickReplies.sets.find((x) => x.name === name.trim());
		if (!s) throw new ScriptError(`빠른 답장 세트를 찾지 못함 (${name})`);
		return s;
	};
	const setEnabled = async (name: string, on: boolean) => {
		const s = setOf(name);
		await quickReplies.save(toInput({ ...s, enabled: on }));
	};
	const setInChat = async (name: string, on: boolean) => {
		const s = setOf(name);
		const off = ((conv()?.meta as { quickReplyOff?: string[] } | null)?.quickReplyOff ?? []).filter((x) => x !== s.id);
		await library.patchMeta(convId, { quickReplyOff: on ? off : [...off, s.id] });
	};

	const commandsTable: Record<string, Handler> = {
		// 보내기 (AI를 부르지 않음)
		send: ({ unnamed }) => append("user", text(unnamed)),
		sys: ({ unnamed }) => append("system", text(unnamed)),
		narrator: ({ unnamed }) => append("system", text(unnamed)),
		sendas: ({ named, unnamed }) => {
			const name = text(named.name).trim() || charName();
			return append("assistant", text(unnamed), name === charName() ? null : name);
		},
		// AI 응답 (비용)
		trigger: async () => {
			ai("trigger");
			await session.send("");
		},
		continue: async () => {
			ai("continue");
			await session.continueLast();
		},
		regen: async () => {
			ai("regen");
			await session.regenerate(false);
		},
		swipe: async () => {
			ai("swipe");
			await session.regenerate(true);
		},
		gen: async ({ unnamed }) => {
			ai("gen");
			return generated("gen", await call(commands.slashGenerate(convId, text(unnamed), false, null), src));
		},
		// /genraw system=… 지시: 대화 맥락 없이 지시만
		genraw: async ({ named, unnamed }) => {
			ai("genraw");
			return generated("genraw", await call(commands.slashGenerate(convId, text(unnamed), true, text(named.system) || null), src));
		},
		// /impersonate 초안: 대필로 내 다음 대사를 입력칸에 (대필 플러그인이 켜져 있어야 함)
		impersonate: async ({ unnamed }, ) => {
			if (!plugins.on("ghostwrite")) throw new ScriptError("/impersonate: 대필 플러그인을 켜야 함 (설정 > 코어 플러그인)");
			ai("impersonate");
			let draft = opts.input ?? "";
			const out = await ghost.run(convId, text(unnamed) || null, () => draft, (v) => {
				draft = v;
				opts.setInput?.(v);
			});
			return out ?? "";
		},
		// AI에게 보내지 않는 메모 메시지 (SillyTavern /comment)
		comment: ({ unnamed }) => append("system", text(unnamed), null, { hidden: true, comment: true }),
		// /messages names=off 2-5 → 그 메시지들의 글 (0부터, 비우면 마지막 하나)
		messages: ({ named, unnamed }) => {
			const names = !["off", "false", "0"].includes(text(named.names).toLowerCase());
			const nameOf = (m: (typeof session.messages)[number]) => m.name || (m.role === "user" ? userName() : m.role === "assistant" ? charName() : "");
			return indexRange(text(unnamed), session.messages.length)
				.map((i) => session.messages[i])
				.map((m) => (names && nameOf(m) ? `${nameOf(m)}: ${m.content}` : m.content))
				.join("\n\n");
		},
		// /cut 2-5 → 지우고 그 글을 돌려줌
		cut: async ({ unnamed }) => {
			const picked = indexRange(text(unnamed), session.messages.length).map((i) => session.messages[i]);
			const out = picked.map((m) => m.content).join("\n\n");
			await session.removeMany(picked.map((m) => m.id));
			return out;
		},
		// 입력칸
		setinput: ({ unnamed }) => {
			opts.setInput?.(text(unnamed));
		},
		// 대화 설정: 이름을 주면 바꾸고, 안 주면 지금 것의 이름
		persona: async ({ unnamed }, ) => {
			const name = text(unnamed).trim();
			const c = conv();
			if (!name || !c) return userName();
			const p = library.personas.find((x) => x.name === name) ?? library.personas.find((x) => x.name.toLowerCase() === name.toLowerCase());
			if (!p) throw new ScriptError(`/persona: 페르소나를 찾지 못함 (${name})`);
			await library.setOptions(c.id, { personaId: p.id, promptPresetId: c.promptPresetId, apiProfileId: c.apiProfileId, model: c.model });
			return p.name;
		},
		preset: async ({ unnamed }) => {
			const name = text(unnamed).trim();
			const c = conv();
			if (!c) return "";
			if (!name) return library.presets.find((x) => x.id === c.promptPresetId)?.name ?? "";
			const p = library.presets.find((x) => x.name === name);
			if (!p) throw new ScriptError(`/preset: 프리셋을 찾지 못함 (${name})`);
			await library.setOptions(c.id, { personaId: c.personaId, promptPresetId: p.id, apiProfileId: c.apiProfileId, model: c.model });
			return p.name;
		},
		model: async ({ unnamed }) => {
			const name = text(unnamed).trim();
			const c = conv();
			if (!c) return "";
			if (!name) return c.model ?? "";
			await library.setOptions(c.id, { personaId: c.personaId, promptPresetId: c.promptPresetId, apiProfileId: c.apiProfileId, model: name });
			return name;
		},
		// /inject id=… position=before|after|chat|none depth=4 role=system 글 → 대화 meta.injects (프롬프트에 들어감)
		inject: async ({ named, unnamed }) => {
			const id = text(named.id).trim() || `inject-${Date.now()}`;
			const injects = { ...(((conv()?.meta as { injects?: Record<string, unknown> } | null)?.injects ?? {}) as Record<string, unknown>) };
			if (!text(unnamed).trim()) delete injects[id];
			else {
				const position = text(named.position) || "after";
				if (!["before", "after", "chat", "none"].includes(position)) throw new ScriptError(`/inject: position은 before·after·chat·none (${position})`);
				injects[id] = { value: text(unnamed), position, depth: Number(text(named.depth)) || 4, role: text(named.role) || "system" };
			}
			await library.patchMeta(convId, { injects });
			return id;
		},
		listinjects: () => JSON.stringify((conv()?.meta as { injects?: unknown } | null)?.injects ?? {}),
		flushinject: async ({ unnamed }) => {
			const id = text(unnamed).trim();
			const injects = { ...(((conv()?.meta as { injects?: Record<string, unknown> } | null)?.injects ?? {}) as Record<string, unknown>) };
			if (id) delete injects[id];
			await library.patchMeta(convId, { injects: id ? injects : {} });
		},
		// 빠른 답장 세트 켜고 끄기: /qr-set-on 이름 (전체), /qr-chat-set-on 이름 (이 대화)
		"qr-set-on": ({ unnamed }) => setEnabled(text(unnamed), true),
		"qr-set-off": ({ unnamed }) => setEnabled(text(unnamed), false),
		"qr-chat-set-on": ({ unnamed }) => setInChat(text(unnamed), true),
		"qr-chat-set-off": ({ unnamed }) => setInChat(text(unnamed), false),
		// 메시지 정리
		del: async ({ unnamed }) => {
			const n = Math.max(1, Number(text(unnamed)) || 1);
			await session.removeMany(session.messages.slice(-n).map((m) => m.id));
		},
		hide: async ({ unnamed }) => {
			for (const m of bySeq(range(text(unnamed), session.messages.at(-1)?.seq ?? 0))) await session.setHidden(m, true);
		},
		unhide: async ({ unnamed }) => {
			for (const m of bySeq(range(text(unnamed), session.messages.at(-1)?.seq ?? 0))) await session.setHidden(m, false);
		},
		// 알림·입력·변수
		// /echo severity=info|success|warning|error title=… timeout=ms 글 (SillyTavern 알림과 같은 옵션)
		echo: ({ named, unnamed }) => {
			const sev = text(named.severity).toLowerCase();
			const tone = sev === "success" || sev === "warning" || sev === "error" ? sev : "info";
			const title = text(named.title).trim();
			const ms = Number(text(named.timeout));
			toasts.show({ tone, message: title ? `${title} · ${text(unnamed)}` : text(unnamed), source: "슬래시 커맨드" }, Number.isFinite(ms) && ms > 0 ? ms : undefined);
		},
		// /input default=… placeholder=… 질문 → 입력한 글 (취소하면 빈 글)
		// 취소하면 스크립트를 멈춤 (5-E-20-6-9). 다시 실행(6-11)할 때는 앞 실행의 답을 다시 묻지 않고 씀
		input: async ({ named, unnamed }) => {
			const title = text(unnamed) || "입력";
			const again = opts.answers?.[title];
			const v = again ?? (await promptText({ title, input: { value: text(named.default), placeholder: text(named.placeholder) }, confirmLabel: text(named.okButton) || undefined }));
			if (v === null) abortScript();
			answers[title] = v;
			return v;
		},
		// /popup okButton=… cancelButton=… 글 → 확인 1 / 취소 0 (취소 버튼은 cancelButton을 줄 때만)
		popup: async ({ named, unnamed }) => {
			const cancel = text(named.cancelButton).trim();
			const ok = await confirmAction({ title: text(named.title) || "알림", message: text(unnamed), confirmLabel: text(named.okButton) || "확인", cancelLabel: cancel || undefined, hideCancel: !cancel });
			return ok ? "1" : "0";
		},
		// /buttons labels=["가","나"] multiple=true 질문 → 고른 이름 (multiple이면 JSON 목록, 취소하면 빈 글)
		buttons: async ({ named, unnamed }) => {
			let labels: string[] = [];
			try {
				const v = JSON.parse(text(named.labels)) as unknown;
				labels = Array.isArray(v) ? v.map(String) : [];
			} catch {
				labels = text(named.labels).split(",").map((x) => x.trim()).filter(Boolean);
			}
			if (!labels.length) throw new ScriptError('/buttons: labels=["가","나"] 가 필요함');
			const multiple = ["true", "on", "1"].includes(text(named.multiple).toLowerCase());
			return (await chooseFrom({ title: text(unnamed) || "고르기", choices: labels, multiple })) ?? "";
		},
		...varCommands(vars, () => (varsDirty = true), ""),
		...varCommands(globals, () => (globalsDirty = true), "global"),
	};

	return {
		commands: commandsTable,
		unsupported: UNSUPPORTED,
		quickReply: quickReplySource,
		answers,
		variable: (name) => vars[name] ?? globals[name],
		flush: async () => {
			if (varsDirty) await library.patchMeta(convId, { variables: { ...vars } });
			if (globalsDirty && pluginSetting.value) {
				const cur = pluginSetting.value;
				await pluginSetting.save({ ...cur, quickReply: { ...(cur.quickReply ?? {}), globals: { ...globals } } } as typeof cur);
			}
			varsDirty = globalsDirty = false;
		},
		macro: (name, arg) => {
			switch (name) {
				case "char":
					return charName();
				case "user":
					return userName();
				case "getvar":
					return vars[(arg ?? "").trim()] ?? "";
				case "getglobalvar":
					return globals[(arg ?? "").trim()] ?? "";
				case "lastmessage":
					return session.messages.at(-1)?.content ?? "";
				case "newline":
					return "\n";
				case "input":
					return opts.input ?? "";
				// 1:1 대화에서는 캐릭터 이름 (단체 대화가 없음)
				case "groupnotmuted":
				case "charifnotgroup":
				case "group":
					return charName();
				case "lastmessageid":
					return String(Math.max(0, session.messages.length - 1));
				case "lastcharmessage":
					return [...session.messages].reverse().find((m) => m.role === "assistant")?.content ?? "";
				case "lastusermessage":
					return [...session.messages].reverse().find((m) => m.role === "user")?.content ?? "";
				case "idle_duration": {
					const last = session.messages.at(-1)?.createdAt;
					if (!last) return "처음";
					const min = Math.floor((Date.now() - last) / 60000);
					return min < 1 ? "방금" : min < 60 ? `${min}분` : min < 1440 ? `${Math.floor(min / 60)}시간` : `${Math.floor(min / 1440)}일`;
				}
				default:
					return undefined;
			}
		},
	};
}

/** 입력칸·빠른 답장에서 실행. 오류는 알림으로 (멈춘 것은 조용히) */
export async function runSlash(source: string, session: ChatSession, opts: SlashOpts = {}): Promise<void> {
	if (session.busy) return void toasts.error("응답을 받는 중이라 실행하지 않음");
	const convId = session.convId;
	if (slashRuns.isRunning(convId)) return void toasts.error("이 대화에서 스크립트가 도는 중 — 멈춘 뒤 다시");
	if (!quickReplies.loaded) await quickReplies.load().catch(() => {});
	let at = "";
	const env = { ...slashEnv(session, opts), stopped: () => slashRuns.stopped(convId), trace: (name: string) => (at = name) };
	slashRuns.start(convId);
	try {
		const r = await runScript(source, env);
		if (r.stopped) toasts.info("스크립트를 멈춤");
	} catch (e) {
		if (slashRuns.stopped(convId)) toasts.info("스크립트를 멈춤");
		else {
			// 실패: 어디서였는지와 '다시 실행' (앞 실행의 /input 답은 다시 묻지 않음, 5-E-20-6-11).
			// AI·저장 오류 자체는 call이 이미 알림을 띄웠으므로 여기서는 멈춘 자리와 다시 실행만
			const why = e instanceof ScriptError ? e.message : `/${at || "?"}에서 실패해 멈춤`;
			const answers = { ...(opts.answers ?? {}), ...env.answers };
			toasts.show(
				{ tone: "error", message: why, source: "슬래시 커맨드", action: { label: "다시 실행", run: () => void runSlash(source, session, { ...opts, answers }) } },
				20_000,
			);
		}
	} finally {
		// 멈추거나 실패해도 그때까지 바꾼 변수는 남긴다
		await env.flush().catch(() => {});
		slashRuns.end(convId);
	}
}

/** HTML 블록의 triggerSlash 받기 (5-E-20-6-12): await 없이 연달아 부르는 일이 많아 하나씩 차례로 돌린다
 *  (runSlash는 같은 대화에서 겹쳐 돌지 않음). 대기가 max개를 넘으면 버린다. opts는 돌 차례에 읽는다 ({{input}}이 앞 실행의 /setinput을 보게) */
export function slashQueue(max = 20): (text: string, session: ChatSession, opts: () => SlashOpts) => void {
	let chain: Promise<void> = Promise.resolve();
	let queued = 0;
	return (text, session, opts) => {
		if (queued >= max) return void toasts.error("슬래시 커맨드 요청이 너무 많아 건너뜀");
		queued++;
		chain = chain
			.then(() => runSlash(text, session, opts()))
			.catch(() => {})
			.finally(() => queued--);
	};
}

/** 입력이 슬래시 커맨드인가 (`/`로 시작, `//`·`/ ` 같은 글은 아님) */
export function isSlash(text: string): boolean {
	return /^\/[A-Za-z]/.test(text.trimStart());
}
