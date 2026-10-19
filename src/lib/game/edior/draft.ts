// 에디터가 실제로 다루는 작업용 모델입니다.
// DB에서 읽어온 GameScene(Row, 문자열 JSON 포함)을 사람이 다루기 쉬운 형태(Draft)로
// 풀어놓고, 저장할 때 다시 Rust가 원하는 Input 형태(문자열 JSON)로 조립합니다.

import { v4 as uuidv4 } from 'uuid';
import type {
  GameScene,
  GameNode,
  ChoiceRow,
  BranchRow,
  SceneInput,
  NodeInput,
  ChoiceInput,
  BranchInput,
} from '../db/rowTypes';
import type { ConditionOp } from '../types';

export interface ChoiceDraft {
  id: string;
  text: string;
  next: string | null;
  effectVariable: string; // ''면 효과 없음
  effectOp: 'add' | 'set';
  effectValue: string;
}

export interface BranchDraft {
  id: string;
  conditionVariable: string; // ''면 조건 없음 (기본/fallback 분기)
  conditionOp: ConditionOp;
  conditionValue: string;
  next: string | null;
}

export interface NodeDraft {
  id: string;
  nodeKey: string;
  type: 'line' | 'choice' | 'branch' | 'ai';
  speaker: string;
  text: string;
  next: string | null; // line/ai 전용
  choices: ChoiceDraft[]; // choice 전용
  branches: BranchDraft[]; // branch 전용
  sortOrder: number;
  // ai 전용
  characterId: string;
  promptContext: string;
  maxTurns: number | null; // 없으면 엔진 기본값(5) 사용
}

export interface SceneDraft {
  id: string;
  sceneName: string;
  description: string;
  sceneKey: string;
  isEnding: boolean;
  settingsJson: string;
  createdAt: string;
  updatedAt: string;
  nodes: NodeDraft[];
}

// ---------- 새 항목 생성 ----------

export function generateNodeKey(existingKeys: Iterable<string>): string {
  const used = new Set(existingKeys);
  let i = 1;
  while (used.has(`node_${i}`)) i++;
  return `node_${i}`;
}

export function createEmptyLineNode(sortOrder: number, existingKeys: Iterable<string>): NodeDraft {
  return {
    id: uuidv4(),
    nodeKey: generateNodeKey(existingKeys),
    type: 'line',
    speaker: '',
    text: '',
    next: null,
    choices: [],
    branches: [],
    sortOrder,
    characterId: '',
    promptContext: '',
    maxTurns: null,
  };
}

export function createChoiceDraft(): ChoiceDraft {
  return {
    id: uuidv4(),
    text: '',
    next: null,
    effectVariable: '',
    effectOp: 'add',
    effectValue: '',
  };
}

export function createBranchDraft(): BranchDraft {
  return {
    id: uuidv4(),
    conditionVariable: '',
    conditionOp: 'gte',
    conditionValue: '',
    next: null,
  };
}

// ---------- Row -> Draft ----------

function parseJson<T>(value: string | null): T | undefined {
  if (!value) return undefined;
  try {
    return JSON.parse(value) as T;
  } catch {
    return undefined;
  }
}

function choiceRowToDraft(row: ChoiceRow): ChoiceDraft {
  const effect = parseJson<{ add?: Record<string, number>; set?: Record<string, unknown> }>(
    row.effect
  );
  const addEntry = effect?.add ? Object.entries(effect.add)[0] : undefined;
  const setEntry = effect?.set ? Object.entries(effect.set)[0] : undefined;

  return {
    id: row.id,
    text: row.text,
    next: row.nextNodeId,
    effectVariable: addEntry?.[0] ?? setEntry?.[0] ?? '',
    effectOp: addEntry ? 'add' : 'set',
    effectValue: String(addEntry?.[1] ?? setEntry?.[1] ?? ''),
  };
}

