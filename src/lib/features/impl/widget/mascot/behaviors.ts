import { buildBehaviorsFromWeights, defaultWeights, type Behavior } from "./simejiType";

export const behaviors: Behavior[] = buildBehaviorsFromWeights(defaultWeights(), 5);
