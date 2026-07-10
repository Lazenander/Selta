// A judge whose behavior depends on its settings — the M4 acceptance
// fixture: reconfigure at pool scope without touching the schema (docs/07).
import { host, pass, fail } from "../../../../packages/extension/index.js";

host.verifier(
  "model_judge",
  {
    determinism: "nondeterministic",
    settingsSchema: {
      type: "object",
      open: true,
      fields: {
        model: { type: "str" },
        api_key: { type: "str" },
      },
    },
  },
  async (_value, ctx) => {
    if (ctx.settings.api_key !== "sk-test-secret") {
      return fail(`api key was not injected (got '${ctx.settings.api_key ?? ""}')`);
    }
    if (ctx.settings.model === "strict-model") {
      return fail("strict-model rejects everything");
    }
    return pass();
  },
);

host.run({ name: "model-judge-host", version: "0.0.1" });
