#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use selta_core::{
    Determinism, EffectClass, Envelope, ExtensionDecl, ExtensionHost, HostCall, InputDomain, Needs,
    Node, Options, Registry, Report, WireDelta,
};
use serde_json::Value;

/// A scripted host: pops pre-programmed responses; defaults to `pass` when
/// the script runs out. The "test handler" from docs/08.
#[derive(Default)]
pub struct ScriptedHost {
    responses: Mutex<VecDeque<Result<Envelope, String>>>,
    calls: AtomicU32,
}

impl ScriptedHost {
    pub fn script(items: Vec<Result<Envelope, String>>) -> Arc<Self> {
        Arc::new(ScriptedHost {
            responses: Mutex::new(items.into()),
            calls: AtomicU32::new(0),
        })
    }

    pub fn calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ExtensionHost for ScriptedHost {
    async fn verify(&self, _call: HostCall<'_>) -> Result<Envelope, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(Envelope::pass()))
    }
}

pub fn registry_with(
    name: &str,
    determinism: Determinism,
    delta_schema: Option<Node>,
    host: Arc<ScriptedHost>,
) -> Registry {
    let mut registry = Registry::with_builtins(None);
    registry
        .register(
            vec![ExtensionDecl {
                name: name.to_string(),
                semantic_revision: format!("selta.test.{name}.v1"),
                cacheable: determinism == Determinism::Deterministic,
                determinism,
                effect_class: EffectClass::Pure,
                accepted_input: InputDomain::any(),
                config_schema: None,
                config_preflight: None,
                needs: Needs::default(),
                settings_schema: None,
                delta_schema,
            }],
            host,
        )
        .expect("no name clash");
    registry
}

pub fn pass() -> Result<Envelope, String> {
    Ok(Envelope::pass())
}

pub fn fail(message: &str) -> Result<Envelope, String> {
    Ok(Envelope::fail(WireDelta::message(message)))
}

pub fn fail_with_data(message: &str, data: Value) -> Result<Envelope, String> {
    let mut delta = WireDelta::message(message);
    delta.data = Some(data);
    Ok(Envelope::fail(delta))
}

pub fn err(message: &str) -> Result<Envelope, String> {
    Err(message.to_string())
}

pub fn schema(json: Value) -> Node {
    Node::from_value(json).expect("test schema is well-formed")
}

pub async fn run(
    schema: &Node,
    value: Value,
    env: Value,
    options: &Options,
    registry: &Registry,
) -> Report {
    selta_core::verify(
        schema,
        selta_core::Input::Value(value),
        &env,
        options,
        &selta_core::Runtime::new(registry, &selta_core::NoCache),
    )
    .await
}
