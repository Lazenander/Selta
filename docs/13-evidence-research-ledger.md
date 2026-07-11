# 13 — Evidence research ledger

> **Status: initial, non-normative ledger.** A cited result constrains Selta
> only through the transfer argument recorded here. This is a starting set, not
> a claim that one existing theory solves epistemic composition.

## Why a ledger

The motivating failure is not well described as a curse of dimensionality.
Several distinct mechanisms can produce the same visible symptom:

- correlated assessors repeatedly make the same mistake;
- genuine ambiguity is collapsed into one synthetic ground truth;
- a stopping or inclusion rule selects favorable observations;
- an evaluator is optimized as a proxy for the real criterion;
- support and conflict are lost when evidence is projected too early; or
- operational success is mistaken for epistemic authority.

The research track should combine constraints from several mature fields while
keeping their assumptions separate.

## Boundary results

| Source | Result in its own domain | Transfer constraint for Selta | Limit of transfer |
|---|---|---|---|
| [Rice, *Classes of Recursively Enumerable Sets and Their Decision Problems* (1953)](https://www.ams.org/journals/tran/1953-074-02/S0002-9947-1953-0053041-6/S0002-9947-1953-0053041-6.pdf) | Non-trivial extensional properties of recursively enumerable sets do not have a general decision procedure. | Do not promise a sound, complete, terminating decider for arbitrary program semantics. | This does not prevent decidable restricted languages, proof-carrying witnesses, conservative analyses, or useful fallible assessors. |
| [List and Pettit, *Aggregating Sets of Judgments: An Impossibility Result* (2002)](https://www.cambridge.org/core/journals/economics-and-philosophy/article/aggregating-sets-of-judgments-an-impossibility-result/35BB2A979DC8D2548B3040A1757B058B) | Interconnected rational judgments cannot be aggregated under all of a stated set of natural conditions on an unrestricted domain. | Every aggregation profile must declare which domain or desideratum it restricts; there is no universal coherent vote operator. | The theorem has exact axioms. It does not prohibit useful rules on restricted agendas. |
| [Blackwell, *Equivalent Comparisons of Experiments* (1953)](https://projecteuclid.org/journals/annals-of-mathematical-statistics/volume-24/issue-2/Equivalent-Comparisons-of-Experiments/10.1214/aoms/1177729032.full) | Statistical experiments can be ordered by informativeness; garbling does not make an experiment more informative for every decision problem. | A deterministic restatement or post-processing of the same observation cannot be assumed to add information. Define a candidate evidence-information preorder before claiming amplification. | Applying Blackwell order directly requires a statistical experiment and decision model. Opaque testimony needs an explicit bridge. |
| [Gao, Schulman, and Hilton, *Scaling Laws for Reward Model Overoptimization* (2023)](https://proceedings.mlr.press/v202/gao23h.html) | In their synthetic reward-model setup, stronger optimization of an imperfect proxy can reduce gold-model performance. | Retries, best-of-N search, and repair against a verifier must be evaluated for proxy exploitation on hidden authority data. | A gold reward model is itself a proxy for humans; the empirical law is not universal across tasks. |

## Disagreement, voting, and abstention

| Source | Result in its own domain | Transfer constraint for Selta | Limit of transfer |
|---|---|---|---|
| [Pavlick and Kwiatkowski, *Inherent Disagreements in Human Textual Inferences* (2019)](https://aclanthology.org/Q19-1043/) | Some NLI disagreements persist with more ratings and context, and model uncertainty does not reproduce the human distribution. | Preserve individual judgments and their distribution; do not define all disagreement as annotation noise. | The evidence is task-specific and does not make every disagreement irreducible. |
| [Davani, Díaz, and Prabhakaran, *Dealing with Disagreements* (2022)](https://aclanthology.org/2022.tacl-1.6/) | Majority aggregation can erase systematic annotator perspectives on subjective tasks; multi-annotator modeling can retain useful structure. | Annotator/source perspective belongs in provenance, and majority is an explicit projection rather than ground-truth construction. | The proposed model is one empirical approach, not a general evidence calculus. |
| [Ladha, *Condorcet's Jury Theorem with Correlated Votes* (1995)](https://doi.org/10.1016/0167-2681(94)00068-P) | Under studied correlated-vote models, majority-vote effectiveness decreases as correlation increases. | Calls, actors, or model names are not independent evidence. Dependence assumptions and common lineage must be bound before vote margin raises assurance. | The theorem uses particular probability models and competence assumptions. |
| [Geifman and El-Yaniv, *SelectiveNet* (2019)](https://proceedings.mlr.press/v97/geifman19a.html) | Selective prediction explicitly optimizes risk over covered examples under a coverage constraint. | `inconclusive` or an unresolved conclusion is a normal policy outcome; evaluations must show risk-coverage curves rather than accuracy only on answered items. | The learned architecture is not proposed for Selta, and guarantees depend on the experimental setting. |
| [Angelopoulos et al., *Conformal Risk Control* (2024)](https://proceedings.iclr.cc/paper_files/paper/2024/hash/f3549ef9b5ff520a7e41ff3cc306ab2b-Abstract-Conference.html) | Expected monotone loss can be controlled under the paper's conformal setup and assumptions. | A risk or coverage guarantee must bind its calibration corpus, loss, level, exchangeability or shift assumptions, and policy revision. | An uncalibrated model score is not conformal evidence, and guarantees do not transfer without their assumptions. |
| [Dawid and Skene, *Maximum Likelihood Estimation of Observer Error-Rates* (1979)](https://doi.org/10.2307/2346806) | A latent-class model estimates observer-specific error rates under a specified probabilistic structure. | Source reliability may be an optional interpretation learned from data, not a universal equal-vote rule. | It assumes a latent truth and conditional structure that can be false for subjective or dependent judgments. |

## Provenance, conflict, and revision

| Source | Result in its own domain | Transfer constraint for Selta | Limit of transfer |
|---|---|---|---|
| [Green, Karvounarakis, and Tannen, *Provenance Semirings* (2007)](https://web.cs.ucdavis.edu/~green/papers/pods07.pdf) | A general polynomial provenance representation can factor several relational interpretations. | Preserve a free derivation form before choosing probability, trust, Boolean, or other quotients; test whether the factorization analogy actually holds for the candidate DSL. | The theorem is for positive relational algebra and Datalog settings, not arbitrary semantic testimony. |
| [W3C PROV-DM (2013)](https://www.w3.org/TR/prov-dm/) | A domain-neutral standard represents entities, activities, agents, derivations, responsibility, and provenance bundles. | Map acquisition attempts, evidence, sources, and derivations to established provenance concepts where possible, then specify stricter validity and canonicalization. | PROV describes provenance; it does not establish that a derivation is epistemically sound or complete. |
| [Belnap, *A Useful Four-Valued Logic* (1977)](https://doi.org/10.1007/978-94-010-1161-7_2) | A four-valued treatment distinguishes support only, refutation only, both, and neither. | Do not collapse conflicting evidence into missing evidence. A bilattice is a candidate interpretation, while K3 can remain a legacy projection. | Belnap's logic is not by itself a provenance, trust, sampling, or acquisition model. |
| [Doyle, *A Truth Maintenance System* (1979)](https://doi.org/10.1016/0004-3702(79)90008-0) | Recorded reasons and assumptions support explanation and belief revision. | Keep evidence immutable while allowing conclusions to be recomputed when assumptions, trust, or rules change. | A TMS manages justified beliefs; it does not certify observations or solve statistical dependence. |
| [Dung, *On the Acceptability of Arguments* (1995)](https://doi.org/10.1016/0004-3702(94)00041-X) | Abstract attack relations support several non-monotonic acceptability semantics. | Incompatible claims, attacks, and the chosen acceptance semantics must be explicit rather than hidden in a verifier prompt. | Abstract argumentation omits acquisition, source reliability, and payload truth unless extended. |
| [Zadeh, *A Simple View of the Dempster–Shafer Theory* (1986)](https://doi.org/10.1609/aimag.v7i2.542) | The paper exposes counterintuitive certainty from a normalization rule under highly conflicting evidence. | Preserve conflict and forbid a default fusion rule; each combination operator declares dependence and normalization assumptions. | The example criticizes a particular evidence combination, not every probabilistic or belief-function interpretation. |

## Acquisition, missingness, and adaptive stopping

This is the first S1 research priority because an internally perfect evidence
graph can still be misleading when its acquisition boundary selects the
observations it exposes.

| Source | Result in its own domain | Transfer constraint for Selta | Limit of transfer |
|---|---|---|---|
| [Rubin, *Inference and Missing Data* (1976)](https://doi.org/10.1093/biomet/63.3.581) | Likelihood or Bayesian handling of missing data depends on the sampling and missingness mechanism and on conditions under which it is ignorable. | Missing attempts cannot be interpreted without a versioned acquisition and missingness model; `unavailable` is not automatically neutral or negative. | Selta observations need not satisfy Rubin's statistical model, so the exact categories are candidates rather than universal enums. |
| [Johari et al., *Always Valid Inference: Continuous Monitoring of A/B Tests* (2021)](https://doi.org/10.1287/opre.2021.2135) | Conventional fixed-sample inference becomes unreliable under endogenous continuous monitoring; the paper constructs always-valid sequential quantities for its setting. | Any confidence claim under adaptive stopping must bind a stopping-safe method or restrict the stopping rule. The number of retries and stop event belong in the basis. | The construction concerns statistical experiments with defined hypotheses, not arbitrary opaque judgments. |
| [Dwork et al., *The Reusable Holdout* (2015)](https://doi.org/10.1126/science.aaa9375) | Repeated adaptive use of holdout feedback can overfit it; the paper gives a mechanism for preserving validity under its assumptions. | Prompt, policy, and verifier tuning must separate development from hidden authority data and record adaptive feedback. | Differential-privacy-based holdout reuse is an evaluation technique, not a proposed core evidence algebra. |
| [RFC 9162, *Certificate Transparency Version 2.0* (2021)](https://www.rfc-editor.org/rfc/rfc9162.html) | Signed append-only Merkle logs support inclusion and consistency auditing for submitted certificates. | An acquisition channel may use append-only commitments to make recorded omissions or rewrites detectable and to bind attempt order. | A transparency log proves properties of submissions to that log; it cannot reveal secret attempts made outside its exclusive or attestable channel. |

## Model-based assessors

| Source | Result in its own domain | Transfer constraint for Selta | Limit of transfer |
|---|---|---|---|
| [Zheng et al., *Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena* (2023)](https://papers.neurips.cc/paper_files/paper/2023/hash/91f18a1287b398d378ef22505bf41832-Abstract-Datasets_and_Benchmarks.html) | Strong model judges can align well with human preference in the studied setup but exhibit position, verbosity, self-enhancement, and reasoning biases. | Judge evidence records model revision, prompt, rubric, ordering, generator relationship, context, and counterfactual controls. | Reported agreement is task- and model-specific; human preference is not correctness for every schema. |
| [Huang et al., *Large Language Models Cannot Self-Correct Reasoning Yet* (2024)](https://openreview.net/forum?id=IkmD3fKBPQ) | In the studied reasoning tasks, intrinsic prompted self-correction often fails or degrades without external feedback. | Another critique using the same information channel is a derivation, not automatically a fresh observation; depth has no default epistemic meaning. | The paper does not prove that all models, prompts, or tasks cannot self-correct. |
| [Kamoi et al., *When Can LLMs Actually Correct Their Own Mistakes?* (2024)](https://aclanthology.org/2024.tacl-1.78/) | The survey finds stronger support for correction with reliable external feedback and identifies frequent evaluation weaknesses. | Compare recursive correction with strong, cost-matched baselines and separately evaluate feedback quality. | It is a critical survey of existing evidence, not a timeless impossibility theorem. |
| [Farquhar et al., *Detecting Hallucinations Using Semantic Entropy* (2024)](https://www.nature.com/articles/s41586-024-07421-0) | Grouping sampled generations by meaning can detect a studied class of confabulations better than sequence-level variation. | Surface paraphrases are not automatically independent atoms; semantic equivalence and common lineage require explicit treatment. | Semantic entropy detects a subset of failures and agreement still does not establish truth. |
| [Verga et al., *Replacing Judges with Juries* (2024)](https://arxiv.org/abs/2404.18796) | Cross-family panels of smaller judges outperform one larger judge in the studied evaluations and reduce measured intra-model bias. | Source diversity is metadata and an empirical hypothesis worth testing, not proof of independence. | The result is an experimental preprint over selected datasets, models, and judge settings. |

## Initial synthesis

The smallest common design obligation supported by this ledger is:

```text
declared acquisition plan
    -> channel-attested attempt trace with scoped completeness claim
    -> immutable provenance-preserving evidence graph
    -> interpretation under explicit trust and assumptions
    -> versioned conclusion and abstention projection
```

Verification depth, number of calls, number of apparent actors, and model-family
labels remain operational facts. They acquire epistemic meaning only through a
declared, testable interpretation whose assumptions and acquisition history are
part of the replay basis.

## Ledger maintenance rule

Every future entry must state:

1. the exact cited result and domain;
2. the assumptions necessary for that result;
3. the proposed Selta constraint;
4. the argument that transfers the result to Selta; and
5. the counterexample or observation that would reject that transfer.

Reviews and surveys may route discovery, but a promotion claim should trace to
primary evidence or a self-contained formal argument.
