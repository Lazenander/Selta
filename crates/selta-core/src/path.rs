//! Value-tree paths: `$`, `$.key`, `$.items[2]`, `$["weird key"]`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Path(Vec<Seg>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Index(usize),
}

impl Path {
    pub fn root() -> Self {
        Path(Vec::new())
    }

    pub fn child_key(&self, key: &str) -> Self {
        let mut segs = self.0.clone();
        segs.push(Seg::Key(key.to_string()));
        Path(segs)
    }

    pub fn child_index(&self, index: usize) -> Self {
        let mut segs = self.0.clone();
        segs.push(Seg::Index(index));
        Path(segs)
    }
}

fn is_plain_ident(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$")?;
        for seg in &self.0 {
            match seg {
                Seg::Key(k) if is_plain_ident(k) => write!(f, ".{k}")?,
                Seg::Key(k) => write!(f, "[{}]", serde_json::to_string(k).unwrap_or_default())?,
                Seg::Index(i) => write!(f, "[{i}]")?,
            }
        }
        Ok(())
    }
}
