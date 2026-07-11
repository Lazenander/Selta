export interface Delta {
  message: string;
  data?: unknown;
  expected?: string;
  actual?: string;
}

export interface Usage {
  input_tokens?: number;
  output_tokens?: number;
  cost_usd?: number;
}

export interface VerifyResult {
  verdict: "pass" | "fail";
  delta?: Delta;
  usage?: Usage;
}

export interface Context {
  config: unknown;
  /** Resolved server ⊕ pool settings, secrets already injected (docs/05). */
  settings: unknown;
  path: string;
  root?: unknown;
  env?: unknown;
  budget: { depth: number; deadline_ms?: number };
}

export type Handler = (value: unknown, ctx: Context) => VerifyResult | Promise<VerifyResult>;

export interface VerifierOptions {
  determinism?: "deterministic" | "nondeterministic";
  /** Stable identity for observable verifier semantics. Required when cacheable is true. */
  semanticRevision?: string;
  /** Opt in only for deterministic, referentially transparent verifiers. Defaults to false. */
  cacheable?: boolean;
  configSchema?: unknown;
  settingsSchema?: unknown;
  deltaSchema?: unknown;
  needs?: ("root" | "env")[];
}

export function pass(): VerifyResult;
export function fail(delta: Delta | string): VerifyResult;

export declare const host: {
  verifier(name: string, options: VerifierOptions, handler: Handler): void;
  run(info?: { name: string; version: string }): void;
};
