use selta_core::{
    AdmissionPolicy, AdmissionProfile, AdmittedNode, CheckResult, Delta, DeltaKind,
    EvidenceProjection, EvidenceState, EvidenceSummary, MetaIssue, MetaIssueCode, Node, Registry,
    Verdict, VerifierSpec, SELTA_META_VALIDATOR_REVISION, SELTA_META_VALIDATOR_REVISION_V1,
    SELTA_META_VALIDATOR_REVISION_V2, SELTA_SCHEMA_LANGUAGE_REVISION,
    SELTA_SCHEMA_LANGUAGE_REVISION_V1, SELTA_SCHEMA_LANGUAGE_REVISION_V2,
};
use serde_json::{json, Value};

const STATES: [EvidenceState; 4] = [
    EvidenceState::Neither,
    EvidenceState::SupportOnly,
    EvidenceState::RefuteOnly,
    EvidenceState::Both,
];

fn admit_at(value: Value, profile: AdmissionProfile) -> Result<AdmittedNode, Vec<MetaIssue>> {
    AdmittedNode::admit_source_at(&serde_json::to_vec(&value).unwrap(), profile)
}

fn has(issues: &[MetaIssue], code: MetaIssueCode, pointer: &str) -> bool {
    issues
        .iter()
        .any(|issue| issue.code == code && issue.pointer == pointer)
}

fn delta(message: &str) -> Delta {
    Delta {
        path: "$.claim".to_string(),
        kind: DeltaKind::Semantic,
        message: message.to_string(),
        expected: None,
        actual: None,
        source: "judge".to_string(),
        data: None,
        votes: None,
    }
}

#[test]
fn evidence_state_has_the_four_presence_values_and_cautious_projection() {
    assert_eq!(
        EvidenceState::from_presence(false, false),
        EvidenceState::Neither
    );
    assert_eq!(
        EvidenceState::from_presence(true, false),
        EvidenceState::SupportOnly
    );
    assert_eq!(
        EvidenceState::from_presence(false, true),
        EvidenceState::RefuteOnly
    );
    assert_eq!(
        EvidenceState::from_presence(true, true),
        EvidenceState::Both
    );
    assert_eq!(EvidenceState::Neither.project(), Verdict::Inconclusive);
    assert_eq!(EvidenceState::SupportOnly.project(), Verdict::Pass);
    assert_eq!(EvidenceState::RefuteOnly.project(), Verdict::Fail);
    assert_eq!(EvidenceState::Both.project(), Verdict::Inconclusive);
}

#[test]
fn knowledge_join_and_truth_operations_obey_the_documented_laws() {
    for a in STATES {
        assert_eq!(a.join(a), a, "join idempotence for {a:?}");
        assert_eq!(a.join(EvidenceState::Neither), a);
        assert_eq!(a.negate().negate(), a);
        for b in STATES {
            assert_eq!(a.join(b), b.join(a), "join commutativity");
            assert_eq!(a.conjoin(b), b.conjoin(a), "conjunction symmetry");
            assert_eq!(a.disjoin(b), b.disjoin(a), "disjunction symmetry");
            assert_eq!(
                a.conjoin(b).negate(),
                a.negate().disjoin(b.negate()),
                "first De Morgan law"
            );
            assert_eq!(
                a.disjoin(b).negate(),
                a.negate().conjoin(b.negate()),
                "second De Morgan law"
            );
            for c in STATES {
                assert_eq!(a.join(b).join(c), a.join(b.join(c)));
                assert_eq!(a.conjoin(b).conjoin(c), a.conjoin(b.conjoin(c)));
                assert_eq!(a.disjoin(b).disjoin(c), a.disjoin(b.disjoin(c)));
            }
        }
    }
}

