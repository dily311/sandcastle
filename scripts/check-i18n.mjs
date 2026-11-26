#!/usr/bin/env node
// 프런트엔드 하드코딩 한글 검사 (plans/roadmap.md 5-C-2-2, ADR-032 ⑥).
// - 주석(//, /* */, <!-- -->), 테스트(*.test.ts), 가짜 백엔드(dev-mock), 자동 생성 바인딩(bindings.ts),
//   시안(design-preview, mockup.ts), 콘솔/내부 로그(console.*, logs.*), 명시적 예외(i18n-ignore)는 제외한다.
// - 파일별 한글 포함 줄 수가 scripts/i18n-baseline.json 기준보다 늘어나면(새 파일은 0 기준) 실패(exit 1)한다.
// - 줄어든 파일이 있으면 `pnpm i18n:baseline`으로 기준표를 낮추도록 안내한다 (톱니바퀴 방식).

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const BASELINE_PATH = path.join(ROOT, "scripts", "i18n-baseline.json");
const HAS_HANGUL = /[가-힣]/;

function walk(dir) {
	const out = [];
	for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
		const p = path.join(dir, e.name);
		if (e.isDirectory()) out.push(...walk(p));
		else out.push(p);
	}
	return out;
}

function toRel(absPath) {
	return path.relative(ROOT, absPath).replace(/\\/g, "/");
}

function isCheckedFile(rel) {
	if (!rel.endsWith(".svelte") && !rel.endsWith(".ts")) return false;
	if (rel.endsWith(".test.ts")) return false;
	if (rel.startsWith("src/ipc/dev-mock/")) return false;
	if (rel === "src/ipc/bindings.ts") return false;
	if (rel.startsWith("src/design-preview/")) return false;
	if (rel === "src/data/mockup.ts") return false;
	if (rel.startsWith("src/locales/")) return false;
	return true;
}

/** 줄 번호를 유지한 채 주석과 i18n-ignore 표시 줄을 비운다 */
export function stripCommentsPreserveLines(code) {
	const rawLines = code.split(/\r?\n/);
	let s = code.replace(/<!--[\s\S]*?-->/g, (m) => m.replace(/[^\n]/g, " "));
	s = s.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "));
	return s.split(/\r?\n/).map((line, idx) => {
		if (rawLines[idx]?.includes("i18n-ignore") || line.includes("i18n-ignore")) return "";
		let inS = false;
		let inD = false;
		let inB = false;
		for (let i = 0; i < line.length; i++) {
			const c = line[i];
			if (c === "\\") {
				i++;
				continue;
			}
			if (!inD && !inB && c === "'") inS = !inS;
			else if (!inS && !inB && c === '"') inD = !inD;
			else if (!inS && !inD && c === "`") inB = !inB;
			else if (!inS && !inD && !inB && c === "/" && line[i + 1] === "/") {
				return line.slice(0, i);
			}
		}
		return line;
	});
}

function isLogLine(line) {
	if (/^\s*(console|logs)\.(log|info|warn|error|debug|add)\b/.test(line)) return true;
	if (/=>\s*(void\s+)?(console|logs)\.(log|info|warn|error|debug|add)\b/.test(line)) return true;
	if (/\bfor\s*\([^)]*\)\s*(console|logs)\.(log|info|warn|error|debug|add)\b/.test(line)) return true;
	return false;
}

export function scanWorkspace(srcRoot = path.join(ROOT, "src")) {
	const counts = {};
	const details = {};
	for (const abs of walk(srcRoot)) {
		const rel = toRel(abs);
		if (!isCheckedFile(rel)) continue;
		const raw = fs.readFileSync(abs, "utf8");
		const lines = stripCommentsPreserveLines(raw);
		const hits = [];
		lines.forEach((l, idx) => {
			if (!HAS_HANGUL.test(l)) return;
			if (isLogLine(l)) return;
			hits.push({ line: idx + 1, text: l.trim() });
		});
		if (hits.length > 0) {
			counts[rel] = hits.length;
			details[rel] = hits;
		}
	}
	const sorted = Object.fromEntries(Object.entries(counts).sort(([a], [b]) => a.localeCompare(b)));
	return { counts: sorted, details };
}

function main() {
	const args = new Set(process.argv.slice(2));
	const writeBaseline = args.has("--write-baseline");
	const verbose = args.has("--verbose");

	const { counts, details } = scanWorkspace();
	const totalLines = Object.values(counts).reduce((a, b) => a + b, 0);
	const totalFiles = Object.keys(counts).length;

	if (writeBaseline) {
		fs.writeFileSync(BASELINE_PATH, JSON.stringify(counts, null, "\t") + "\n", "utf8");
		console.log(`[i18n] 기준표 저장 완료: 파일 ${totalFiles}개 · 하드코딩 한글 ${totalLines}줄 (${toRel(BASELINE_PATH)})`);
		return;
	}

	if (!fs.existsSync(BASELINE_PATH)) {
		console.error(`[i18n] 기준표 파일이 없습니다: ${toRel(BASELINE_PATH)} (pnpm i18n:baseline 실행 필요)`);
		process.exit(1);
	}

	const baseline = JSON.parse(fs.readFileSync(BASELINE_PATH, "utf8"));
	const regressions = [];
	const improvements = [];

	const allFiles = new Set([...Object.keys(baseline), ...Object.keys(counts)]);
	for (const file of Array.from(allFiles).sort()) {
		const prev = baseline[file] ?? 0;
		const cur = counts[file] ?? 0;
		if (cur > prev) {
			regressions.push({ file, prev, cur, hits: details[file] ?? [] });
		} else if (cur < prev) {
			improvements.push({ file, prev, cur });
		}
	}

	if (verbose) {
		for (const [file, hits] of Object.entries(details)) {
			console.log(`\n${file} (${hits.length})`);
			for (const h of hits) console.log(`  L${h.line}: ${h.text}`);
		}
	}

	if (regressions.length > 0) {
		console.error(`[i18n] 하드코딩 한글이 기준표보다 늘었습니다 (${regressions.length}개 파일):`);
		for (const r of regressions) {
			console.error(`  - ${r.file}: 기준 ${r.prev}줄 → 현재 ${r.cur}줄`);
			for (const h of r.hits.slice(0, 8)) {
				console.error(`      L${h.line}: ${h.text}`);
			}
			if (r.hits.length > 8) console.error(`      ... 외 ${r.hits.length - 8}줄`);
		}
		console.error("\n화면 문구는 src/locales/<언어>/<영역>.json 키와 $_()를 사용하세요.");
		process.exit(1);
	}

	console.log(`[i18n] 하드코딩 한글 검사 통과 (현재 파일 ${totalFiles}개 · ${totalLines}줄)`);
	if (improvements.length > 0) {
		const saved = improvements.reduce((s, x) => s + (x.prev - x.cur), 0);
		console.log(`[i18n] ${improvements.length}개 파일에서 ${saved}줄 줄었습니다. \`pnpm i18n:baseline\`으로 기준표를 낮춰 주세요.`);
	}
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	main();
}
