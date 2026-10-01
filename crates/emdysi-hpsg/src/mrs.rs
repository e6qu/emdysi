//! Minimal Recursion Semantics (MRS): read-out from a parse, SimpleMRS
//! output and input, and isomorphism (equivalence up to variable names).
//!
//! The read-out follows the DELPH-IN conventions used by ACE and the LKB
//! for the ERG: the semantics is at `SYNSEM LOCAL CONT`, with `HOOK`
//! (`LTOP`, `INDEX`, `XARG`), `RELS`, `HCONS` and `ICONS` difference
//! lists. Each elementary predication (EP) has a `PRED`, a label `LBL`,
//! role arguments and possibly a constant `CARG`; character spans come
//! from `CFROM`/`CTO`. A fresh top handle `h0` is introduced, qeq the
//! grammar's `LTOP` (as ACE does). Variable types and properties are
//! mapped to the interface by the grammar's VPM.

use std::collections::HashMap;
use std::fmt::Write;

use crate::Grammar;
use crate::dag::Dag;
use crate::typesys::{FeatId, LiteralKind};
use crate::vpm::Vpm;

/// An elementary predication.
#[derive(Debug, Clone, PartialEq)]
pub struct Ep {
    pub pred: String,
    /// Character span in the input, if known.
    pub lnk: Option<(usize, usize)>,
    pub label: String,
    /// Role arguments (`ARG0`, `ARG1`, `RSTR`, ...), in display order.
    /// Values are variable names.
    pub args: Vec<(String, String)>,
    /// A constant argument (e.g. the name in `named`).
    pub carg: Option<String>,
}

impl Ep {
    pub fn arg(&self, role: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|(r, _)| r == role)
            .map(|(_, v)| v.as_str())
    }

    /// The intrinsic variable (`ARG0`).
    pub fn arg0(&self) -> Option<&str> {
        self.arg("ARG0")
    }

    /// A quantifier: an EP with `RSTR`.
    pub fn is_quantifier(&self) -> bool {
        self.arg("RSTR").is_some()
    }
}

/// A handle or individual constraint: `(high, relation, low)`.
pub type Constraint = (String, String, String);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mrs {
    pub top: String,
    pub index: Option<String>,
    pub eps: Vec<Ep>,
    pub hcons: Vec<Constraint>,
    pub icons: Vec<Constraint>,
    /// Properties of each variable (by name), e.g. `TENSE: past`.
    pub props: HashMap<String, Vec<(String, String)>>,
}

/// The type letter of a variable name (`e2` -> `e`).
pub fn var_type(v: &str) -> &str {
    let end = v.find(|c: char| c.is_ascii_digit()).unwrap_or(v.len());
    &v[..end]
}

/// Feature paths and settings for reading MRSs out of feature structures.
pub struct MrsConfig {
    cont: Vec<FeatId>,
    ltop: Vec<FeatId>,
    index: Vec<FeatId>,
    xarg: Vec<FeatId>,
    rels: Vec<FeatId>,
    hcons: Vec<FeatId>,
    icons: Vec<FeatId>,
    pred: FeatId,
    lbl: FeatId,
    carg: Option<FeatId>,
    cfrom: Option<FeatId>,
    cto: Option<FeatId>,
    harg: FeatId,
    larg: FeatId,
    iarg1: Option<FeatId>,
    iarg2: Option<FeatId>,
    deleted: Vec<FeatId>,
    pub vpm: Vpm,
}

/// Display order of roles; others follow in alphabetical order.
const ROLE_ORDER: &[&str] = &[
    "ARG0", "ARG1", "ARG2", "ARG3", "ARG4", "ARG", "L-INDEX", "R-INDEX", "L-HNDL", "R-HNDL",
    "RSTR", "BODY", "CARG",
];

impl MrsConfig {
    /// Paths of the ERG (and other Matrix-derived grammars). `deleted`
    /// lists roles not shown (ACE's `mrs-deleted-roles`).
    pub fn new(g: &Grammar, vpm: Vpm, deleted: &[String]) -> Option<MrsConfig> {
        let p = |s: &str| g.path(s);
        let cont = p("SYNSEM LOCAL CONT")?;
        Some(MrsConfig {
            ltop: p("HOOK LTOP")?,
            index: p("HOOK INDEX")?,
            xarg: p("HOOK XARG").unwrap_or_default(),
            rels: p("RELS LIST")?,
            hcons: p("HCONS LIST")?,
            icons: p("ICONS LIST").unwrap_or_default(),
            cont,
            pred: g.feat("PRED")?,
            lbl: g.feat("LBL")?,
            carg: g.feat("CARG"),
            cfrom: g.feat("CFROM"),
            cto: g.feat("CTO"),
            harg: g.feat("HARG")?,
            larg: g.feat("LARG")?,
            iarg1: g.feat("IARG1"),
            iarg2: g.feat("IARG2"),
            deleted: deleted.iter().filter_map(|r| g.feat(r)).collect(),
            vpm,
        })
    }
}

