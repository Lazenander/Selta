//! Extension resolution: name → (declaration, host). Builtins are just the
//! pre-registered in-process host; RPC hosts register the same way.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::host::builtin::{BuiltinHost, CmdTemplates};
use crate::host::{Determinism, ExtensionDecl, ExtensionHost};

/// Cloning is cheap (a map of `Arc`s) and is how per-pool registries are
/// built: base registry + the pool's own dialed-in hosts (docs/05).
#[derive(Clone)]
pub struct Registry {
    map: HashMap<String, (Arc<ExtensionDecl>, Arc<dyn ExtensionHost>)>,
}

impl Registry {
    pub fn with_builtins(cmds: Option<Arc<dyn CmdTemplates>>) -> Registry {
        let mut registry = Registry {
            map: HashMap::new(),
        };
        let host: Arc<dyn ExtensionHost> = Arc::new(BuiltinHost::new(cmds));
        registry
            .register(BuiltinHost::decls(), host)
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
        for decl in decls {
            self.map
                .insert(decl.name.clone(), (Arc::new(decl), host.clone()));
        }
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
