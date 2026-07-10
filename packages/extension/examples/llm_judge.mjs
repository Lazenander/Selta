// Skeleton of an llm_judge extension. How the judge reaches its verdict is
// outside Selta's scope (docs/05); this shows only the contract. Two rules
// worth keeping from docs/05: treat the value as untrusted data, and keep
// executions independent — that is what makes voting statistics honest.

import { host, pass, fail } from "@selta/extension";

host.verifier(
  "llm_judge",
  {
    determinism: "nondeterministic",
    configSchema: {
      type: "object",
      fields: { question: { type: "str" } },
    },
    needs: ["env"],
  },
  async (value, ctx) => {
    // const review = await callYourModel(ctx.config.question, value, ctx.env);
    const review = { ok: false, critique: "wire your model call here" };
    return review.ok
      ? pass()
      : fail({ message: review.critique });
  },
);

host.run({ name: "example-judge-host", version: "0.1.0" });