struct Namer<'a> {
    g: &'a Grammar,
    cfg: &'a MrsConfig,
    dag: &'a Dag,
    names: HashMap<u32, String>,
    next: usize,
    props: HashMap<String, Vec<(String, String)>>,
}

impl Namer<'_> {
    fn var(&mut self, n: u32) -> String {
        if let Some(v) = self.names.get(&n) {
            return v.clone();
        }
        let letter = self.cfg.vpm.var_type(self.g, self.dag.ty(n));
        let name = format!("{letter}{}", self.next);
        self.next += 1;
        let props = self.cfg.vpm.properties(self.g, self.dag, n, &letter);
        if !props.is_empty() {
            self.props.insert(name.clone(), props);
        }
        self.names.insert(n, name.clone());
        name
    }
}

fn list_items(g: &Grammar, dag: &Dag, mut n: u32) -> Vec<u32> {
    let mut items = Vec::new();
    while let Some(first) = dag.arc(n, g.lists.first) {
        items.push(first);
        match dag.arc(n, g.lists.rest) {
            Some(r) => n = r,
            None => break,
        }
    }
    items
}

fn string_value(g: &Grammar, dag: &Dag, n: u32) -> Option<String> {
    match g.ts.literal_value(dag.ty(n)) {
        Some((LiteralKind::Str, s)) => Some(s.to_string()),
        _ => None,
    }
}

/// Normalize a predicate name: no quotes, no `_rel` suffix, lower case.
fn pred_name(s: &str) -> String {
    let s = s.trim_matches('"');
    s.strip_suffix("_rel").unwrap_or(s).to_lowercase()
}

/// Read the MRS of a feature structure (a complete parse).
pub fn extract(g: &Grammar, cfg: &MrsConfig, dag: &Dag) -> Option<Mrs> {
    let cont = dag.follow(0, &cfg.cont)?;
    let mut nm = Namer {
        g,
        cfg,
        dag,
        names: HashMap::new(),
        next: 1,
        props: HashMap::new(),
    };
    let ltop = dag.follow(cont, &cfg.ltop).map(|n| nm.var(n));
    let index = dag.follow(cont, &cfg.index).map(|n| nm.var(n));
    if !cfg.xarg.is_empty() {
        if let Some(n) = dag.follow(cont, &cfg.xarg) {
            nm.var(n);
        }
    }
    let mut eps = Vec::new();
    for e in dag
        .follow(cont, &cfg.rels)
        .map(|n| list_items(g, dag, n))
        .unwrap_or_default()
    {
        let pred_node = dag.arc(e, cfg.pred)?;
        let pred = match g.ts.literal_value(dag.ty(pred_node)) {
            Some((_, s)) => pred_name(&s),
            None => pred_name(g.ts.hier.name(dag.ty(pred_node))),
        };
        let lnk = match (cfg.cfrom, cfg.cto) {
            (Some(f), Some(t)) => {
                let num = |feat| {
                    dag.arc(e, feat)
                        .and_then(|n| string_value(g, dag, n))
                        .and_then(|s| s.parse::<usize>().ok())
                };
                num(f).zip(num(t))
            }
            _ => None,
        };
        let label = match dag.arc(e, cfg.lbl) {
            Some(n) => nm.var(n),
            None => String::new(),
        };
        let mut carg = None;
        let mut roles: Vec<(String, u32)> = Vec::new();
        for &(f, v) in dag.arcs(e) {
            if f == cfg.pred
                || f == cfg.lbl
                || Some(f) == cfg.cfrom
                || Some(f) == cfg.cto
                || cfg.deleted.contains(&f)
            {
                continue;
            }
            if Some(f) == cfg.carg {
                carg = string_value(g, dag, v);
                continue;
            }
            roles.push((g.feats.name(f).to_string(), v));
        }
        let rank = |r: &str| {
            ROLE_ORDER
                .iter()
                .position(|x| *x == r)
                .unwrap_or(ROLE_ORDER.len())
        };
        roles.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then(a.0.cmp(&b.0)));
        let args = roles.into_iter().map(|(r, v)| (r, nm.var(v))).collect();
        eps.push(Ep {
            pred,
            lnk,
            label,
            args,
            carg,
        });
    }
    let constraints = |path: &[FeatId], hi: Option<FeatId>, lo: Option<FeatId>, nm: &mut Namer| {
        let mut out = Vec::new();
        if path.is_empty() {
            return out;
        }
        for c in dag
            .follow(cont, path)
            .map(|n| list_items(g, dag, n))
            .unwrap_or_default()
        {
            let (Some(h), Some(l)) = (
                hi.and_then(|f| dag.arc(c, f)),
                lo.and_then(|f| dag.arc(c, f)),
            ) else {
                continue;
            };
            let rel = g.ts.hier.name(dag.ty(c)).to_string();
            out.push((nm.var(h), rel, nm.var(l)));
        }
        out
    };
    let mut hcons = vec![];
    if let Some(l) = &ltop {
        hcons.push(("h0".to_string(), "qeq".to_string(), l.clone()));
    }
    hcons.extend(constraints(
        &cfg.hcons,
        Some(cfg.harg),
        Some(cfg.larg),
        &mut nm,
    ));
    let icons = constraints(&cfg.icons, cfg.iarg1, cfg.iarg2, &mut nm);
    Some(Mrs {
        top: "h0".to_string(),
        index,
        eps,
        hcons,
        icons,
        props: nm.props,
    })
}

