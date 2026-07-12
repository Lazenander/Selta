# Mechanics conformance controls

The three case/oracle pairs use the closed S2 conformance-control DSL. The
forbidden-read case injects a resolver probe at operational-rule call ordinal
4 and must return the exact trap before exposing the response value. The two
semantic-output controls inject schema-valid outputs whose payload is the
otherwise opaque response reference; both dispatches commit and return
`relation.semantic_mismatch` with the mutant's actual fuel.

The expected error packages are ordinary candidate packages and contain no
control field. Oracles compare their conformance projections because truthful
implementation artifact identities may differ between independent models.
