import type { SceneDraft } from './draft';

export interface ValidationError {
  severity: 'error' | 'warning';
  code: string;
  nodeId?: string;
  message: string;
}

export function validateScene(scene: SceneDraft, knownVariables: Set<string>): ValidationError[] {
  const errors: ValidationError[] = [];
  const nodeIds = new Set(scene.nodes.map((n) => n.id));
  const referenced = new Set<string>();

  const keyCounts = new Map<string, number>();
  for (const node of scene.nodes) {
    keyCounts.set(node.nodeKey, (keyCounts.get(node.nodeKey) ?? 0) + 1);
  }
  for (const node of scene.nodes) {
    if ((keyCounts.get(node.nodeKey) ?? 0) > 1) {
      errors.push({
        severity: 'error',
        code: 'DUPLICATE_KEY',
        nodeId: node.id,
        message: `노드 키 "${node.nodeKey}"가 다른 노드와 중복돼요. (DB에서 씬 안 유일해야 해요)`,
      });
    }
  }

  function checkNext(nodeId: string, next: string | null, label: string) {
    if (next === null) return;
    referenced.add(next);
    if (!nodeIds.has(next)) {
      errors.push({
        severity: 'error',
        code: 'BROKEN_LINK',
        nodeId,
        message: `${label}의 연결이 끊어졌습니다.`,
      });
    }
  }

  for (const node of scene.nodes) {
    if (node.type === 'line') {
      checkNext(node.id, node.next, '다음 노드');
      if (!node.text.trim()) {
        errors.push({
          severity: 'warning',
          code: 'EMPTY_TEXT',
          nodeId: node.id,
          message: '대사 내용이 비어 있어요.',
        });
      }
    }

    if (node.type === 'choice') {
      if (node.choices.length === 0) {
        errors.push({
          severity: 'error',
          code: 'NO_CHOICES',
          nodeId: node.id,
          message: '선택지가 하나도 없어요.',
        });
      }
      node.choices.forEach((c, i) => {
        checkNext(node.id, c.next, `선택지 "${c.text || i + 1}"`);
        if (c.effectVariable && !knownVariables.has(c.effectVariable)) {
          errors.push({
            severity: 'error',
            code: 'UNKNOWN_VARIABLE',
            nodeId: node.id,
            message: `선택지 "${c.text || i + 1}"가 정의되지 않은 변수 "${c.effectVariable}"를 사용해요.`,
          });
        }
      });
    }

    if (node.type === 'branch') {
      if (node.branches.length === 0) {
        errors.push({
          severity: 'error',
          code: 'NO_BRANCHES',
          nodeId: node.id,
          message: '분기 조건이 하나도 없어요.',
        });
      }
      const hasFallback = node.branches.some((b) => !b.conditionVariable);
      if (!hasFallback) {
        errors.push({
          severity: 'warning',
          code: 'NO_FALLBACK',
          nodeId: node.id,
          message: '조건 없는 기본 분기(fallback)가 없어서, 모든 조건이 거짓이면 막힐 수 있어요.',
        });
      }
      node.branches.forEach((b) => {
        checkNext(node.id, b.next, '분기');
        if (b.conditionVariable && !knownVariables.has(b.conditionVariable)) {
          errors.push({
            severity: 'error',
            code: 'UNKNOWN_VARIABLE',
            nodeId: node.id,
            message: `분기가 정의되지 않은 변수 "${b.conditionVariable}"를 사용해요.`,
          });
        }
      });
    }

    if (node.type === 'ai') {
      checkNext(node.id, node.next, '다음 노드');
      if (!node.characterId) {
        errors.push({
          severity: 'error',
          code: 'NO_CHARACTER',
          nodeId: node.id,
          message: 'AI 노드인데 캐릭터가 선택되지 않았어요.',
        });
      }
      if (node.maxTurns !== null && node.maxTurns <= 0) {
        errors.push({
          severity: 'warning',
          code: 'INVALID_MAX_TURNS',
          nodeId: node.id,
          message: '최대 왕복 횟수는 1 이상이거나 비워둬야(기본값 사용) 해요.',
        });
      }
    }
  }

  const startId = scene.nodes[0]?.id;
  for (const node of scene.nodes) {
    if (node.id !== startId && !referenced.has(node.id)) {
      errors.push({
        severity: 'warning',
        code: 'ORPHAN_NODE',
        nodeId: node.id,
        message: '이 노드로 연결되는 경로가 없어요 (고립된 노드).',
      });
    }
  }

  return errors;
}