impl Mrs {
    fn var_text(&self, v: &str, seen: &mut std::collections::HashSet<String>) -> String {
        match self.props.get(v) {
            Some(ps) if seen.insert(v.to_string()) => {
                let mut s = format!("{v} [ {}", var_type(v));
                for (k, val) in ps {
                    let _ = write!(s, " {k}: {val}");
                }
                s.push_str(" ]");
                s
            }
            _ => v.to_string(),
        }
    }

    /// SimpleMRS, the DELPH-IN text format (as written by ACE).
    pub fn to_simple(&self) -> String {
        let mut seen = std::collections::HashSet::new();
        let mut s = format!("[ LTOP: {}", self.top);
        if let Some(i) = &self.index {
            let _ = write!(s, " INDEX: {}", self.var_text(i, &mut seen));
        }
        s.push_str(" RELS: < ");
        for ep in &self.eps {
            let _ = write!(s, "[ {}", ep.pred);
            if let Some((a, b)) = ep.lnk {
                let _ = write!(s, "<{a}:{b}>");
            }
            let _ = write!(s, " LBL: {}", self.var_text(&ep.label, &mut seen));
            if let Some(c) = &ep.carg {
                let _ = write!(s, " CARG: {c:?}");
            }
            for (r, v) in &ep.args {
                let _ = write!(s, " {r}: {}", self.var_text(v, &mut seen));
            }
            s.push_str(" ] ");
        }
        s.push_str("> HCONS: < ");
        for (h, r, l) in &self.hcons {
            let _ = write!(s, "{h} {r} {l} ");
        }
        s.push_str("> ICONS: < ");
        for (h, r, l) in &self.icons {
            let _ = write!(s, "{h} {r} {l} ");
        }
        s.push_str("> ]");
        s
    }

    /// Parse SimpleMRS.
    pub fn parse_simple(src: &str) -> Option<Mrs> {
        let toks = tokenize(src);
        let mut p = SimpleParser { toks, i: 0 };
        p.mrs()
    }

    /// Variables (with their properties) of `EP`s and constraints.
    pub fn variables(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut push = |v: &String| {
            if !out.contains(v) {
                out.push(v.clone());
            }
        };
        push(&self.top);
        if let Some(i) = &self.index {
            push(i);
        }
        for ep in &self.eps {
            push(&ep.label);
            for (_, v) in &ep.args {
                push(v);
            }
        }
        for (h, _, l) in self.hcons.iter().chain(&self.icons) {
            push(h);
            push(l);
        }
        out
    }

