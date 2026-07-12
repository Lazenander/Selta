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

Separately prepared engineering heldout inputs, oracle, and nonce are withheld
outside the checkout. Only `holdout.commitment.json` remains visible before the
prompt freeze. The baseline prompt has been amended only with the general
cross-polarity quotation-disjointness rule and has not been run against these
cases.

## Constructed balance

The fixture has 24 cases across exactly two domain families:
`service_integrity` and `software_reliability`. Each of the four derived states
has six cases, split into three `clear` and three `boundary` cases. Each domain
has 12 cases and six cases at each clarity level.

Only `claim` and `text` are assessor payload. `id`, `domain_family`, and
`clarity` are harness metadata and must not be included in a model prompt.
