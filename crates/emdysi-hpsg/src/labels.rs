//! Phrase-structure labels for derivation nodes, from a grammar's label
//! templates (the ERG's `parse-nodes.tdl`), following the LKB.
//!
//! Label templates are tried in definition order; a node gets the `LNAME`
//! of the first template whose features (other than the label features)
//! unify with the node. Meta templates such as `slash` add a suffix: when a
//! node unifies with the meta template, the value at the meta path (e.g. the
//! first gapped `LOCAL` on `SLASH`) is labelled in turn, giving `S/NP`.

use std::sync::Arc;

use crate::chartmap::string_at;
use crate::dag::Dag;
use crate::grammar::Grammar;
use crate::parser::Deriv;
use crate::typesys::FeatId;
use crate::unify::Unifier;

struct Template {
    dag: Arc<Dag>,
    name: String,
}

struct Meta {
    dag: Arc<Dag>,
    prefix: String,
    suffix: String,
}

pub struct Labeler {
    labels: Vec<Template>,
    metas: Vec<Meta>,
    /// Root features of templates that carry the label itself.
    label_feats: Vec<FeatId>,
    /// Path in a meta template to the structure to label (e.g. the slashed
    /// LOCAL), and the path labels are compared on for it (SYNSEM LOCAL).
    meta_path: Vec<FeatId>,
    local_path: Vec<FeatId>,
}

/// A labelled tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tree {
    Node {
        label: String,
        start: usize,
        end: usize,
        kids: Vec<Tree>,
    },
    Leaf {
        form: String,
        start: usize,
        end: usize,
    },
}

impl Labeler {
    /// Build from the instances whose type is a subtype of `label` or `meta`.
    pub fn new(g: &Grammar, u: &mut Unifier) -> Option<Labeler> {
        let label_t = g.ts.hier.id("label")?;
        let meta_t = g.ts.hier.id("meta")?;
        let lname = g.path("LNAME")?;
        let prefix = g.path("META-PREFIX")?;
        let suffix = g.path("META-SUFFIX")?;
        let mut labels = Vec::new();
        let mut metas = Vec::new();
        for inst in g.instances.iter().filter(|i| i.status.is_none()) {
            let Ok(dag) = g.expand(&inst.body, u) else {
                continue;
            };
            let t = dag.root_type();
            if g.ts.subsumed_by(t, meta_t) {
                metas.push(Meta {
                    prefix: string_at(&g.ts, &dag, &prefix).unwrap_or_default(),
                    suffix: string_at(&g.ts, &dag, &suffix).unwrap_or_default(),
                    dag: Arc::new(dag),
                });
            } else if g.ts.subsumed_by(t, label_t) {
                if let Some(name) = string_at(&g.ts, &dag, &lname) {
                    labels.push(Template {
                        name,
                        dag: Arc::new(dag),
                    });
                }
            }
        }
        Some(Labeler {
            labels,
            metas,
            label_feats: [lname[0], prefix[0], suffix[0]].to_vec(),
            meta_path: g.path("SYNSEM NONLOC SLASH LIST FIRST")?,
            local_path: g.path("SYNSEM LOCAL")?,
        })
    }

    /// Unify the non-label root features of `template` (at `tnode`) with
    /// the corresponding parts of `node` (at `nnode`).
    fn matches(
        &self,
        g: &Grammar,
        u: &mut Unifier,
        template: &Arc<Dag>,
        tnode: u32,
        node: &Arc<Dag>,
        nnode: u32,
    ) -> bool {
        let cons = g.constraint_fn();
        u.begin();
        let t = u.add(template.clone());
        let n = u.add(node.clone());
        for &(f, v) in template.arcs(tnode) {
            if tnode == 0 && self.label_feats.contains(&f) {
                continue;
            }
            let Some(nv) = node.arc(nnode, f) else {
                continue;
            };
            if !u.unify(t + v, n + nv, &g.ts, &cons) {
                return false;
            }
        }
        true
    }

    fn base_label(&self, g: &Grammar, u: &mut Unifier, dag: &Arc<Dag>) -> Option<String> {
        self.labels
            .iter()
            .find(|t| self.matches(g, u, &t.dag, 0, dag, 0))
            .map(|t| t.name.clone())
    }

    /// Label for a gapped LOCAL value: the first template whose
    /// `SYNSEM LOCAL` unifies with it.
    fn local_label(
        &self,
        g: &Grammar,
        u: &mut Unifier,
        dag: &Arc<Dag>,
        local: u32,
    ) -> Option<String> {
        let cons = g.constraint_fn();
        self.labels
            .iter()
            .find(|t| {
                let Some(tl) = t.dag.follow(0, &self.local_path) else {
                    return false;
                };
                u.begin();
                let a = u.add(t.dag.clone());
                let b = u.add(dag.clone());
                u.unify(a + tl, b + local, &g.ts, &cons)
            })
            .map(|t| t.name.clone())
    }

    /// The label of a node's feature structure, e.g. `NP` or `S/NP`.
    pub fn label(&self, g: &Grammar, u: &mut Unifier, dag: &Arc<Dag>) -> String {
        let mut label = self.base_label(g, u, dag).unwrap_or_else(|| "?".into());
        for m in &self.metas {
            if !self.matches(g, u, &m.dag, 0, dag, 0) {
                continue;
            }
            if let Some(local) = dag.follow(0, &self.meta_path) {
                if let Some(l) = self.local_label(g, u, dag, local) {
                    label = format!("{label}{}{l}{}", m.prefix, m.suffix);
                }
            }
        }
        label
    }

    /// Label every node of a derivation. `form` gives the surface string of
    /// a lexical node's tokens.
    pub fn tree(
        &self,
        g: &Grammar,
        u: &mut Unifier,
        d: &Deriv,
        form: &dyn Fn(&Deriv) -> Option<String>,
    ) -> Tree {
        let label = self.label(g, u, &d.dag);
        let kids = match form(d) {
            Some(f) if d.daughters.is_empty() => vec![Tree::Leaf {
                form: f,
                start: d.start,
                end: d.end,
            }],
            _ => d
                .daughters
                .iter()
                .map(|k| self.tree(g, u, k, form))
                .collect(),
        };
        Tree::Node {
            label,
            start: d.start,
            end: d.end,
            kids,
        }
    }
}

impl Tree {
    /// Collapse unary chains whose nodes share a label, e.g. `NP -> NP`.
    pub fn collapse(self) -> Tree {
        match self {
            Tree::Node {
                label,
                start,
                end,
                kids,
            } => {
                let kids: Vec<Tree> = kids.into_iter().map(Tree::collapse).collect();
                if kids.len() == 1 {
                    if let Tree::Node { label: kl, .. } = &kids[0] {
                        if *kl == label {
                            return kids.into_iter().next().unwrap();
                        }
                    }
                }
                Tree::Node {
                    label,
                    start,
                    end,
                    kids,
                }
            }
            leaf => leaf,
        }
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            Tree::Node { label, .. } => Some(label),
            Tree::Leaf { .. } => None,
        }
    }

    /// Bracketed form: `(S (NP (N Abrams)) (VP (V barked)))`.
    pub fn bracketed(&self) -> String {
        match self {
            Tree::Leaf { form, .. } => form.clone(),
            Tree::Node { label, kids, .. } => {
                let inner: Vec<String> = kids.iter().map(Tree::bracketed).collect();
                format!("({label} {})", inner.join(" "))
            }
        }
    }
}