function branchRowToDraft(row: BranchRow): BranchDraft {
  // condition 형태: { "gte": { "affection_yuna": 3 } }
  const condition = parseJson<Record<string, Record<string, unknown>>>(row.condition);
  const opEntry = condition ? Object.entries(condition)[0] : undefined;
  const varEntry = opEntry ? Object.entries(opEntry[1])[0] : undefined;

  return {
    id: row.id,
    conditionVariable: varEntry?.[0] ?? '',
    conditionOp: (opEntry?.[0] as ConditionOp) ?? 'gte',
    conditionValue: varEntry ? String(varEntry[1]) : '',
    next: row.nextNodeId,
  };
}

function nodeRowToDraft(row: GameNode): NodeDraft {
  return {
    id: row.id,
    nodeKey: row.nodeKey,
    type: row.type as NodeDraft['type'],
    speaker: row.speaker ?? '',
    text: row.text ?? '',
    next: row.nextNodeId,
    choices: (row.choices ?? []).map(choiceRowToDraft),
    branches: (row.branches ?? []).map(branchRowToDraft),
    sortOrder: row.sortOrder,
    characterId: row.characterId ?? '',
    promptContext: row.promptContext ?? '',
    maxTurns: row.maxTurns ?? null,
  };
}

export function sceneRowToDraft(row: GameScene): SceneDraft {
  return {
    id: row.id,
    sceneName: row.sceneName,
    description: row.description ?? '',
    sceneKey: row.sceneKey,
    isEnding: row.isEnding,
    settingsJson: row.settingsJson ?? '',
    nodes: [...row.nodes].sort((a, b) => a.sortOrder - b.sortOrder).map(nodeRowToDraft),
  };
}

// ---------- Draft -> Input (Rust로 보낼 형태) ----------

function coerceValue(raw: string): unknown {
  if (raw === '') return raw;
  if (raw === 'true') return true;
  if (raw === 'false') return false;
  const n = Number(raw);
  return Number.isNaN(n) ? raw : n;
}

function choiceDraftToInput(draft: ChoiceDraft, sortOrder: number): ChoiceInput {
  let effect: string | undefined;
  if (draft.effectVariable) {
    effect = JSON.stringify({
      [draft.effectOp]: { [draft.effectVariable]: coerceValue(draft.effectValue) },
    });
  }

  return {
    id: draft.id,
    text: draft.text,
    nextNodeId: draft.next,
    effect,
    sortOrder,
  };
}

function branchDraftToInput(draft: BranchDraft, sortOrder: number): BranchInput {
  let condition: string | undefined;
  if (draft.conditionVariable) {
    condition = JSON.stringify({
      [draft.conditionOp]: { [draft.conditionVariable]: coerceValue(draft.conditionValue) },
    });
  }

  return {
    id: draft.id,
    condition,
    nextNodeId: draft.next,
    sortOrder,
  };
}

export function nodeDraftToInput(draft: NodeDraft, sortOrder: number): NodeInput {
  const isLineLike = draft.type === 'line' || draft.type === 'ai';

  return {
    id: draft.id,
    nodeKey: draft.nodeKey,
    type: draft.type,
    speaker: draft.type === 'line' ? draft.speaker || null : null,
    text: draft.type === 'line' ? draft.text || null : null,
    nextNodeId: isLineLike ? draft.next : null,
    sortOrder,
    choices: draft.type === 'choice' ? draft.choices.map((c, i) => choiceDraftToInput(c, i)) : [],
    branches:
      draft.type === 'branch' ? draft.branches.map((b, i) => branchDraftToInput(b, i)) : [],
    characterId: draft.type === 'ai' ? draft.characterId || null : null,
    promptContext: draft.type === 'ai' ? draft.promptContext || null : null,
    maxTurns: draft.type === 'ai' ? draft.maxTurns : null,
  };
}

export function sceneDraftToInput(draft: SceneDraft): SceneInput {
  return {
    id: draft.id,
    sceneName: draft.sceneName,
    description: draft.description || null,
    sceneKey: draft.sceneKey,
    isEnding: draft.isEnding,
    settingsJson: draft.settingsJson || null,
    nodes: draft.nodes.map((n, i) => nodeDraftToInput(n, i)),
    createdAt: draft.createdAt,
    updatedAt: draft.updatedAt
  };
}
