// Scripted host for the RpcHost integration test: a real child process
// speaking the docs/05 protocol through the reference SDK.

import {
  both,
  host,
  neither,
  pass,
  fail,
  refute,
  support,
} from "../../../../packages/extension/index.js";

host.verifier(
  "length_judge",
  { determinism: "nondeterministic" },
  (value, ctx) => {
    const min = ctx.config?.min ?? 5;
    const text = String(value);
    return text.length >= min
      ? pass()
      : fail({ message: `too short: ${text.length} < ${min}` });
  },
);

host.assessor("length_judge", (value, ctx) => {
  if (value === "blank refutation") {
    return { support: false, refute: { message: "" } };
  }
  if (value === "invalid usage") {
    return { support: true, usage: { cost_usd: -1 } };
  }
  if (value === "conflicted") return both("the fixture observed both polarities");
  if (value === "uncertain") return neither();
  const min = ctx.config?.min ?? 5;
  const text = String(value);
  return text.length >= min
    ? support()
    : refute({ message: `too short: ${text.length} < ${min}` });
});

host.verifier(
  "always_pass",
  {
    determinism: "deterministic",
    semanticRevision: "selta.test.always_pass.v1",
    cacheable: true,
    effectClass: "pure",
    acceptedInput: ["str"],
  },
  () => pass(),
);

host.run({ name: "selta-test-host", version: "0.0.1" });