    /// Structural checks after pyDelphin's `is_well_formed`: every EP has
    /// an intrinsic variable (`ARG0`) distinct from other EPs' (quantifiers
    /// excepted), the EPs are connected through shared variables, and every
    /// qeq links a handle argument to a label.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut intrinsic: HashMap<&str, usize> = HashMap::new();
        for (i, ep) in self.eps.iter().enumerate() {
            match ep.arg0() {
                None => out.push(format!("EP {} has no ARG0", ep.pred)),
                Some(v) if !ep.is_quantifier() => {
                    if let Some(j) = intrinsic.insert(v, i) {
                        out.push(format!(
                            "{} and {} share the intrinsic variable {v}",
                            self.eps[j].pred, ep.pred
                        ));
                    }
                }
                _ => {}
            }
        }
        // Connectedness over EPs, linking EPs that share any variable or
        // that are related by a qeq.
        let n = self.eps.len();
        if n > 1 {
            let mut parent: Vec<usize> = (0..n).collect();
            fn find(p: &mut [usize], x: usize) -> usize {
                let mut r = x;
                while p[r] != r {
                    r = p[r];
                }
                p[x] = r;
                r
            }
            let mut by_var: HashMap<String, usize> = HashMap::new();
            let qeq: HashMap<&str, &str> = self
                .hcons
                .iter()
                .map(|(h, _, l)| (h.as_str(), l.as_str()))
                .collect();
            for (i, ep) in self.eps.iter().enumerate() {
                let mut vars: Vec<String> = vec![ep.label.clone()];
                for (_, v) in &ep.args {
                    vars.push(v.clone());
                    if let Some(l) = qeq.get(v.as_str()) {
                        vars.push(l.to_string());
                    }
                }
                for v in vars {
                    match by_var.get(&v) {
                        Some(&j) => {
                            let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                            parent[a] = b;
                        }
                        None => {
                            by_var.insert(v, i);
                        }
                    }
                }
            }
            let root = find(&mut parent, 0);
            if (1..n).any(|i| find(&mut parent, i) != root) {
                out.push("the EPs are not connected".to_string());
            }
        }
        let labels: std::collections::HashSet<&str> =
            self.eps.iter().map(|e| e.label.as_str()).collect();
        for (h, r, l) in &self.hcons {
            if r == "qeq" && !labels.contains(l.as_str()) {
                out.push(format!("{h} qeq {l}: {l} labels no EP"));
            }
        }
        out
    }
}

fn tokenize(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if "[]<>:".contains(c) {
            out.push(c.to_string());
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::from("\"");
            while let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        s.push(n);
                    }
                } else if c == '"' {
                    break;
                } else {
                    s.push(c);
                }
            }
            out.push(s);
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || "[]<>:\"".contains(c) {
                    break;
                }
                s.push(c);
                chars.next();
            }
            out.push(s);
        }
    }
    out
}

struct SimpleParser {
    toks: Vec<String>,
    i: usize,
}

impl SimpleParser {
    fn peek(&self) -> Option<&str> {
        self.toks.get(self.i).map(String::as_str)
    }
    fn peek_at(&self, k: usize) -> Option<&str> {
        self.toks.get(self.i + k).map(String::as_str)
    }
    fn next(&mut self) -> Option<String> {
        let t = self.toks.get(self.i).cloned();
        self.i += 1;
        t
    }
    fn expect(&mut self, t: &str) -> Option<()> {
        (self.next()? == t).then_some(())
    }

    /// A variable, possibly followed by `[ type KEY: value ... ]`.
    fn var(&mut self, props: &mut HashMap<String, Vec<(String, String)>>) -> Option<String> {
        let v = self.next()?;
        if self.peek() == Some("[") {
            self.next();
            self.next()?; // type letter
            let mut ps = Vec::new();
            while self.peek()? != "]" {
                let k = self.next()?;
                self.expect(":")?;
                let val = self.next()?;
                ps.push((k.to_uppercase(), val.to_lowercase()));
            }
            self.next();
            if !ps.is_empty() {
                props.insert(v.clone(), ps);
            }
        }
        Some(v)
    }

    fn lnk(&mut self) -> Option<(usize, usize)> {
        if self.peek() == Some("<") && self.peek_at(2) == Some(":") && self.peek_at(4) == Some(">")
        {
            let a = self.toks[self.i + 1].parse().ok();
            let b = self.toks[self.i + 3].parse().ok();
            self.i += 5;
            return a.zip(b);
        }
        None
    }

