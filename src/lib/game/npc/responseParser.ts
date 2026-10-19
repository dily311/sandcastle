// AI가 준 원문 텍스트를 파싱합니다. 이 파일은 호스트 쪽에서만 실행됩니다
// (원문을 로깅하기 전에 어차피 들여다봐야 해서 호스트가 파싱을 담당합니다).
//
// 지원하는 태그:
//   <state>key += value</state>   -- 여러 줄 가능, 한 줄에 하나씩
//   <state>key -= value</state>
//   <state>key = value</state>
//   <choice>선택지 텍스트</choice>  -- 태그 하나 = 선택지 하나, 여러 개면 태그 반복
//
// JSON 대신 이 문법을 쓰는 이유: LLM이 훨씬 안정적으로 뱉어냅니다.

import type { StateChangeInstruction } from '../types';

export interface ParsedAiResponse {
  lines: string[];
  stateChanges: StateChangeInstruction[];
  choices: string[];
  /** 태그를 걷어낸 텍스트 (messages.content에 저장할 때 씁니다) */
  cleanedText: string;
}

const STATE_TAG_RE = /<state>([\s\S]*?)<\/state>/g;
const CHOICE_TAG_RE = /<choice>([\s\S]*?)<\/choice>/g;

function coerceValue(raw: string): unknown {
  const trimmed = raw.trim();
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const n = Number(trimmed);
  return Number.isNaN(n) ? trimmed : n;
}

function parseStateBlock(block: string): StateChangeInstruction[] {
  const changes: StateChangeInstruction[] = [];

  for (const rawLine of block.split('\n')) {
    const line = rawLine.trim();
    if (!line) continue;

    const addMatch = line.match(/^([\w.]+)\s*\+=\s*(.+)$/);
    const subMatch = line.match(/^([\w.]+)\s*-=\s*(.+)$/);
    const setMatch = line.match(/^([\w.]+)\s*=\s*(.+)$/);

    if (addMatch) {
      changes.push({ op: 'add', key: addMatch[1], value: coerceValue(addMatch[2]) });
    } else if (subMatch) {
      changes.push({ op: 'sub', key: subMatch[1], value: coerceValue(subMatch[2]) });
    } else if (setMatch) {
      changes.push({ op: 'set', key: setMatch[1], value: coerceValue(setMatch[2]) });
    } else {
      console.warn(`[responseParser] 상태 변경 줄을 이해하지 못했어요: "${line}"`);
    }
  }

  return changes;
}

export function parseAiResponse(raw: string): ParsedAiResponse {
  const stateChanges: StateChangeInstruction[] = [];
  const choices: string[] = [];

  let text = raw.replace(STATE_TAG_RE, (_match, inner: string) => {
    stateChanges.push(...parseStateBlock(inner));
    return '';
  });

  text = text.replace(CHOICE_TAG_RE, (_match, inner: string) => {
    const choiceText = inner.trim();
    if (choiceText) choices.push(choiceText);
    return '';
  });

  const lines = text
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l.length > 0);

  return { lines, stateChanges, choices, cleanedText: lines.join('\n') };
}