#[test]
fn evidence_summary_separates_unavailability_and_preserves_refutations() {
    let mut unavailable = EvidenceSummary::default();
    unavailable.record_unavailable().unwrap();
    assert_eq!(unavailable.state, None);
    assert_eq!(unavailable.unavailable, 1);

    let mut summary = EvidenceSummary::default();
    summary.record(EvidenceState::Neither, None).unwrap();
    summary.record(EvidenceState::SupportOnly, None).unwrap();
    summary
        .record(EvidenceState::RefuteOnly, Some(delta("counterexample")))
        .unwrap();
    summary
        .record(EvidenceState::Both, Some(delta("counterexample")))
        .unwrap();
    assert_eq!(summary.state, Some(EvidenceState::Both));
    assert_eq!(summary.neither, 1);
    assert_eq!(summary.support_only, 1);
    assert_eq!(summary.refute_only, 1);
    assert_eq!(summary.both, 1);
    assert_eq!(summary.refutations, vec![delta("counterexample")]);

    summary.checked_join(&unavailable).unwrap();
    assert_eq!(summary.state, Some(EvidenceState::Both));
    assert_eq!(summary.unavailable, 1);
}

#[test]
fn evidence_summary_updates_are_checked_and_transactional() {
    let mut missing = EvidenceSummary::default();
    assert!(missing.record(EvidenceState::Both, None).is_err());
    assert_eq!(missing, EvidenceSummary::default());
    assert!(missing
        .record(EvidenceState::SupportOnly, Some(delta("not allowed")))
        .is_err());
    assert_eq!(missing, EvidenceSummary::default());
    assert!(missing
        .record(EvidenceState::RefuteOnly, Some(delta("  ")))
        .is_err());
    assert_eq!(missing, EvidenceSummary::default());

    let mut full = EvidenceSummary {
        neither: u32::MAX,
        ..EvidenceSummary::default()
    };
    let before = full.clone();
    assert!(full.record(EvidenceState::Neither, None).is_err());
    assert_eq!(full, before);

    let mut left = EvidenceSummary {
        unavailable: u32::MAX,
        ..EvidenceSummary::default()
    };
    let right = EvidenceSummary {
        unavailable: 1,
        ..EvidenceSummary::default()
    };
    let before = left.clone();
    assert!(left.checked_join(&right).is_err());
    assert_eq!(left, before);
}

#[test]
fn semantic_inconclusive_is_not_an_error_or_skip() {
    let check = CheckResult::inconclusive("judge");
    assert_eq!(check.verdict, Verdict::Inconclusive);
    assert_eq!(check.error, None);
    assert!(!check.skipped);
    assert_eq!(check.evidence, None);
    assert!(serde_json::to_value(&check)
        .unwrap()
        .get("evidence")
        .is_none());
}

#[test]
fn evidence_is_an_additive_leaf_field_in_the_typed_schema() {
    let node = Node::from_value(json!({
        "type": "str",
        "verify": [{ "ext": "judge", "evidence": "cautious" }]
    }))
    .unwrap();
    let VerifierSpec::Leaf(leaf) = &node.verify[0] else {
        panic!("fixture must project to a leaf");
    };
    assert_eq!(leaf.evidence, Some(EvidenceProjection::Cautious));
    assert_eq!(
        serde_json::to_value(&node).unwrap()["verify"][0]["evidence"],
        "cautious"
    );
}

#[test]
fn admission_revisions_are_explicit_and_the_legacy_aliases_do_not_move() {
    assert_eq!(
        SELTA_SCHEMA_LANGUAGE_REVISION,
        SELTA_SCHEMA_LANGUAGE_REVISION_V1
    );
    assert_eq!(
        SELTA_META_VALIDATOR_REVISION,
        SELTA_META_VALIDATOR_REVISION_V1
    );
    assert_eq!(
        AdmissionProfile::Selta1.schema_language_revision(),
        "selta.schema-language/1"
    );
    assert_eq!(
        AdmissionProfile::Selta1.meta_validator_revision(),
        "selta.meta-validator/1"
    );
    assert_eq!(
        AdmissionProfile::Selta2.schema_language_revision(),
        SELTA_SCHEMA_LANGUAGE_REVISION_V2
    );
    assert_eq!(
        AdmissionProfile::Selta2.meta_validator_revision(),
        SELTA_META_VALIDATOR_REVISION_V2
    );

    let source = json!({
        "type": "str",
        "verify": [{ "ext": "judge", "evidence": "cautious" }]
    });
    let legacy = admit_at(source.clone(), AdmissionProfile::Selta1).unwrap_err();
    assert!(has(
        &legacy,
        MetaIssueCode::UnexpectedProperty,
        "/verify/0/evidence"
    ));
    let admitted = admit_at(source, AdmissionProfile::Selta2).unwrap();
    assert_eq!(admitted.profile(), AdmissionProfile::Selta2);

    let legacy_plain = admit_at(json!({ "type": "str" }), AdmissionProfile::Selta1).unwrap();
    let current_plain = admit_at(json!({ "type": "str" }), AdmissionProfile::Selta2).unwrap();
    assert_eq!(
        legacy_plain.as_node().ty.name(),
        current_plain.as_node().ty.name()
    );
    assert_ne!(legacy_plain.profile(), current_plain.profile());
}

