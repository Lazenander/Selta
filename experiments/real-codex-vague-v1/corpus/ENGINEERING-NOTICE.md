# Engineering-corpus notice

`dev.inputs.jsonl` and `dev.oracle.jsonl` are an author-constructed development
fixture. They are not the three-human-annotated development split specified by
the formal pilot protocol.

The cases are synthetic and were written together with their expected evidence
spans. Consequently, the oracle is not independent of corpus construction and
cannot support a claim about semantic accuracy, prompt quality, human agreement,
real-world prevalence, calibration, or the value of Selta's evidence design.

Permitted uses are limited to engineering work such as:

- JSONL parsing and case/oracle joins;
- structured-output and Selta-admission plumbing;
- exact-span and four-state derivation checks;
- scorer and report-format development; and
- non-authoritative smoke runs whose results remain labelled engineering-only.

It must not be pooled with, substituted for, or described as the formal pilot
corpus. A formal run still requires newly sourced cases, three blinded human
annotations per case, retained disagreement, adjudication, a group-separated
development/held-out split, and a withheld oracle procedure.

The separately prepared engineering heldout inputs, oracle, and nonce were
withheld outside the checkout through prompt selection and model execution.
After the complete prediction bundle was committed and pushed as the pre-reveal
receipt, the exact disclosed bytes were commitment-verified, scored, and
preserved here as `holdout.inputs.jsonl`, `holdout.oracle.jsonl`, and
`holdout.nonce`. Pre-development synthetic smoke calibration clarified the
baseline prompt's general byte-fidelity and no-repeat invariants; it did not use
these cases or held-out data. No semantic correctness, polarity, or selection
threshold informed that wording, and no held-out result may revise it. The
held-out oracle is also author-constructed engineering data and inherits every
non-claim above.

## Constructed balance

The development fixture has 24 cases across exactly two domain families:
`service_integrity` and `software_reliability`. Each of the four derived states
has six cases, split into three `clear` and three `boundary` cases. Each domain
has 12 cases and six cases at each clarity level.

The disclosed held-out fixture likewise has 24 cases and six cases per derived
state. Its independent domain labels are `medical_scheduling` and
`open_source_governance`, with 12 cases per domain and 12 cases at each clarity
level.

Only `claim` and `text` are assessor payload. `id`, `domain_family`, and
`clarity` are harness metadata and must not be included in a model prompt.