    fn mrs(&mut self) -> Option<Mrs> {
        let mut m = Mrs::default();
        self.expect("[")?;
        self.lnk();
        while self.peek()? != "]" {
            let key = self.next()?.to_uppercase();
            self.expect(":")?;
            match key.as_str() {
                "LTOP" | "TOP" => m.top = self.var(&mut m.props)?,
                "INDEX" => m.index = Some(self.var(&mut m.props)?),
                "RELS" => {
                    self.expect("<")?;
                    while self.peek()? != ">" {
                        let mut props = std::mem::take(&mut m.props);
                        let ep = self.ep(&mut props)?;
                        m.props = props;
                        m.eps.push(ep);
                    }
                    self.next();
                }
                "HCONS" | "ICONS" => {
                    self.expect("<")?;
                    while self.peek()? != ">" {
                        let h = self.var(&mut m.props)?;
                        let r = self.next()?;
                        let l = self.var(&mut m.props)?;
                        if key == "HCONS" {
                            m.hcons.push((h, r, l));
                        } else {
                            m.icons.push((h, r, l));
                        }
                    }
                    self.next();
                }
                _ => {
                    self.var(&mut m.props)?;
                }
            }
        }
        Some(m)
    }

    fn ep(&mut self, props: &mut HashMap<String, Vec<(String, String)>>) -> Option<Ep> {
        self.expect("[")?;
        let pred = pred_name(&self.next()?);
        let lnk = self.lnk();
        let mut ep = Ep {
            pred,
            lnk,
            label: String::new(),
            args: Vec::new(),
            carg: None,
        };
        while self.peek()? != "]" {
            let role = self.next()?.to_uppercase();
            self.expect(":")?;
            if role == "CARG" {
                ep.carg = Some(self.next()?.trim_start_matches('"').to_string());
            } else if role == "LBL" {
                ep.label = self.var(props)?;
            } else {
                let v = self.var(props)?;
                ep.args.push((role, v));
            }
        }
        self.next();
        Some(ep)
    }
}