#[test]
fn revision_two_closes_evidence_values_vote_and_combinator_modes() {
    let wrong = admit_at(
        json!({
            "type": "str",
            "verify": [{ "ext": "judge", "evidence": "majority" }]
        }),
        AdmissionProfile::Selta2,
    )
    .unwrap_err();
    assert!(has(
        &wrong,
        MetaIssueCode::InvalidEvidence,
        "/verify/0/evidence"
    ));

    let vote = admit_at(
        json!({
            "type": "str",
            "verify": [{
                "ext": "judge",
                "evidence": "cautious",
                "sampling": { "samples": 3, "vote": "majority" }
            }]
        }),
        AdmissionProfile::Selta2,
    )
    .unwrap_err();
    assert!(has(
        &vote,
        MetaIssueCode::InvalidEvidence,
        "/verify/0/sampling/vote"
    ));

    let mixed = admit_at(
        json!({
            "type": "str",
            "verify": [{ "all_of": [
                { "ext": "first", "evidence": "cautious" },
                { "ext": "second" }
            ] }]
        }),
        AdmissionProfile::Selta2,
    )
    .unwrap_err();
    assert!(has(&mixed, MetaIssueCode::InvalidEvidence, "/verify/0"));
}

#[test]
fn revision_two_admits_homogeneous_subtrees_and_separate_modes() {
    let homogeneous = admit_at(
        json!({
            "type": "str",
            "verify": [{ "any_of": [
                { "ext": "first", "evidence": "cautious" },
                { "not":
                    { "ext": "second", "evidence": "cautious" },
                    "message": "second must not support"
                }
            ] }]
        }),
        AdmissionProfile::Selta2,
    )
    .expect("homogeneous evidence subtree admits");
    assert_eq!(homogeneous.profile(), AdmissionProfile::Selta2);

    admit_at(
        json!({
            "type": "str",
            "verify": [
                { "ext": "legacy" },
                { "ext": "assessor", "evidence": "cautious" }
            ]
        }),
        AdmissionProfile::Selta2,
    )
    .expect("separate verifier entries may use separate modes");
}

#[test]
fn registry_admission_preserves_the_selected_profile() {
    let registry = Registry::with_pure_builtins();
    let source = serde_json::to_vec(&json!({
        "type": "str",
        "verify": [{ "ext": "non_empty", "evidence": "cautious" }]
    }))
    .unwrap();
    let admitted = registry
        .admit_source_at(
            &source,
            &AdmissionPolicy::pure_only(),
            AdmissionProfile::Selta2,
        )
        .unwrap();
    assert_eq!(admitted.profile(), AdmissionProfile::Selta2);
    assert!(registry
        .admit_source(&source, &AdmissionPolicy::pure_only())
        .is_err());
}

#[test]
fn compatibility_meta_validation_rejects_a_mixed_typed_subtree() {
    let registry = Registry::with_pure_builtins();
    let node = Node::from_value(json!({
        "type": "str",
        "verify": [{ "all_of": [
            { "ext": "non_empty", "evidence": "cautious" },
            { "ext": "non_empty" }
        ] }]
    }))
    .unwrap();
    let errors = selta_core::meta::validate(&node, &registry);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("wholly legacy or wholly cautious")),
        "{errors:?}"
    );
}
