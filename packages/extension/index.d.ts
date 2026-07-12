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

export interface AssessmentResult {
  support: boolean;
  refute?: Delta;
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
export type Assessor =
  (value: unknown, ctx: Context) => AssessmentResult | Promise<AssessmentResult>;

export interface VerifierOptions {
  determinism?: "deterministic" | "nondeterministic";
  /** Stable identity for observable verifier semantics. Required when cacheable is true. */
  semanticRevision?: string;
  /** Opt in only for deterministic, referentially transparent verifiers. Defaults to false. */
  cacheable?: boolean;
  /** Coarse effect claim. Omitted declarations are treated as unknown. */
  effectClass?: "pure" | "process_io" | "unknown";
  /** Selta node kinds accepted by this verifier. Omission preserves legacy any-kind behavior. */
  acceptedInput?: ("null" | "bool" | "int" | "float" | "str" | "object" | "array")[];
  configSchema?: unknown;
  settingsSchema?: unknown;
  deltaSchema?: unknown;
  needs?: ("root" | "env")[];
}

export function pass(): VerifyResult;
export function fail(delta: Delta | string): VerifyResult;
export function support(): AssessmentResult;
export function refute(delta: Delta | string): AssessmentResult;
export function both(delta: Delta | string): AssessmentResult;
export function neither(): AssessmentResult;

export declare const host: {
  verifier(name: string, options: VerifierOptions, handler: Handler): void;
  assessor(name: string, handler: Assessor): void;
  run(info?: { name: string; version: string }): void;
};