/// Whether two MRSs are equal up to a renaming of variables (predicates,
/// constants, roles, variable types and properties, and constraints must
/// match; character spans are ignored).
pub fn isomorphic(a: &Mrs, b: &Mrs) -> bool {
    if a.eps.len() != b.eps.len()
        || a.hcons.len() != b.hcons.len()
        || a.icons.len() != b.icons.len()
    {
        return false;
    }
    let mut st = Iso {
        a,
        b,
        fwd: HashMap::new(),
        bwd: HashMap::new(),
        used: vec![false; b.eps.len()],
        budget: 200_000,
    };
    if !st.bind(&a.top, &b.top) {
        return false;
    }
    match (&a.index, &b.index) {
        (Some(x), Some(y)) => {
            if !st.bind(x, y) {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }
    st.search(0)
}

struct Iso<'a> {
    a: &'a Mrs,
    b: &'a Mrs,
    fwd: HashMap<String, String>,
    bwd: HashMap<String, String>,
    used: Vec<bool>,
    budget: usize,
}

impl Iso<'_> {
    fn props_eq(&self, x: &str, y: &str) -> bool {
        let norm = |m: &Mrs, v: &str| {
            let mut p: Vec<(String, String)> = m
                .props
                .get(v)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k.to_uppercase(), v.to_lowercase()))
                .collect();
            p.sort();
            p
        };
        norm(self.a, x) == norm(self.b, y)
    }

    /// Extend the bijection with `x -> y`; false if inconsistent.
    fn bind(&mut self, x: &str, y: &str) -> bool {
        match (self.fwd.get(x), self.bwd.get(y)) {
            (Some(fy), Some(bx)) => fy == y && bx == x,
            (None, None) => {
                if var_type(x) != var_type(y) || !self.props_eq(x, y) {
                    return false;
                }
                self.fwd.insert(x.to_string(), y.to_string());
                self.bwd.insert(y.to_string(), x.to_string());
                true
            }
            _ => false,
        }
    }

    fn constraints_match(&mut self, xs: &[Constraint], ys: &[Constraint]) -> bool {
        let mut left: Vec<&Constraint> = ys.iter().collect();
        for (h, r, l) in xs {
            let (Some(mh), Some(ml)) = (self.fwd.get(h).cloned(), self.fwd.get(l).cloned()) else {
                // A constraint over variables not in any EP: bind it.
                let Some(pos) = left.iter().position(|(h2, r2, l2)| {
                    r2 == r
                        && self.fwd.get(h).is_none_or(|m| m == h2)
                        && self.fwd.get(l).is_none_or(|m| m == l2)
                }) else {
                    return false;
                };
                let (h2, _, l2) = left.remove(pos);
                if !self.bind(h, h2) || !self.bind(l, l2) {
                    return false;
                }
                continue;
            };
            let Some(pos) = left
                .iter()
                .position(|(h2, r2, l2)| *h2 == mh && r2 == r && *l2 == ml)
            else {
                return false;
            };
            left.remove(pos);
        }
        true
    }

    fn search(&mut self, k: usize) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        if k == self.a.eps.len() {
            let (fwd, bwd) = (self.fwd.clone(), self.bwd.clone());
            let ok = self.constraints_match(&self.a.hcons.clone(), &self.b.hcons.clone())
                && self.constraints_match(&self.a.icons.clone(), &self.b.icons.clone());
            if !ok {
                self.fwd = fwd;
                self.bwd = bwd;
            }
            return ok;
        }
        let ea = &self.a.eps[k];
        for j in 0..self.b.eps.len() {
            if self.used[j] {
                continue;
            }
            let eb = &self.b.eps[j];
            if ea.pred != eb.pred || ea.carg != eb.carg || ea.args.len() != eb.args.len() {
                continue;
            }
            let mut roles_a: Vec<&String> = ea.args.iter().map(|(r, _)| r).collect();
            let mut roles_b: Vec<&String> = eb.args.iter().map(|(r, _)| r).collect();
            roles_a.sort();
            roles_b.sort();
            if roles_a != roles_b {
                continue;
            }
            let (fwd, bwd) = (self.fwd.clone(), self.bwd.clone());
            let mut ok = self.bind(&ea.label, &eb.label);
            for (r, v) in &ea.args {
                if !ok {
                    break;
                }
                let w = eb.arg(r).unwrap_or_default();
                ok = self.bind(v, w);
            }
            if ok {
                self.used[j] = true;
                if self.search(k + 1) {
                    return true;
                }
                self.used[j] = false;
            }
            self.fwd = fwd;
            self.bwd = bwd;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABRAMS: &str = r#"[ LTOP: h0 INDEX: e2 [ e SF: prop TENSE: past MOOD: indicative PROG: - PERF: - ] RELS: < [ proper_q<0:6> LBL: h4 ARG0: x3 [ x PERS: 3 NUM: sg IND: + ] RSTR: h5 BODY: h6 ]  [ named<0:6> LBL: h7 CARG: "Abrams" ARG0: x3 ]  [ _bark_v_1<7:13> LBL: h1 ARG0: e2 ARG1: x3 ] > HCONS: < h0 qeq h1 h5 qeq h7 > ICONS: < > ]"#;

    #[test]
    fn simple_mrs_round_trip() {
        let m = Mrs::parse_simple(ABRAMS).unwrap();
        assert_eq!(m.eps.len(), 3);
        assert_eq!(m.eps[1].carg.as_deref(), Some("Abrams"));
        assert_eq!(m.eps[2].lnk, Some((7, 13)));
        let again = Mrs::parse_simple(&m.to_simple()).unwrap();
        assert!(isomorphic(&m, &again));
        assert!(m.problems().is_empty(), "{:?}", m.problems());
    }

    #[test]
    fn isomorphism_ignores_names_but_not_structure() {
        let a = Mrs::parse_simple(ABRAMS).unwrap();
        let renamed = ABRAMS
            .replace("x3", "x9")
            .replace("h7", "h70")
            .replace("e2", "e5");
        assert!(isomorphic(&a, &Mrs::parse_simple(&renamed).unwrap()));
        let other_pred = ABRAMS.replace("_bark_v_1", "_bite_v_1");
        assert!(!isomorphic(&a, &Mrs::parse_simple(&other_pred).unwrap()));
        let other_tense = ABRAMS.replacen("TENSE: past", "TENSE: pres", 1);
        assert!(!isomorphic(&a, &Mrs::parse_simple(&other_tense).unwrap()));
        let other_qeq = ABRAMS.replace("h5 qeq h7", "h6 qeq h7");
        assert!(!isomorphic(&a, &Mrs::parse_simple(&other_qeq).unwrap()));
    }

    #[test]
    fn problems_found() {
        let disconnected = ABRAMS.replace("ARG1: x3 ]", "ARG1: x8 ]");
        let m = Mrs::parse_simple(&disconnected).unwrap();
        assert!(m.problems().iter().any(|p| p.contains("connected")));
    }
}
