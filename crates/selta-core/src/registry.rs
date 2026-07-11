//! Extension resolution: name → (declaration, host). Builtins are just the
//! pre-registered in-process host; RPC hosts register the same way.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::host::builtin::{BuiltinHost, CmdTemplates};
use crate::host::{Determinism, EffectClass, ExtensionDecl, ExtensionHost};
use crate::registry_admission::{
    first_verifier_pointer, validate_node_with_policy, AdmissionPolicy,
};
use crate::strict_admission::{AdmittedNode, MetaIssue};

/// Cloning is cheap (a map of `Arc`s) and is how per-pool registries are
/// built: base registry + the pool's own dialed-in hosts (docs/05).
#[derive(Clone)]
pub struct Registry {
    map: HashMap<String, (Arc<ExtensionDecl>, Arc<dyn ExtensionHost>)>,
}

impl Registry {
    pub fn with_builtins(cmds: Option<Arc<dyn CmdTemplates>>) -> Registry {
        let declarations = if cmds.is_some() {
            BuiltinHost::decls()
        } else {
            BuiltinHost::pure_decls()
        };
        Self::with_builtin_declarations(declarations, cmds)
    }

    /// Foundation-safe in-process profile: pure builtins only, never `cmd`.
    pub fn with_pure_builtins() -> Registry {
        Self::with_builtin_declarations(BuiltinHost::pure_decls(), None)
    }

    fn with_builtin_declarations(
        declarations: Vec<ExtensionDecl>,
        cmds: Option<Arc<dyn CmdTemplates>>,
    ) -> Registry {
        let mut registry = Registry {
            map: HashMap::new(),
        };
        let host: Arc<dyn ExtensionHost> = Arc::new(BuiltinHost::new(cmds));
        registry
            .register(declarations, host)
            .expect("builtin names cannot clash in an empty registry");
        registry
    }

    pub fn register(
        &mut self,
        decls: Vec<ExtensionDecl>,
        host: Arc<dyn ExtensionHost>,
    ) -> Result<(), String> {
        let mut incoming = HashSet::with_capacity(decls.len());
        for decl in &decls {
            if decl.name.trim().is_empty() {
                return Err("extension name must not be empty".to_string());
            }
            if decl.semantic_revision.trim().is_empty() {
                return Err(format!(
                    "extension '{}': semantic_revision must not be empty",
                    decl.name
                ));
            }
            if decl.cacheable && decl.determinism != Determinism::Deterministic {
                return Err(format!(
                    "extension '{}': only deterministic extensions may be cacheable",
                    decl.name
                ));
            }
            if decl.cacheable && decl.effect_class != EffectClass::Pure {
                return Err(format!(
                    "extension '{}': cacheable extensions must declare a pure effect class",
                    decl.name
                ));
            }
            if self.map.contains_key(&decl.name) {
                return Err(format!("extension name clash: '{}'", decl.name));
            }
            if !incoming.insert(decl.name.clone()) {
                return Err(format!(
                    "duplicate extension name in registration batch: '{}'",
                    decl.name
                ));
            }
        }

        // Normalize every declaration schema before constructing a
        // prospective registry. Nothing below mutates `self` until every
        // cross-declaration delta reference has also passed.
        let mut normalized = Vec::with_capacity(decls.len());
        for mut decl in decls {
            decl.config_schema = normalize_declaration_schema(
                &decl.name,
                "config_schema",
                decl.config_schema,
                true,
            )?;
            decl.settings_schema = normalize_declaration_schema(
                &decl.name,
                "settings_schema",
                decl.settings_schema,
                true,
            )?;
            decl.delta_schema =
                normalize_declaration_schema(&decl.name, "delta_schema", decl.delta_schema, false)?;
            normalized.push(decl);
        }

        let mut prospective = self.clone();
        for decl in &normalized {
            prospective
                .map
                .insert(decl.name.clone(), (Arc::new(decl.clone()), host.clone()));
        }

        let policy = AdmissionPolicy::allow_all();
        for decl in &normalized {
            let Some(delta_schema) = &decl.delta_schema else {
                continue;
            };
            let issues = validate_node_with_policy(&prospective, delta_schema, &policy);
            if !issues.is_empty() {
                return Err(format!(
                    "extension '{}': delta_schema failed registry admission: {}",
                    decl.name,
                    format_meta_issues(&issues)
                ));
            }
        }

        *self = prospective;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<(Arc<ExtensionDecl>, Arc<dyn ExtensionHost>)> {
        self.map
            .get(name)
            .map(|(decl, host)| (decl.clone(), host.clone()))
    }

    pub fn decl(&self, name: &str) -> Option<Arc<ExtensionDecl>> {
        self.map.get(name).map(|(decl, _)| decl.clone())
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.map.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn decls(&self) -> Vec<Arc<ExtensionDecl>> {
        let mut decls: Vec<Arc<ExtensionDecl>> =
            self.map.values().map(|(decl, _)| decl.clone()).collect();
        decls.sort_by(|a, b| a.name.cmp(&b.name));
        decls
    }
}

fn normalize_declaration_schema(
    extension: &str,
    role: &str,
    schema: Option<crate::schema::Node>,
    reject_verifiers: bool,
) -> Result<Option<crate::schema::Node>, String> {
    let Some(schema) = schema else {
        return Ok(None);
    };
    if reject_verifiers {
        if let Some(pointer) = first_verifier_pointer(&schema, "") {
            return Err(format!(
                "extension '{extension}': {role} contains verifier annotations at {pointer}; declaration config/settings schemas are structural only"
            ));
        }
    }
    let source = serde_json::to_vec(&schema).map_err(|error| {
        format!("extension '{extension}': {role} cannot be serialized: {error}")
    })?;
    AdmittedNode::admit_source(&source)
        .map(AdmittedNode::into_node)
        .map(Some)
        .map_err(|issues| {
            format!(
                "extension '{extension}': invalid {role}: {}",
                format_meta_issues(&issues)
            )
        })
}

fn format_meta_issues(issues: &[MetaIssue]) -> String {
    issues
        .iter()
        .map(|issue| {
            let pointer = if issue.pointer.is_empty() {
                "/"
            } else {
                &issue.pointer
            };
            format!("{} at {pointer}: {}", issue.code, issue.detail)
        })
        .collect::<Vec<_>>()
        .join("; ")
}
