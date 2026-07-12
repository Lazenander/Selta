use crate::{
    validate_predictions, Case, Evidence, EvidenceState, Oracle, Outcome, Prediction,
    SeltaContract, TokenUsage,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
pub struct Evaluation {
    pub version: u32,
    pub binding: ScoreBinding,
    pub oracle_sha256: String,
    pub prompts: BTreeMap<String, PromptMetrics>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScoreBinding {
    pub inputs_sha256: String,
    pub predictions_sha256: String,
    pub manifest_sha256: String,
    pub jobs_sha256: String,
    pub completion_sha256: String,
    pub raw_index_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PromptMetrics {
    pub prompt_sha256: String,
    pub overall: MetricSummary,
    pub by_clarity: BTreeMap<String, MetricSummary>,
    pub by_domain_family: BTreeMap<String, MetricSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MetricSummary {
    pub cases: usize,
    pub response_contract_rate: f64,
    pub exact_membership_given_contract_rate: f64,
    pub fully_mechanically_valid_rate: f64,
    pub operational_error_rate: f64,
    pub aligned_state_accuracy: f64,
    pub exact_evidence_set_accuracy: f64,
    pub macro_f1: f64,
    pub confusion: BTreeMap<String, BTreeMap<String, u64>>,
    pub per_state: BTreeMap<String, PrecisionRecallF1>,
    pub support_presence: PrecisionRecallF1,
    pub refutation_presence: PrecisionRecallF1,
    pub support_span_alignment: PrecisionRecallF1,
    pub refutation_span_alignment: PrecisionRecallF1,
    pub raw_presence_state: RawStateMetrics,
    pub aligned_decisive_coverage: f64,
    pub aligned_decisive_risk: Option<f64>,
    pub decisive_gold_recall: Option<f64>,
    pub both_recall: Option<f64>,
    pub neither_recall: Option<f64>,
    pub operational_errors: BTreeMap<String, u64>,
    pub usage: UsageSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct RawStateMetrics {
    pub exact_state_accuracy: f64,
    pub macro_f1: f64,
    pub confusion: BTreeMap<String, BTreeMap<String, u64>>,
    pub per_state: BTreeMap<String, PrecisionRecallF1>,
    pub support_presence: PrecisionRecallF1,
    pub refutation_presence: PrecisionRecallF1,
    pub decisive_coverage: f64,
    pub decisive_risk: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PrecisionRecallF1 {
    pub true_positive: u64,
    pub false_positive: u64,
    pub false_negative: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct UsageSummary {
    pub calls_with_usage: usize,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub mean_input_tokens: Option<f64>,
    pub mean_output_tokens: Option<f64>,
    pub mean_latency_ms: f64,
}

struct Row<'a> {
    case: &'a Case,
    gold: EvidenceState,
    predicted: Option<EvidenceState>,
    raw_predicted: Option<EvidenceState>,
    semantic_unaligned: bool,
    exact_evidence_set: bool,
    support_predicted_spans: u64,
    support_oracle_spans: u64,
    support_aligned_spans: u64,
    refute_predicted_spans: u64,
    refute_oracle_spans: u64,
    refute_aligned_spans: u64,
    error_class: Option<&'a str>,
    response_contract_valid: bool,
    usage: Option<&'a TokenUsage>,
    latency_ms: u64,
}

pub fn score(
    cases: &[Case],
    predictions: &[Prediction],
    oracles: &[Oracle],
    oracle_sha256: String,
    contract: &SeltaContract,
    binding: ScoreBinding,
) -> Result<Evaluation> {
    validate_predictions(cases, predictions, contract)?;

    let oracle_by_id: BTreeMap<_, _> = oracles
        .iter()
        .map(|oracle| (oracle.id.as_str(), oracle))
        .collect();
    let mut prompt_ids = BTreeSet::new();
    let mut prediction_by_pair = BTreeMap::new();
    let mut prompt_digests = BTreeMap::new();
    for prediction in predictions {
        prompt_ids.insert(prediction.prompt_id.as_str());
        prediction_by_pair.insert(
            (prediction.prompt_id.as_str(), prediction.case_id.as_str()),
            prediction,
        );
        match prompt_digests.insert(
            prediction.prompt_id.as_str(),
            prediction.prompt_sha256.as_str(),
        ) {
            Some(previous) if previous != prediction.prompt_sha256 => {
                bail!("prompt `{}` has inconsistent digests", prediction.prompt_id)
            }
            _ => {}
        }
    }

    let mut prompts = BTreeMap::new();
    for prompt_id in prompt_ids {
        let rows: Vec<_> = cases
            .iter()
            .map(|case| {
                let oracle = *oracle_by_id
                    .get(case.id.as_str())
                    .with_context(|| format!("oracle is missing case `{}`", case.id))?;
                let prediction = prediction_by_pair
                    .get(&(prompt_id, case.id.as_str()))
                    .with_context(|| {
                        format!(
                            "prompt `{prompt_id}` is missing a prediction for case `{}`",
                            case.id
                        )
                    })?;
                let error_class = match &prediction.outcome {
                    Outcome::Admitted { .. } => None,
                    Outcome::OperationalError { class, .. } => Some(class.as_str()),
                };
                let (raw_predicted, predicted, semantic_unaligned, exact_evidence_set, spans) =
                    match &prediction.outcome {
                        Outcome::Admitted { state, response } => {
                            let support = aligned_count(&response.support, &oracle.support);
                            let refute = aligned_count(&response.refute, &oracle.refute);
                            let aligned = support == response.support.len()
                                && refute == response.refute.len();
                            (
                                Some(*state),
                                aligned.then_some(*state),
                                !aligned,
                                exact_evidence_set(response, oracle),
                                (
                                    response.support.len(),
                                    oracle.support.len(),
                                    support,
                                    response.refute.len(),
                                    oracle.refute.len(),
                                    refute,
                                ),
                            )
                        }
                        Outcome::OperationalError { .. } => (
                            None,
                            None,
                            false,
                            false,
                            (0, oracle.support.len(), 0, 0, oracle.refute.len(), 0),
                        ),
                    };
                Ok(Row {
                    case,
                    gold: oracle.state,
                    predicted,
                    raw_predicted,
                    semantic_unaligned,
                    exact_evidence_set,
                    support_predicted_spans: spans.0 as u64,
                    support_oracle_spans: spans.1 as u64,
                    support_aligned_spans: spans.2 as u64,
                    refute_predicted_spans: spans.3 as u64,
                    refute_oracle_spans: spans.4 as u64,
                    refute_aligned_spans: spans.5 as u64,
                    error_class,
                    response_contract_valid: prediction.response_contract_valid,
                    usage: prediction.usage.as_ref(),
                    latency_ms: prediction.latency_ms,
                })
            })
            .collect::<Result<_>>()?;

        let by_clarity = group_summaries(&rows, |row| row.case.clarity.as_deref());
        let by_domain_family = group_summaries(&rows, |row| Some(row.case.domain_family.as_str()));
        prompts.insert(
            prompt_id.to_owned(),
            PromptMetrics {
                prompt_sha256: prompt_digests[prompt_id].to_owned(),
                overall: summarize(&rows),
                by_clarity,
                by_domain_family,
            },
        );
    }

    Ok(Evaluation {
        version: 1,
        binding,
        oracle_sha256,
        prompts,
    })
}

fn group_summaries<'a, F>(rows: &'a [Row<'a>], key: F) -> BTreeMap<String, MetricSummary>
where
    F: Fn(&Row<'a>) -> Option<&'a str>,
{
    let mut groups: BTreeMap<String, Vec<&Row<'a>>> = BTreeMap::new();
    for row in rows {
        if let Some(key) = key(row) {
            groups.entry(key.to_owned()).or_default().push(row);
        }
    }
    groups
        .into_iter()
        .map(|(key, rows)| {
            let borrowed: Vec<_> = rows.into_iter().map(clone_row).collect();
            (key, summarize(&borrowed))
        })
        .collect()
}

fn clone_row<'a>(row: &Row<'a>) -> Row<'a> {
    Row {
        case: row.case,
        gold: row.gold,
        predicted: row.predicted,
        raw_predicted: row.raw_predicted,
        semantic_unaligned: row.semantic_unaligned,
        exact_evidence_set: row.exact_evidence_set,
        support_predicted_spans: row.support_predicted_spans,
        support_oracle_spans: row.support_oracle_spans,
        support_aligned_spans: row.support_aligned_spans,
        refute_predicted_spans: row.refute_predicted_spans,
        refute_oracle_spans: row.refute_oracle_spans,
        refute_aligned_spans: row.refute_aligned_spans,
        error_class: row.error_class,
        response_contract_valid: row.response_contract_valid,
        usage: row.usage,
        latency_ms: row.latency_ms,
    }
}

fn summarize(rows: &[Row<'_>]) -> MetricSummary {
    let total = rows.len() as u64;
    let mechanically_exact = rows
        .iter()
        .filter(|row| row.raw_predicted.is_some())
        .count() as u64;
    let contract_valid = rows
        .iter()
        .filter(|row| row.response_contract_valid)
        .count() as u64;
    let correct = rows
        .iter()
        .filter(|row| row.predicted == Some(row.gold))
        .count() as u64;

    let mut confusion = BTreeMap::new();
    for gold in EvidenceState::ALL {
        let mut columns = BTreeMap::new();
        for predicted in EvidenceState::ALL {
            columns.insert(predicted.as_str().to_owned(), 0);
        }
        columns.insert("semantic_unaligned".to_owned(), 0);
        columns.insert("operational_error".to_owned(), 0);
        confusion.insert(gold.as_str().to_owned(), columns);
    }
    for row in rows {
        let column = if row.semantic_unaligned {
            "semantic_unaligned"
        } else {
            row.predicted
                .map(EvidenceState::as_str)
                .unwrap_or("operational_error")
        };
        *confusion
            .get_mut(row.gold.as_str())
            .expect("all gold states are initialized")
            .get_mut(column)
            .expect("all prediction columns are initialized") += 1;
    }

    let per_state: BTreeMap<_, _> = EvidenceState::ALL
        .into_iter()
        .map(|state| {
            let counts = binary_counts(
                rows,
                |row| row.gold == state,
                |row| row.predicted == Some(state),
            );
            (state.as_str().to_owned(), counts)
        })
        .collect();
    let macro_f1 = per_state.values().map(|metric| metric.f1).sum::<f64>() / 4.0;

    let support_presence = binary_counts(
        rows,
        |row| row.gold.supports(),
        |row| row.predicted.is_some_and(EvidenceState::supports),
    );
    let refutation_presence = binary_counts(
        rows,
        |row| row.gold.refutes(),
        |row| row.predicted.is_some_and(EvidenceState::refutes),
    );
    let support_span_alignment = prf(
        rows.iter().map(|row| row.support_aligned_spans).sum(),
        rows.iter().map(|row| row.support_predicted_spans).sum(),
        rows.iter().map(|row| row.support_oracle_spans).sum(),
    );
    let refutation_span_alignment = prf(
        rows.iter().map(|row| row.refute_aligned_spans).sum(),
        rows.iter().map(|row| row.refute_predicted_spans).sum(),
        rows.iter().map(|row| row.refute_oracle_spans).sum(),
    );
    let exact_evidence_sets = rows.iter().filter(|row| row.exact_evidence_set).count() as u64;
    let raw_presence_state = raw_state_metrics(rows);

    let predicted_decisive = rows
        .iter()
        .filter(|row| row.predicted.is_some_and(EvidenceState::is_decisive))
        .count() as u64;
    let wrong_decisive = rows
        .iter()
        .filter(|row| {
            row.predicted.is_some_and(EvidenceState::is_decisive) && row.predicted != Some(row.gold)
        })
        .count() as u64;
    let decisive_gold = rows.iter().filter(|row| row.gold.is_decisive()).count() as u64;
    let decisive_gold_correct = rows
        .iter()
        .filter(|row| row.gold.is_decisive() && row.predicted == Some(row.gold))
        .count() as u64;

    let both_gold = rows
        .iter()
        .filter(|row| row.gold == EvidenceState::Both)
        .count() as u64;
    let both_correct = rows
        .iter()
        .filter(|row| row.gold == EvidenceState::Both && row.predicted == Some(EvidenceState::Both))
        .count() as u64;
    let neither_gold = rows
        .iter()
        .filter(|row| row.gold == EvidenceState::Neither)
        .count() as u64;
    let neither_correct = rows
        .iter()
        .filter(|row| {
            row.gold == EvidenceState::Neither && row.predicted == Some(EvidenceState::Neither)
        })
        .count() as u64;

    let mut operational_errors = BTreeMap::new();
    for class in rows.iter().filter_map(|row| row.error_class) {
        *operational_errors.entry(class.to_owned()).or_insert(0) += 1;
    }

    let calls_with_usage = rows.iter().filter(|row| row.usage.is_some()).count();
    let input_tokens = rows
        .iter()
        .filter_map(|row| row.usage)
        .map(|usage| usage.input_tokens)
        .sum();
    let cached_input_tokens = rows
        .iter()
        .filter_map(|row| row.usage)
        .filter_map(|usage| usage.cached_input_tokens)
        .sum();
    let output_tokens = rows
        .iter()
        .filter_map(|row| row.usage)
        .map(|usage| usage.output_tokens)
        .sum();
    let latency = rows.iter().map(|row| row.latency_ms).sum::<u64>();

    MetricSummary {
        cases: rows.len(),
        response_contract_rate: ratio(contract_valid, total).unwrap_or(0.0),
        exact_membership_given_contract_rate: ratio(mechanically_exact, contract_valid)
            .unwrap_or(0.0),
        fully_mechanically_valid_rate: ratio(mechanically_exact, total).unwrap_or(0.0),
        operational_error_rate: ratio(total - mechanically_exact, total).unwrap_or(0.0),
        aligned_state_accuracy: ratio(correct, total).unwrap_or(0.0),
        exact_evidence_set_accuracy: ratio(exact_evidence_sets, total).unwrap_or(0.0),
        macro_f1,
        confusion,
        per_state,
        support_presence,
        refutation_presence,
        support_span_alignment,
        refutation_span_alignment,
        raw_presence_state,
        aligned_decisive_coverage: ratio(predicted_decisive, total).unwrap_or(0.0),
        aligned_decisive_risk: ratio(wrong_decisive, predicted_decisive),
        decisive_gold_recall: ratio(decisive_gold_correct, decisive_gold),
        both_recall: ratio(both_correct, both_gold),
        neither_recall: ratio(neither_correct, neither_gold),
        operational_errors,
        usage: UsageSummary {
            calls_with_usage,
            input_tokens,
            cached_input_tokens,
            output_tokens,
            mean_input_tokens: ratio(input_tokens, calls_with_usage as u64),
            mean_output_tokens: ratio(output_tokens, calls_with_usage as u64),
            mean_latency_ms: ratio(latency, total).unwrap_or(0.0),
        },
    }
}

fn binary_counts<G, P>(rows: &[Row<'_>], gold: G, predicted: P) -> PrecisionRecallF1
where
    G: Fn(&Row<'_>) -> bool,
    P: Fn(&Row<'_>) -> bool,
{
    let mut true_positive = 0;
    let mut false_positive = 0;
    let mut false_negative = 0;
    for row in rows {
        match (gold(row), predicted(row)) {
            (true, true) => true_positive += 1,
            (false, true) => false_positive += 1,
            (true, false) => false_negative += 1,
            (false, false) => {}
        }
    }
    let precision = ratio(true_positive, true_positive + false_positive).unwrap_or(0.0);
    let recall = ratio(true_positive, true_positive + false_negative).unwrap_or(0.0);
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    PrecisionRecallF1 {
        true_positive,
        false_positive,
        false_negative,
        precision,
        recall,
        f1,
    }
}

fn prf(matches: u64, predicted: u64, gold: u64) -> PrecisionRecallF1 {
    let precision = ratio(matches, predicted).unwrap_or(0.0);
    let recall = ratio(matches, gold).unwrap_or(0.0);
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    PrecisionRecallF1 {
        true_positive: matches,
        false_positive: predicted.saturating_sub(matches),
        false_negative: gold.saturating_sub(matches),
        precision,
        recall,
        f1,
    }
}

fn raw_state_metrics(rows: &[Row<'_>]) -> RawStateMetrics {
    let decisive = rows
        .iter()
        .filter(|row| row.raw_predicted.is_some_and(EvidenceState::is_decisive))
        .count() as u64;
    let wrong_decisive = rows
        .iter()
        .filter(|row| {
            row.raw_predicted.is_some_and(EvidenceState::is_decisive)
                && row.raw_predicted != Some(row.gold)
        })
        .count() as u64;
    let mut confusion = BTreeMap::new();
    for gold in EvidenceState::ALL {
        let mut columns = BTreeMap::new();
        for predicted in EvidenceState::ALL {
            columns.insert(predicted.as_str().to_owned(), 0);
        }
        columns.insert("operational_error".to_owned(), 0);
        confusion.insert(gold.as_str().to_owned(), columns);
    }
    for row in rows {
        let column = row
            .raw_predicted
            .map(EvidenceState::as_str)
            .unwrap_or("operational_error");
        *confusion
            .get_mut(row.gold.as_str())
            .expect("gold state exists")
            .get_mut(column)
            .expect("raw prediction column exists") += 1;
    }
    let per_state: BTreeMap<_, _> = EvidenceState::ALL
        .into_iter()
        .map(|state| {
            (
                state.as_str().to_owned(),
                binary_counts(
                    rows,
                    |row| row.gold == state,
                    |row| row.raw_predicted == Some(state),
                ),
            )
        })
        .collect();
    RawStateMetrics {
        exact_state_accuracy: ratio(
            rows.iter()
                .filter(|row| row.raw_predicted == Some(row.gold))
                .count() as u64,
            rows.len() as u64,
        )
        .unwrap_or(0.0),
        macro_f1: per_state.values().map(|metric| metric.f1).sum::<f64>() / 4.0,
        confusion,
        per_state,
        support_presence: binary_counts(
            rows,
            |row| row.gold.supports(),
            |row| row.raw_predicted.is_some_and(EvidenceState::supports),
        ),
        refutation_presence: binary_counts(
            rows,
            |row| row.gold.refutes(),
            |row| row.raw_predicted.is_some_and(EvidenceState::refutes),
        ),
        decisive_coverage: ratio(decisive, rows.len() as u64).unwrap_or(0.0),
        decisive_risk: ratio(wrong_decisive, decisive),
    }
}

fn exact_evidence_set(predicted: &Evidence, oracle: &Oracle) -> bool {
    fn set(spans: &[String]) -> BTreeSet<&str> {
        spans.iter().map(String::as_str).collect()
    }
    set(&predicted.support) == set(&oracle.support) && set(&predicted.refute) == set(&oracle.refute)
}

fn aligned_count(predicted: &[String], oracle: &[String]) -> usize {
    fn search(predicted: &[String], oracle: &[String], index: usize, used: u8) -> usize {
        if index == predicted.len() {
            return 0;
        }
        let mut best = search(predicted, oracle, index + 1, used);
        for (oracle_index, gold) in oracle.iter().enumerate() {
            let bit = 1_u8 << oracle_index;
            if used & bit == 0 && predicted[index].contains(gold) {
                best = best.max(1 + search(predicted, oracle, index + 1, used | bit));
            }
        }
        best
    }
    search(predicted, oracle, 0, 0)
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator != 0).then_some(numerator as f64 / denominator as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AttemptRole, Evidence, RawArtifacts};

    fn binding() -> ScoreBinding {
        ScoreBinding {
            inputs_sha256: "1".repeat(64),
            predictions_sha256: "2".repeat(64),
            manifest_sha256: "3".repeat(64),
            jobs_sha256: "4".repeat(64),
            completion_sha256: "5".repeat(64),
            raw_index_sha256: "6".repeat(64),
        }
    }

    fn prediction(id: &str, predicted: Option<EvidenceState>) -> Prediction {
        let outcome = match predicted {
            Some(state) => Outcome::Admitted {
                state,
                response: Evidence {
                    support: state
                        .supports()
                        .then(|| "support".into())
                        .into_iter()
                        .collect(),
                    refute: state
                        .refutes()
                        .then(|| "refute".into())
                        .into_iter()
                        .collect(),
                },
            },
            None => Outcome::OperationalError {
                class: "timeout".into(),
                message: "timed out".into(),
            },
        };
        Prediction {
            attempt_id: format!("a-{id}"),
            attempt_role: AttemptRole::ScoredFirstAttempt,
            job_index: 0,
            case_id: id.into(),
            prompt_id: "p0".into(),
            prompt_sha256: "a".repeat(64),
            request_sha256: "b".repeat(64),
            response_contract_valid: predicted.is_some(),
            outcome,
            latency_ms: 10,
            usage: Some(TokenUsage {
                input_tokens: 20,
                cached_input_tokens: Some(5),
                output_tokens: 4,
            }),
            raw: RawArtifacts::default(),
        }
    }

    #[test]
    fn operational_failure_is_a_miss_not_neither() {
        let contract =
            SeltaContract::from_source(include_bytes!("../../schemas/response.selta.json"))
                .unwrap();
        let cases = vec![Case {
            id: "one".into(),
            domain_family: "domain".into(),
            clarity: Some("clear".into()),
            claim: "claim".into(),
            text: "support refute".into(),
        }];
        let oracles = vec![Oracle {
            id: "one".into(),
            state: EvidenceState::SupportOnly,
            support: vec!["support".into()],
            refute: vec![],
        }];
        let evaluation = score(
            &cases,
            &[prediction("one", None)],
            &oracles,
            "oracle".into(),
            &contract,
            binding(),
        )
        .expect("score");
        let metrics = &evaluation.prompts["p0"].overall;
        assert_eq!(metrics.aligned_state_accuracy, 0.0);
        assert_eq!(metrics.confusion["support_only"]["operational_error"], 1);
        assert_eq!(metrics.confusion["support_only"]["neither"], 0);
        assert_eq!(metrics.decisive_gold_recall, Some(0.0));
    }

    #[test]
    fn perfect_four_state_predictions_score_one() {
        let contract =
            SeltaContract::from_source(include_bytes!("../../schemas/response.selta.json"))
                .unwrap();
        let mut cases = Vec::new();
        let mut oracles = Vec::new();
        let mut predictions = Vec::new();
        for (index, state) in EvidenceState::ALL.into_iter().enumerate() {
            let id = state.as_str();
            cases.push(Case {
                id: id.into(),
                domain_family: "domain".into(),
                clarity: Some("clear".into()),
                claim: "claim".into(),
                text: "support refute".into(),
            });
            oracles.push(Oracle {
                id: id.into(),
                state,
                support: state
                    .supports()
                    .then(|| "support".into())
                    .into_iter()
                    .collect(),
                refute: state
                    .refutes()
                    .then(|| "refute".into())
                    .into_iter()
                    .collect(),
            });
            let mut prediction = prediction(id, Some(state));
            prediction.job_index = index;
            predictions.push(prediction);
        }
        let evaluation = score(
            &cases,
            &predictions,
            &oracles,
            "oracle".into(),
            &contract,
            binding(),
        )
        .expect("score");
        let metrics = &evaluation.prompts["p0"].overall;
        assert_eq!(metrics.aligned_state_accuracy, 1.0);
        assert_eq!(metrics.macro_f1, 1.0);
        assert_eq!(metrics.support_presence.f1, 1.0);
        assert_eq!(metrics.refutation_presence.f1, 1.0);
    }

    #[test]
    fn unaligned_exact_quote_is_raw_support_but_primary_semantic_miss() {
        let contract =
            SeltaContract::from_source(include_bytes!("../../schemas/response.selta.json"))
                .unwrap();
        let cases = vec![Case {
            id: "one".into(),
            domain_family: "domain".into(),
            clarity: Some("clear".into()),
            claim: "claim".into(),
            text: "oracle evidence; unrelated quote".into(),
        }];
        let oracles = vec![Oracle {
            id: "one".into(),
            state: EvidenceState::SupportOnly,
            support: vec!["oracle evidence".into()],
            refute: vec![],
        }];
        let mut prediction = prediction("one", Some(EvidenceState::SupportOnly));
        if let Outcome::Admitted { response, .. } = &mut prediction.outcome {
            response.support = vec!["unrelated quote".into()];
        }
        let evaluation = score(
            &cases,
            &[prediction],
            &oracles,
            "oracle".into(),
            &contract,
            binding(),
        )
        .unwrap();
        let metrics = &evaluation.prompts["p0"].overall;
        assert_eq!(metrics.confusion["support_only"]["semantic_unaligned"], 1);
        assert_eq!(
            metrics.raw_presence_state.confusion["support_only"]["support_only"],
            1
        );
        assert_eq!(metrics.macro_f1, 0.0);
        assert_eq!(metrics.decisive_gold_recall, Some(0.0));
    }

    #[test]
    fn span_alignment_is_asymmetric_and_one_to_one() {
        assert_eq!(
            aligned_count(&["prefix minimal suffix".into()], &["minimal".into()]),
            1
        );
        assert_eq!(
            aligned_count(&["minimal".into()], &["minimal sufficient span".into()]),
            0
        );
        assert_eq!(
            aligned_count(
                &["prefix minimal".into(), "minimal suffix".into()],
                &["minimal".into()]
            ),
            1
        );
    }
}
