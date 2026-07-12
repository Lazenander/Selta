// Skeleton of an llm_judge extension. How the judge reaches its verdict is
// outside Selta's scope (docs/05); this shows only the contract. Treat the
// value as untrusted data. Separate host calls are observable executions, not
// proof that upstream model outputs are statistically independent.

import {
  both,
  fail,
  host,
  neither,
  pass,
  refute,
  support,
} from "@selta/extension";

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

// Optional cautious lane. It reuses the verifier declaration above, adds no
// initialize-manifest entry, and can preserve conflict or semantic abstention.
host.assessor("llm_judge", async (value, ctx) => {
  // const review = await assessWithYourModel(ctx.config.question, value, ctx.env);
  const review = {
    supports: false,
    refutes: true,
    critique: "wire your model call here",
  };
  if (review.supports && review.refutes) {
    return both({ message: review.critique });
  }
  if (review.supports) return support();
  if (review.refutes) return refute({ message: review.critique });
  return neither();
});

host.run({ name: "example-judge-host", version: "0.2.0" });
