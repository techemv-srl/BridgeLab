//! FHIRPath evaluator.
//!
//! Everything in FHIRPath is a collection, so every expression evaluates to
//! a `Vec<Value>`. The empty collection doubles as "no value" and propagates
//! through most operators; operators that need a single value use
//! [`singleton`], which errors on a collection of more than one.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use super::ast::*;
use super::types;

/// The longest string one operation may build (`replace()`, `join()`, `&`
/// and the like). A short expression can otherwise ask for terabytes:
/// twelve chained `.replace('a', 'aaaaaaaaaa')` make 10^12 bytes, and a
/// failed allocation aborts the whole app. Far above any value a resource
/// carries in practice.
pub const MAX_STRING_BYTES: usize = 16 * 1024 * 1024;

/// What one evaluation may build in all, as counted by [`approx_size`]:
/// strings made by operations, and the values a lambda (`select()`,
/// `repeat()`...), `combine()`, `union()`, `descendants()`, `resolve()` or
/// `aggregate()` returns. Growth that stays under the string cap still
/// multiplies: `(1|2|…|10).select(…)` chained ten times is 10^10 values.
/// `descendants()` of a 10 MB Bundle of Observations counts about 115 MB.
pub const MAX_EVAL_BYTES: usize = 512 * 1024 * 1024;

/// Roughly what `v` takes in memory: each value, key and string's text.
/// Close enough to tell a runaway expression from a real one, and cheap
/// next to the copy it is measured for.
pub fn approx_size(v: &Value) -> usize {
    const VALUE: usize = std::mem::size_of::<Value>();
    let mut total = 0usize;
    let mut stack = vec![v];
    while let Some(v) = stack.pop() {
        total = total.saturating_add(VALUE);
        match v {
            Value::String(s) => total = total.saturating_add(s.len()),
            Value::Array(items) => stack.extend(items),
            Value::Object(map) => {
                for (k, item) in map {
                    total = total.saturating_add(k.len() + VALUE);
                    stack.push(item);
                }
            }
            _ => {}
        }
    }
    total
}

/// [`approx_size`] of a whole collection.
pub fn approx_size_of(values: &[Value]) -> usize {
    values.iter().fold(0usize, |n, v| n.saturating_add(approx_size(v)))
}

/// The most work one evaluation may do, in steps. A step is one part of
/// the expression evaluated, one value a path copies out of the resource,
/// one item a `where()`, `select()`, `all()` or `exists()` runs its
/// criteria on, one comparison made by `distinct()`, `|`, `in` and the
/// like, or a quarter KB of a resource copied whole. The memory budget
/// cannot see work that builds nothing: nested `where(… .exists())`
/// multiply by the length of the list at each level, so ten names nested
/// nine deep are 10^9 criteria and hours of work, in a few MB. A real rule
/// takes a few hundred steps, `isDistinct()` over 5000 values 12 million,
/// and a `where()` comparing each entry of a 1000-entry Bundle with every
/// other 11 million. The limit is a few seconds of work, up to about
/// twenty for the slowest kind of step.
pub const MAX_EVAL_STEPS: u64 = 50_000_000;

/// The longest one evaluation may run, whatever its steps. A step is
/// usually a fraction of a microsecond, but copying a resource of many MB
/// or comparing two of them is one step too, and takes milliseconds; this
/// stops those. Well past what the step budget takes on its own, so it
/// decides only for those.
pub const MAX_EVAL_TIME: Duration = Duration::from_secs(60);

/// The FHIR rules of one validation share this many evaluations' worth of
/// steps and time. A Bundle is checked entry by entry, and a slow rule
/// must not take its full budget again for each of thousands of entries.
pub const VALIDATION_EVALS: u32 = 2;

/// The clock is read once every this many charges: reading it costs more
/// than many steps do.
const CLOCK_EVERY: u32 = 64;

/// Work evaluations may still do: steps left, and a time at which they
/// stop. Each evaluation has its own ([`Budget::new`]); the FHIR rules of
/// one validation also share one ([`Budget::for_validation`]).
#[derive(Debug, Clone)]
pub struct Budget {
    steps: u64,
    deadline: Instant,
    /// The limits it started with, which the error names.
    max_steps: u64,
    max_time: Duration,
    /// What each evaluation drawing on it may do on its own.
    each_steps: u64,
    each_time: Duration,
    /// Charges since the clock was last read.
    unclocked: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Self::new()
    }
}

impl Budget {
    /// What one evaluation may do: [`MAX_EVAL_STEPS`] and [`MAX_EVAL_TIME`].
    pub fn new() -> Self {
        Self::with_limits(MAX_EVAL_STEPS, MAX_EVAL_TIME)
    }

    /// What the FHIR rules of one validation may do in all, each
    /// evaluation still within its own limits.
    pub fn for_validation() -> Self {
        Self::with_limits(
            MAX_EVAL_STEPS.saturating_mul(VALIDATION_EVALS.into()),
            MAX_EVAL_TIME.saturating_mul(VALIDATION_EVALS),
        )
        .each_at_most(MAX_EVAL_STEPS, MAX_EVAL_TIME)
    }

    /// `steps` steps, for at most `time` from now.
    pub fn with_limits(steps: u64, time: Duration) -> Self {
        let now = Instant::now();
        Budget {
            steps,
            deadline: now.checked_add(time).unwrap_or(now + Duration::from_secs(86_400)),
            max_steps: steps,
            max_time: time,
            each_steps: steps,
            each_time: time,
            unclocked: 0,
        }
    }

    /// Each evaluation drawing on this budget does at most `steps` steps,
    /// for at most `time`, however much is left of it.
    pub fn each_at_most(mut self, steps: u64, time: Duration) -> Self {
        self.each_steps = steps;
        self.each_time = time;
        self
    }

    /// Steps still allowed.
    pub fn steps_left(&self) -> u64 {
        self.steps
    }

    /// The limits it started with, in words: `100 million steps or 120
    /// seconds`.
    pub fn limits(&self) -> String {
        format!("{} steps or {} seconds", in_words(self.max_steps), self.max_time.as_secs())
    }

    /// True once nothing more can be evaluated under it.
    pub fn exhausted(&self) -> bool {
        self.steps == 0 || Instant::now() >= self.deadline
    }

    /// The budget of one evaluation drawing on this shared one: its own
    /// limits, or what is left here when that is less.
    pub fn for_one_evaluation(&self) -> Budget {
        let own = Budget::with_limits(self.each_steps, self.each_time);
        Budget {
            steps: own.steps.min(self.steps),
            deadline: own.deadline.min(self.deadline),
            ..own
        }
    }

    /// Take away what an evaluation drawing on this budget used.
    pub fn used(&mut self, steps: u64) {
        self.steps = self.steps.saturating_sub(steps);
    }

    /// Count `steps` more: an error once past the limit, and from then on.
    pub fn charge(&mut self, steps: u64) -> Result<(), String> {
        if steps > self.steps {
            self.steps = 0;
            return Err(format!(
                "The expression takes too long to evaluate (more than {} steps); narrow it down",
                in_words(self.max_steps)
            ));
        }
        self.steps -= steps;
        self.unclocked += 1;
        if self.unclocked >= CLOCK_EVERY {
            self.unclocked = 0;
            if Instant::now() >= self.deadline {
                self.steps = 0;
                return Err(format!(
                    "The expression takes too long to evaluate (more than {} seconds); narrow it down",
                    self.max_time.as_secs()
                ));
            }
        }
        Ok(())
    }
}

/// `50 million` for 50 000 000, the plain number otherwise.
fn in_words(n: u64) -> String {
    if n >= 1_000_000 && n % 1_000_000 == 0 {
        format!("{} million", n / 1_000_000)
    } else {
        n.to_string()
    }
}

/// Evaluation environment: the resource the expression started from plus any
/// lambda bindings currently in scope.
pub struct Env<'a> {
    /// `%resource` / `%context` — the resource the evaluation started from.
    pub root: &'a Value,
    /// `$this`, when inside a lambda argument.
    pub this: Option<Vec<Value>>,
    /// `$index`, when inside a lambda argument.
    pub index: Option<i64>,
    /// `$total`, inside `aggregate()`.
    pub total: Option<Vec<Value>>,
    /// Lines emitted by `trace()`, in order.
    pub trace: Vec<(String, Vec<Value>)>,
    /// What `resolve()` searches: the root, or the Bundle a rule's entry
    /// resource sits in.
    scope: &'a Value,
    /// Built on the first `resolve()`, then reused for every reference,
    /// and by later evaluations in the same scope ([`References`]).
    references: Option<ResolveIndex<'a>>,
    /// What the evaluation has built so far, and may build in all
    /// ([`MAX_EVAL_BYTES`]).
    spent: usize,
    budget: usize,
    /// The work it may still do ([`MAX_EVAL_STEPS`], [`MAX_EVAL_TIME`]).
    pub work: Budget,
    /// The steps one copy of the root costs, measured on the first copy.
    root_steps: Option<usize>,
    /// The holder (resource and Bundle entry) of `$this`, when the lambda
    /// iterating over it knows: a `resolve()` inside a criteria or a
    /// projection then resolves where the item sits.
    this_holder: Option<Holder>,
    /// The holders of the items a lambda is about to iterate over, and the
    /// address of that collection: only the lambda given exactly that
    /// collection takes them (see [`Env::holder_for`]).
    lambda_holders: Option<(usize, Rc<[Option<Holder>]>)>,
}

/// A copy of this many bytes (as [`approx_size`] counts them) is one step,
/// which takes about as long as an ordinary step.
const BYTES_PER_STEP: usize = 256;

/// Building the `resolve()` index visits this many values per step.
const INDEXED_PER_STEP: usize = 16;

impl<'a> Env<'a> {
    /// Evaluate against `root`, resolving references in `scope` (the Bundle
    /// `root` is an entry of).
    pub fn within(root: &'a Value, scope: &'a Value) -> Self {
        Env {
            root,
            this: None,
            index: None,
            total: None,
            trace: Vec::new(),
            scope,
            references: None,
            spent: 0,
            budget: MAX_EVAL_BYTES,
            work: Budget::new(),
            root_steps: None,
            this_holder: None,
            lambda_holders: None,
        }
    }

    /// The holder of item `i` of `focus`, when `focus` is the collection a
    /// lambda was handed with its holders.
    pub(super) fn holder_for(&self, focus: &[Value], i: usize) -> Option<Holder> {
        match &self.lambda_holders {
            Some((at, holders)) if *at == focus.as_ptr() as usize && holders.len() == focus.len() => {
                holders.get(i).cloned().flatten()
            }
            _ => None,
        }
    }

    /// Bind the holder of `$this`; the previous one is returned, to restore.
    pub(super) fn set_this_holder(&mut self, holder: Option<Holder>) -> Option<Holder> {
        std::mem::replace(&mut self.this_holder, holder)
    }

    /// Run `f` with `holders` given to the lambda that iterates over
    /// `values` (and to no other).
    fn with_lambda_holders<T>(
        &mut self,
        values: &[Value],
        holders: &[Option<Holder>],
        f: impl FnOnce(&mut Env<'a>) -> T,
    ) -> T {
        let saved = self.lambda_holders.replace((values.as_ptr() as usize, holders.into()));
        let out = f(self);
        self.lambda_holders = saved;
        out
    }

    /// A smaller budget, so tests reach it without building half a gigabyte.
    #[cfg(test)]
    pub fn with_budget(mut self, bytes: usize) -> Self {
        self.budget = bytes;
        self
    }

    /// Evaluate under `work` instead of a budget of its own.
    pub fn with_work(mut self, work: Budget) -> Self {
        self.work = work;
        self
    }

    /// Count `steps` steps of work: an error once past the budget.
    pub fn step(&mut self, steps: usize) -> Result<(), String> {
        self.work.charge(steps as u64)
    }

    /// Use the `resolve()` index `kept` holds, when it is of this scope.
    pub fn with_references(mut self, kept: &mut References<'a>) -> Self {
        if kept.scope.is_some_and(|s| std::ptr::eq(s, self.scope)) {
            self.references = kept.index.take();
        }
        self
    }

    /// Hand the `resolve()` index, if there is one, back to `kept` for the
    /// next evaluation in this scope.
    pub fn keep_references(&mut self, kept: &mut References<'a>) {
        if let Some(index) = self.references.take() {
            kept.scope = Some(self.scope);
            kept.index = Some(index);
        }
    }

    /// Count `bytes` the evaluation builds against its budget; an error
    /// once past it, before more is built.
    pub fn spend(&mut self, bytes: usize) -> Result<(), String> {
        self.spent = self.spent.saturating_add(bytes);
        if self.spent > self.budget {
            return Err(format!(
                "The expression would build more than {} MB of values; narrow it down",
                self.budget >> 20
            ));
        }
        Ok(())
    }

    /// Count the values just returned by a step that did not count them
    /// itself: `already` is what that step spent while making them, so a
    /// `select()` of a `select()` is not counted twice.
    pub fn spend_on(&mut self, values: &[Value], already: usize) -> Result<(), String> {
        self.spend(approx_size_of(values).saturating_sub(already))
    }

    /// What has been spent so far, to measure one step with.
    pub fn spent(&self) -> usize {
        self.spent
    }

    /// Before `what` builds a string of `len` bytes: refused past
    /// [`MAX_STRING_BYTES`], counted otherwise.
    pub fn build_string(&mut self, what: &str, len: usize) -> Result<(), String> {
        if len > MAX_STRING_BYTES {
            return Err(format!(
                "{} result would exceed {} MB (the most one string may hold)",
                what,
                MAX_STRING_BYTES >> 20
            ));
        }
        self.spend(len.saturating_add(std::mem::size_of::<Value>()))
    }

    /// Follow a reference found in `item` (a Reference element or its
    /// string) to the resource it names.
    /// `holder` is the resource the reference sits in, when known, and
    /// `entry` the fullUrl of the Bundle entry holding it: `Some(None)` for
    /// an entry with no fullUrl, `None` when the path did not go through
    /// the entry.
    pub fn resolve(
        &mut self,
        reference: &str,
        item: &Value,
        holder: Option<&Value>,
        entry: Option<Option<&str>>,
    ) -> Result<Option<(Value, Option<Option<Rc<str>>>)>, String> {
        let (root, scope) = (self.root, self.scope);
        if self.references.is_none() {
            // A walk of the whole scope: once per validation when the
            // index is kept, otherwise once per evaluation.
            let (index, visited) = ResolveIndex::build(scope);
            self.references = Some(index);
            self.step(visited / INDEXED_PER_STEP)?;
        }
        Ok(self.references.as_ref().and_then(|index| index.resolve(reference, item, holder, entry, root)))
    }

    /// The context node ordinary function arguments are evaluated against:
    /// the item a lambda is iterating over, or the root resource.
    ///
    /// This is *not* the function's input collection, which is why
    /// `name.given.combine(name.family)` finds the family names — `name`
    /// resolves from the context, not from the given names the function
    /// happens to be operating on.
    pub fn context(&mut self) -> Result<Vec<Value>, String> {
        match &self.this {
            Some(this) => Ok(this.clone()),
            None => self.root_copy(),
        }
    }

    /// The root, copied, counted by its size: a criteria that copies a
    /// resource of several MB for every item is far more work than its
    /// handful of steps.
    fn root_copy(&mut self) -> Result<Vec<Value>, String> {
        let root = self.root;
        let steps = *self.root_steps.get_or_insert_with(|| approx_size(root) / BYTES_PER_STEP);
        self.step(steps)?;
        Ok(vec![root.clone()])
    }

    /// Re-bind `$this`/`$index` (and the holder of `$this`, when known) for
    /// one iteration of a lambda.
    fn with_item<T>(
        &mut self,
        item: &Value,
        index: usize,
        holder: Option<Holder>,
        f: impl FnOnce(&mut Env<'a>) -> T,
    ) -> T {
        let saved_this = self.this.take();
        let saved_index = self.index.take();
        let saved_holder = self.set_this_holder(holder);
        self.this = Some(vec![item.clone()]);
        self.index = Some(index as i64);
        let out = f(self);
        self.this = saved_this;
        self.index = saved_index;
        self.this_holder = saved_holder;
        out
    }
}

pub fn eval(expr: &Expr, focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    env.step(1)?;
    match expr {
        Expr::Literal(lit) => Ok(literal_value(lit)),

        Expr::Variable(v) => match v {
            // Outside a lambda, `$this` is the resource the evaluation
            // started from — that is what makes `x.subsetOf($this.name)`
            // mean what it says.
            Variable::This => match &env.this {
                Some(this) => Ok(this.clone()),
                None => env.root_copy(),
            },
            Variable::Index => Ok(env.index.map(|i| vec![Value::from(i)]).unwrap_or_default()),
            Variable::Total => Ok(env.total.clone().unwrap_or_default()),
        },

        Expr::EnvConstant(name) => env_constant(name, env),

        Expr::Member { base, name } => eval_member(base.as_deref(), name, focus, env),

        Expr::Function { base, name, args } => eval_function(base.as_deref(), name, args, focus, env),

        Expr::Index { base, index } => eval_index(base, index, focus, env),

        Expr::Unary { op, operand } => {
            let vals = eval(operand, focus, env)?;
            let Some(v) = singleton(&vals)? else {
                return Ok(vec![]);
            };
            match op {
                UnaryOp::Pos => Ok(vec![v.clone()]),
                UnaryOp::Neg => negate(v).map(|v| vec![v]),
            }
        }

        Expr::TypeOp { op, operand, type_name } => eval_type_op(*op, operand, type_name, focus, env),

        Expr::Binary { first, rest } => {
            // Folded in a loop, so a list of a thousand terms costs one
            // level of recursion rather than a thousand.
            let mut acc = eval(first, focus, env)?;
            // Whether `acc` is the result of a union, and so already holds
            // no duplicates.
            let mut distinct = false;
            for (op, right) in rest {
                if *op == BinOp::Union {
                    let r = eval(right, focus, env)?;
                    acc = union(acc, r, distinct, env)?;
                    distinct = true;
                } else {
                    acc = binary(*op, acc, right, focus, env)?;
                    distinct = false;
                }
            }
            Ok(acc)
        }
    }
}

// The larger arms of `eval` are functions of their own: in a debug build a
// function's stack frame holds the locals of all its arms, and `eval` is
// on the stack once per nesting level of the expression.

#[inline(never)]
fn eval_member(base: Option<&Expr>, name: &str, focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    // A primitive's id and extensions live in its `_name` sibling:
    // `Patient.birthDate.extension` reads `_birthDate.extension`.
    if name == "extension" || name == "id" {
        if let Some((values, elements)) = with_primitive_elements(base, focus, env)? {
            let mut out = member_access(&values, name, false, env.root);
            out.extend(member_access(&elements, name, false, env.root));
            env.step(out.len())?;
            return Ok(out);
        }
    }
    // `%resource.entry` reads the root where it is: a copy of a large
    // Bundle for every item of a `where()` is most of the work.
    let out = if matches!(base, Some(Expr::EnvConstant(c)) if is_root_constant(c)) {
        member_access(std::slice::from_ref(env.root), name, false, env.root)
    } else {
        let target = resolve_base(base, focus, env)?;
        member_access(&target, name, base.is_none(), env.root)
    };
    env.step(out.len())?;
    Ok(out)
}

#[inline(never)]
fn eval_function(
    base: Option<&Expr>,
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    // A type test straight on a path keeps the type a choice
    // element declares in its key (`effectiveDateTime`).
    if matches!(name, "ofType" | "is" | "as") {
        if let Some(typed) = typed_members(base, focus, env)? {
            return super::functions::type_call(name, args, &typed);
        }
    }
    if name == "extension" {
        if let Some((mut values, elements)) = with_primitive_elements(base, focus, env)? {
            values.extend(elements);
            return super::functions::call(name, args, &values, env);
        }
    }
    // resolve() needs to know which resource each reference sits
    // in, so `#id` finds that resource's contained one.
    if name == "resolve" && args.is_empty() {
        return Ok(resolved(base, focus, env)?.into_iter().map(|(v, _)| v).collect());
    }
    // A lambda whose criteria or projection calls resolve()
    // (`select(subject.resolve())`, `where(subject.resolve().exists())`):
    // its input is read with holders, and each item's goes to the lambda
    // as `$this`'s, so the resolve() inside answers where the item sits.
    if LAMBDAS.contains(&name) && args.iter().any(mentions_resolve) {
        let items = match base {
            Some(b) => with_holders(b, focus, env)?,
            None => focus_holders(focus, env),
        };
        let (values, holders): (Vec<Value>, Vec<Option<Holder>>) = items.into_iter().unzip();
        return env.with_lambda_holders(&values, &holders, |env| super::functions::call(name, args, &values, env));
    }
    let target = resolve_base(base, focus, env)?;
    super::functions::call(name, args, &target, env)
}

/// What resolve() on `base` finds, each found resource with its holder:
/// itself, in the entry it was found in (a contained one in its
/// container's), so a resolve() chained on it answers on its own server.
fn resolved(base: Option<&Expr>, focus: &[Value], env: &mut Env) -> Result<Vec<(Value, Option<Holder>)>, String> {
    let items = match base {
        Some(b) => with_holders(b, focus, env)?,
        // `subject.select(resolve())`: the focus is a lambda's item.
        None => focus_holders(focus, env),
    };
    let mut out = Vec::new();
    for (item, holder) in &items {
        env.step(1)?;
        let reference = item
            .get("reference")
            .and_then(|r| r.as_str())
            .or_else(|| item.as_str());
        if let Some(r) = reference {
            let resource = holder.as_ref().map(|h| &*h.resource);
            let entry = holder.as_ref().and_then(|h| h.entry.as_ref()).map(|url| url.as_deref());
            // Each is a copy of a whole resource.
            if let Some((found, found_in)) = env.resolve(r, item, resource, entry)? {
                env.spend(approx_size(&found))?;
                let holder = Holder { resource: Rc::new(found.clone()), entry: found_in };
                out.push((found, Some(holder)));
            }
        }
    }
    Ok(out)
}

/// The functions that evaluate an argument once per input item with
/// `$this` bound.
const LAMBDAS: &[&str] = &["where", "select", "all", "exists", "repeat", "aggregate", "iif", "sort", "trace"];

#[inline(never)]
fn eval_index(base: &Expr, index: &Expr, focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    let target = eval(base, focus, env)?;
    let idx = eval(index, focus, env)?;
    let Some(i) = singleton(&idx)? else {
        return Ok(vec![]);
    };
    let Some(i) = i.as_i64() else {
        return Err("Index must be an integer".into());
    };
    if i < 0 {
        return Ok(vec![]);
    }
    Ok(target.get(i as usize).cloned().into_iter().collect())
}

#[inline(never)]
fn eval_type_op(
    op: TypeOp,
    operand: &Expr,
    type_name: &str,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    let typed = match typed_members(Some(operand), focus, env)? {
        Some(t) => t,
        None => eval(operand, focus, env)?.into_iter().map(|v| (v, None)).collect(),
    };
    let (v, declared) = match typed.as_slice() {
        [] => return Ok(vec![]),
        [one] => one,
        many => return Err(format!("Expected a single value but the expression produced {}", many.len())),
    };
    let matches = types::is_declared_type(v, declared.as_deref(), type_name);
    match op {
        TypeOp::Is => Ok(vec![Value::Bool(matches)]),
        TypeOp::As => Ok(if matches { vec![v.clone()] } else { vec![] }),
    }
}

/// The focus a member access or function call applies to: the base
/// expression's result, or the incoming focus for a leading element.
fn resolve_base(
    base: Option<&Expr>,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    match base {
        Some(b) => eval(b, focus, env),
        None => Ok(focus.to_vec()),
    }
}

/// `%resource` and its synonyms: the resource the evaluation started from.
fn is_root_constant(name: &str) -> bool {
    matches!(name, "resource" | "context" | "rootResource")
}

fn env_constant(name: &str, env: &mut Env) -> Result<Vec<Value>, String> {
    if is_root_constant(name) {
        return env.root_copy();
    }
    match name {
        "ucum" => Ok(vec![Value::from("http://unitsofmeasure.org")]),
        "sct" => Ok(vec![Value::from("http://snomed.info/sct")]),
        "loinc" => Ok(vec![Value::from("http://loinc.org")]),
        // `%vs-<name>` and `%ext-<name>` expand to the canonical URL of a
        // core FHIR value set or extension.
        other => match other.split_once('-') {
            Some(("vs", name)) => Ok(vec![Value::from(format!(
                "http://hl7.org/fhir/ValueSet/{}",
                name
            ))]),
            Some(("ext", name)) => Ok(vec![Value::from(format!(
                "http://hl7.org/fhir/StructureDefinition/{}",
                name
            ))]),
            _ => Err(format!("Unknown environment constant %{}", other)),
        },
    }
}

fn literal_value(lit: &Literal) -> Vec<Value> {
    match lit {
        Literal::Null => vec![],
        Literal::Bool(b) => vec![Value::Bool(*b)],
        Literal::Integer(n) => vec![Value::from(*n)],
        Literal::Decimal(n) => vec![Value::Number(n.clone())],
        Literal::Str(s) => vec![Value::from(s.clone())],
        Literal::DateTime(s) => vec![Value::from(s.clone())],
        Literal::Quantity(v, u) => {
            let mut m = Map::new();
            m.insert("value".into(), Value::Number(v.clone()));
            m.insert("unit".into(), Value::from(u.clone()));
            vec![Value::Object(m)]
        }
    }
}

// ---------------------------------------------------------------------------
// Path navigation
// ---------------------------------------------------------------------------

/// Resolve `name` against every item of `focus`.
///
/// `leading` marks the first element of a path, where a name matching the
/// resource type acts as a filter on the context rather than a member lookup
/// (`Patient.name` evaluated with a Patient as context).
pub fn member_access(focus: &[Value], name: &str, leading: bool, root: &Value) -> Vec<Value> {
    if leading {
        // `Patient.name` / bare `Patient` against a Patient resource.
        if focus.len() == 1 && resource_type_of(&focus[0]) == Some(name) {
            return focus.to_vec();
        }
        // A leading type name with no focus of its own resolves against the
        // resource the evaluation started from.
        if focus.is_empty() && resource_type_of(root) == Some(name) {
            return vec![root.clone()];
        }
    }

    let mut out = Vec::new();
    for item in focus {
        let Some(obj) = item.as_object() else { continue };
        match obj.get(name) {
            Some(Value::Array(arr)) => out.extend(arr.iter().cloned()),
            Some(v) => out.push(v.clone()),
            None => {
                // FHIR choice elements are serialised with the type appended
                // to the element name, so `Observation.value` has to find
                // `valueQuantity`, `valueString`, …
                if let Some(v) = choice_element(obj, name) {
                    match v {
                        Value::Array(arr) => out.extend(arr.iter().cloned()),
                        v => out.push(v.clone()),
                    }
                }
            }
        }
    }
    out
}

/// A `<name><Type>` key, e.g. `valueQuantity` for `value`.
fn choice_element<'a>(obj: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    choice_entry(obj, name).map(|(_, v)| v)
}

/// The type suffix and value of a `<name><Type>` key.
fn choice_entry<'a, 'k>(obj: &'a Map<String, Value>, name: &'k str) -> Option<(&'a str, &'a Value)> {
    obj.iter()
        .find(|(k, _)| {
            k.len() > name.len()
                && k.starts_with(name)
                && k[name.len()..].starts_with(|c: char| c.is_ascii_uppercase())
        })
        .map(|(k, v)| (&k[name.len()..], v))
}

/// When `expr` is a path step `parent.name`: its values, plus the
/// `_name` Elements that carry the id and extensions of primitive values
/// (for a repeating primitive, the non-null entries of the `_name` array).
fn with_primitive_elements(
    expr: Option<&Expr>,
    focus: &[Value],
    env: &mut Env,
) -> Result<Option<(Vec<Value>, Vec<Value>)>, String> {
    let Some(Expr::Member { base, name }) = expr else { return Ok(None) };
    let parents = resolve_base(base.as_deref(), focus, env)?;
    let values = member_access(&parents, name, base.is_none(), env.root);
    let sibling = format!("_{}", name);
    let mut elements = Vec::new();
    for parent in &parents {
        match parent.get(&sibling) {
            Some(Value::Array(arr)) => elements.extend(arr.iter().filter(|v| v.is_object()).cloned()),
            Some(v @ Value::Object(_)) => elements.push(v.clone()),
            _ => {}
        }
    }
    env.step(values.len() + elements.len())?;
    Ok(Some((values, elements)))
}

/// The resource a value was reached through, and where that resource sits
/// in a Bundle when the path went through its entry.
#[derive(Clone)]
pub(super) struct Holder {
    resource: Rc<Value>,
    /// `Some(fullUrl)` when the path reached the resource as an entry's
    /// `resource`, or inside one (`Some(None)`: an entry with no fullUrl);
    /// `None` when it did not. Two identical resources under two servers'
    /// fullUrls are told apart by this, not by their content.
    entry: Option<Option<Rc<str>>>,
}

/// The items `expr` yields, each with the nearest resource it was reached
/// through along the path (`Bundle.entry[1].resource.basedOn` → entry 1's
/// resource, with entry 1's fullUrl). Steps other than a member access keep
/// only resources themselves as holders.
fn with_holders(
    expr: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<(Value, Option<Holder>)>, String> {
    match expr {
        // `birthDate.extension`, `birthDate.id`: as eval_member reads them,
        // from the values and from their `_birthDate` sibling Elements,
        // each keeping the holder of the resource it sits in.
        Expr::Member { base: Some(b), name } if matches!(name.as_str(), "extension" | "id") => {
            let parents = match primitive_parents(b, focus, env)? {
                Some(items) => items,
                None => with_holders(b, focus, env)?,
            };
            Ok(member_step(&parents, name, env)?.into_iter().map(|(v, h, _)| (v, h)).collect())
        }
        Expr::Member { base: Some(b), name } => {
            let parents = with_holders(b, focus, env)?;
            Ok(member_step(&parents, name, env)?.into_iter().map(|(v, h, _)| (v, h)).collect())
        }
        Expr::Member { base: None, .. } => {
            let holder = match focus_holders(focus, env).as_slice() {
                [(_, h)] => h.clone(),
                _ => None,
            };
            Ok(eval(expr, focus, env)?.into_iter().map(|v| tag(v, &holder, None)).collect())
        }
        // `$this` inside a lambda: the item, with the holder the lambda
        // was given for it.
        Expr::Variable(Variable::This) if env.this.is_some() => {
            let this = env.this.clone().unwrap_or_default();
            Ok(focus_holders(&this, env))
        }
        // resolve(): each found resource in the entry it was found in.
        Expr::Function { base, name, args } if name == "resolve" && args.is_empty() => resolved(base.as_deref(), focus, env),
        // A function that returns some of its input items as they are:
        // each keeps the holder it was reached with, so
        // `entry.resource.ofType(X).first().subject` still knows its entry.
        Expr::Function { base: Some(b), name, args } if KEEPS_ITEMS.contains(&name.as_str()) => {
            // ofType() and as() on a path step read the type its JSON key
            // declares (`valueQuantity`), as eval_function does.
            let (items, declared) = if matches!(name.as_str(), "ofType" | "as") {
                with_declared(b, focus, env)?
            } else {
                let items = with_holders(b, focus, env)?;
                let n = items.len();
                (items, vec![None; n])
            };
            let (values, holders): (Vec<Value>, Vec<Option<Holder>>) = items.into_iter().unzip();
            // where()'s criteria sees each item's holder, for a resolve() in it.
            let keep = env.with_lambda_holders(&values, &holders, |env| kept_indices(name, args, &values, &declared, env))?;
            let mut wanted = keep.into_iter().peekable();
            Ok(values
                .into_iter()
                .zip(holders)
                .enumerate()
                .filter_map(|(i, pair)| (wanted.peek() == Some(&i)).then(|| {
                    wanted.next();
                    pair
                }))
                .collect())
        }
        // `x as Observation`: the one item, with its holder, when it is of
        // that type (as eval_type_op decides).
        Expr::TypeOp { op: TypeOp::As, operand, type_name } => {
            let (mut items, declared) = with_declared(operand, focus, env)?;
            match items.len() {
                0 => Ok(vec![]),
                1 if types::is_declared_type(&items[0].0, declared[0].as_deref(), type_name) => Ok(vec![items.swap_remove(0)]),
                1 => Ok(vec![]),
                n => Err(format!("Expected a single value but the expression produced {}", n)),
            }
        }
        // `contained[0]`, `entry[1]`: the item keeps the holder it was
        // reached with, as eval_index picks it.
        Expr::Index { base, index } => {
            let mut items = with_holders(base, focus, env)?;
            let idx = eval(index, focus, env)?;
            let Some(i) = singleton(&idx)? else {
                return Ok(vec![]);
            };
            let Some(i) = i.as_i64() else {
                return Err("Index must be an integer".into());
            };
            if i < 0 || i as usize >= items.len() {
                return Ok(vec![]);
            }
            Ok(vec![items.swap_remove(i as usize)])
        }
        // union() and combine(): items from two collections, each with the
        // holder it was reached with; union() keeps first occurrences.
        Expr::Function { base: Some(b), name, args } if matches!(name.as_str(), "union" | "combine") => {
            let mut items = with_holders(b, focus, env)?;
            let arg = super::functions::one_arg(name, args)?;
            let context = env.context()?;
            let other = with_holders(arg, &context, env)?;
            let size = items.iter().chain(&other).map(|(v, _)| approx_size_of(std::slice::from_ref(v))).sum::<usize>();
            env.spend(size)?;
            items.extend(other);
            if name == "union" {
                distinct_items(items, env)
            } else {
                Ok(items)
            }
        }
        // `a | b | c`: the same as union(), term by term.
        Expr::Binary { first, rest } if !rest.is_empty() && rest.iter().all(|(op, _)| *op == BinOp::Union) => {
            let mut items = with_holders(first, focus, env)?;
            for (_, right) in rest {
                items.extend(with_holders(right, focus, env)?);
            }
            distinct_items(items, env)
        }
        // extension(url): the extensions of each item (a primitive's from
        // its `_name` sibling too, as eval_function reads them), each with
        // the holder of the item it sits in.
        Expr::Function { base: Some(b), name, args } if name == "extension" => {
            let items = match primitive_parents(b, focus, env)? {
                Some(items) => items,
                None => with_holders(b, focus, env)?,
            };
            let mut out = Vec::new();
            for (v, holder) in &items {
                for y in super::functions::call(name, args, std::slice::from_ref(v), env)? {
                    out.push(tag(y, holder, None));
                }
            }
            Ok(out)
        }
        // Functions computed item by item: what each item gives keeps its
        // holder. select() binds $this and $index as it does; iif() has
        // one input at most.
        Expr::Function { base, name, args } if matches!(name.as_str(), "select" | "children" | "descendants" | "iif") => {
            let items = match base {
                Some(b) => with_holders(b, focus, env)?,
                None => focus_holders(focus, env),
            };
            let (values, holders): (Vec<Value>, Vec<Option<Holder>>) = items.into_iter().unzip();
            let per_item: Vec<Vec<Value>> = env.with_lambda_holders(&values, &holders, |env| {
                Ok::<_, String>(match name.as_str() {
                    "select" => {
                        let projection = super::functions::one_arg(name, args)?;
                        eval_per_item(projection, &values, env)?.into_iter().map(|(_, res)| res).collect()
                    }
                    "iif" => vec![super::functions::call(name, args, &values, env)?],
                    _ => {
                        let mut out = Vec::with_capacity(values.len());
                        for v in &values {
                            out.push(super::functions::call(name, args, std::slice::from_ref(v), env)?);
                        }
                        out
                    }
                })
            })?;
            // Something inside the item (children, descendants) is in its
            // entry; a resource a projection reached (a resolve(), say) may
            // not be, and is found by content instead.
            let inside = matches!(name.as_str(), "children" | "descendants");
            let mut out = Vec::new();
            for (i, results) in per_item.into_iter().enumerate() {
                let holder = holders.get(i).cloned().flatten();
                for y in results {
                    let pair = match values.get(i) {
                        Some(x) if *x == y => (y, holder.clone()),
                        _ if inside => tag(y, &holder, None),
                        _ if is_resource(&y) => tag(y, &None, None),
                        _ => (y, holder.clone()),
                    };
                    out.push(pair);
                }
            }
            Ok(out)
        }
        // Any other function (resolve() itself and is(), which
        // eval_function reads in its own way, excepted): each item it
        // returns that is one of its input items, unchanged, gets that
        // item's holder (see `holders_by_value`). A lambda in it (all(),
        // exists(), repeat(), aggregate(), sort()) sees each item's holder.
        Expr::Function { base, name, args } if !matches!(name.as_str(), "resolve" | "is" | "as" | "ofType") => {
            let items = match base {
                Some(b) => with_holders(b, focus, env)?,
                None => focus_holders(focus, env),
            };
            let (values, holders): (Vec<Value>, Vec<Option<Holder>>) = items.into_iter().unzip();
            let out = env.with_lambda_holders(&values, &holders, |env| super::functions::call(name, args, &values, env))?;
            holders_by_value(out, &values, &holders, env)
        }
        _ => Ok(eval(expr, focus, env)?.into_iter().map(|v| tag(v, &None, None)).collect()),
    }
}

/// The items of `expr` with their holders, and the type each one's JSON key
/// declares when `expr` is a path step (`Quantity` for `valueQuantity`), as
/// typed_members reads it for ofType(), as() and `as`.
#[allow(clippy::type_complexity)]
fn with_declared(
    expr: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<(Vec<(Value, Option<Holder>)>, Vec<Option<String>>), String> {
    match expr {
        Expr::Member { base: Some(bb), name: m } => {
            let parents = with_holders(bb, focus, env)?;
            Ok(member_step(&parents, m, env)?.into_iter().map(|(v, h, d)| ((v, h), d)).unzip())
        }
        Expr::Member { base: None, .. } => {
            let items = with_holders(expr, focus, env)?;
            let declared = match typed_members(Some(expr), focus, env)? {
                Some(typed) if typed.len() == items.len() => typed.into_iter().map(|(_, d)| d).collect(),
                _ => vec![None; items.len()],
            };
            Ok((items, declared))
        }
        _ => {
            let items = with_holders(expr, focus, env)?;
            let n = items.len();
            Ok((items, vec![None; n]))
        }
    }
}

/// The items of the current focus with their holders: `$this`'s, when the
/// focus is the item a lambda is iterating over and the lambda knows it;
/// otherwise each resource is its own holder, in no known entry.
fn focus_holders(focus: &[Value], env: &Env) -> Vec<(Value, Option<Holder>)> {
    if let (Some(holder), Some(this), [one]) = (&env.this_holder, &env.this, focus) {
        if this.len() == 1 && this[0] == *one {
            return vec![(one.clone(), Some(holder.clone()))];
        }
    }
    focus.iter().map(|v| tag(v.clone(), &None, None)).collect()
}

/// For a step `b.name` where `b` is itself a path step `parent.m`: the
/// items `name` is read from as eval_member and eval_function read a
/// primitive's id and extensions, the values of `m` and the `_m` sibling
/// Elements, each with the holder of the resource it sits in. `None` when
/// `b` is not a path step.
fn primitive_parents(
    b: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<Option<Vec<(Value, Option<Holder>)>>, String> {
    let Expr::Member { base: bb, name: m } = b else { return Ok(None) };
    let parents: Vec<(Value, Option<Holder>)> = match bb {
        Some(bb) => with_holders(bb, focus, env)?,
        None => focus_holders(focus, env),
    };
    let mut items: Vec<(Value, Option<Holder>)> = match bb {
        Some(_) => member_step(&parents, m, env)?.into_iter().map(|(v, h, _)| (v, h)).collect(),
        // A leading step (`Patient`) can be the focus itself.
        None => with_holders(b, focus, env)?,
    };
    let sibling = format!("_{}", m);
    for (parent, holder) in &parents {
        match parent.get(&sibling) {
            Some(Value::Array(arr)) => {
                items.extend(arr.iter().filter(|v| v.is_object()).map(|v| (v.clone(), holder.clone())))
            }
            Some(v @ Value::Object(_)) => items.push((v.clone(), holder.clone())),
            _ => {}
        }
    }
    env.step(items.len())?;
    Ok(Some(items))
}

/// Whether `expr` calls resolve() anywhere in it.
fn mentions_resolve(expr: &Expr) -> bool {
    match expr {
        Expr::Function { base, name, args } => {
            name == "resolve" || base.as_deref().is_some_and(mentions_resolve) || args.iter().any(mentions_resolve)
        }
        Expr::Member { base, .. } => base.as_deref().is_some_and(mentions_resolve),
        Expr::Index { base, index } => mentions_resolve(base) || mentions_resolve(index),
        Expr::Unary { operand, .. } | Expr::TypeOp { operand, .. } => mentions_resolve(operand),
        Expr::Binary { first, rest } => mentions_resolve(first) || rest.iter().any(|(_, e)| mentions_resolve(e)),
        Expr::Literal(_) | Expr::Variable(_) | Expr::EnvConstant(_) => false,
    }
}

/// `items` without later equal ones, as union() and `|` keep them.
fn distinct_items(items: Vec<(Value, Option<Holder>)>, env: &mut Env) -> Result<Vec<(Value, Option<Holder>)>, String> {
    let (values, holders): (Vec<Value>, Vec<Option<Holder>>) = items.into_iter().unzip();
    let keep = first_occurrences((0..values.len()).collect(), &values, env)?;
    let mut wanted = keep.into_iter().peekable();
    Ok(values
        .into_iter()
        .zip(holders)
        .enumerate()
        .filter_map(|(i, pair)| (wanted.peek() == Some(&i)).then(|| {
            wanted.next();
            pair
        }))
        .collect())
}

/// The entry a holder says its item is in, for comparing holders.
fn entry_of(holder: &Option<Holder>) -> Option<Option<&str>> {
    holder.as_ref().and_then(|h| h.entry.as_ref()).map(|e| e.as_deref())
}

/// Holders for what a function returned (`out`) from `values`: an item
/// equal to input items (the same JSON) takes their holder when they agree
/// on the entry. When equal copies come from entries that differ, and the
/// function returned every copy, the k-th returned copy takes the k-th
/// copy's holder (in order, as the functions keep it); when it returned
/// fewer, which copy is which cannot be told, and the item gets no entry,
/// so resolve() looks for it by content and answers only when the copies'
/// servers agree. Anything else is a new value with no holder.
fn holders_by_value(
    out: Vec<Value>,
    values: &[Value],
    holders: &[Option<Holder>],
    env: &mut Env,
) -> Result<Vec<(Value, Option<Holder>)>, String> {
    if values.is_empty() {
        return Ok(out.into_iter().map(|v| tag(v, &None, None)).collect());
    }
    // Copies of each output value among the inputs, and among the outputs
    // so far.
    let matches: Vec<Vec<usize>> = out
        .iter()
        .map(|y| (0..values.len()).filter(|&i| values[i] == *y).collect())
        .collect();
    env.step(out.len().saturating_mul(values.len()))?;
    let mut result = Vec::with_capacity(out.len());
    for (j, y) in out.iter().enumerate() {
        let same = &matches[j];
        let holder = match same.as_slice() {
            [] => None,
            [first, rest @ ..] if rest.iter().all(|&i| entry_of(&holders[i]) == entry_of(&holders[*first])) => {
                Some(holders[*first].clone())
            }
            _ => {
                let returned = matches.iter().filter(|m| **m == *same).count();
                let k = matches[..j].iter().filter(|m| **m == *same).count();
                (returned == same.len()).then(|| holders[same[k]].clone())
            }
        };
        result.push(match holder {
            Some(h) => (y.clone(), h),
            None => tag(y.clone(), &None, None),
        });
    }
    Ok(result)
}

/// The functions that return some of their input items unchanged, in
/// order (see [`kept_indices`]).
const KEEPS_ITEMS: &[&str] = &[
    "first", "last", "tail", "skip", "take", "single", "ofType", "as", "where", "distinct", "intersect", "exclude", "trace",
];

fn is_resource(v: &Value) -> bool {
    v.get("resourceType").is_some_and(|t| t.is_string())
}

/// `v` with its holder: itself when it is a resource (shared, so the
/// entries of a Bundle do not each copy it), else the one it was reached
/// with. `entry` is the Bundle entry `v` is the `resource` of, if it is. A
/// resource inside another (a contained one) stays in the entry of the
/// resource holding it: its references resolve on that entry's server.
fn tag(v: Value, holder: &Option<Holder>, entry: Option<&Value>) -> (Value, Option<Holder>) {
    let h = if is_resource(&v) {
        let entry = match entry {
            Some(e) => Some(e.get("fullUrl").and_then(|u| u.as_str()).map(Rc::from)),
            None => holder.as_ref().and_then(|h| h.entry.clone()),
        };
        Some(Holder { resource: Rc::new(v.clone()), entry })
    } else {
        holder.clone()
    };
    (v, h)
}

/// `parent.name` for each parent, with holders, and the type a choice
/// element's key declares (`Quantity` for `valueQuantity`), as
/// typed_members reads it.
fn member_step(
    parents: &[(Value, Option<Holder>)],
    name: &str,
    env: &mut Env,
) -> Result<Vec<(Value, Option<Holder>, Option<String>)>, String> {
    let mut out = Vec::new();
    for (parent, holder) in parents {
        let found = member_access(std::slice::from_ref(parent), name, false, env.root);
        env.step(found.len())?;
        let declared = parent
            .as_object()
            .filter(|o| !o.contains_key(name))
            .and_then(|o| choice_entry(o, name))
            .map(|(suffix, _)| suffix.to_string());
        // `entry.resource`: the parent is the entry, with its fullUrl.
        let entry = (name == "resource" && parent.is_object() && !is_resource(parent)).then_some(parent);
        for v in found {
            let (v, h) = tag(v, holder, entry);
            out.push((v, h, declared.clone()));
        }
    }
    Ok(out)
}

/// The positions of the items a [`KEEPS_ITEMS`] function keeps from
/// `values` (whose declared types, for ofType(), are `declared`), in
/// increasing order: the same choice functions::call makes, as positions.
fn kept_indices(
    name: &str,
    args: &[Expr],
    values: &[Value],
    declared: &[Option<String>],
    env: &mut Env,
) -> Result<Vec<usize>, String> {
    use super::functions::{eval_arg, int_arg, no_args, one_arg, type_arg};
    let n = values.len();
    Ok(match name {
        "first" => {
            no_args(name, args)?;
            (0..n.min(1)).collect()
        }
        "last" => {
            no_args(name, args)?;
            (n.saturating_sub(1)..n).collect()
        }
        "tail" => {
            no_args(name, args)?;
            (1.min(n)..n).collect()
        }
        "skip" => {
            let k = int_arg(name, args, values, env)?.max(0) as usize;
            (k.min(n)..n).collect()
        }
        "take" => {
            let k = int_arg(name, args, values, env)?.max(0) as usize;
            (0..k.min(n)).collect()
        }
        "single" => {
            no_args(name, args)?;
            match n {
                0 | 1 => (0..n).collect(),
                n => return Err(format!("single() expects one item but found {}", n)),
            }
        }
        "ofType" => {
            let type_name = type_arg(name, args)?;
            (0..n)
                .filter(|&i| types::is_declared_type(&values[i], declared.get(i).and_then(|d| d.as_deref()), &type_name))
                .collect()
        }
        "where" => {
            let criteria = one_arg(name, args)?;
            let mut keep = Vec::new();
            for (i, (_, res)) in eval_per_item(criteria, values, env)?.into_iter().enumerate() {
                if as_condition(&res)? {
                    keep.push(i);
                }
            }
            keep
        }
        "as" => {
            let type_name = type_arg(name, args)?;
            match singleton(values)? {
                Some(v) if types::is_declared_type(v, declared.first().and_then(|d| d.as_deref()), &type_name) => vec![0],
                _ => vec![],
            }
        }
        "distinct" => {
            no_args(name, args)?;
            first_occurrences((0..n).collect(), values, env)?
        }
        // The items also in the argument, without duplicates.
        "intersect" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            let mut both = Vec::new();
            for (i, v) in values.iter().enumerate() {
                if contains(&other, v, env)? {
                    both.push(i);
                }
            }
            first_occurrences(both, values, env)?
        }
        // The items not in the argument, duplicates kept.
        "exclude" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            let mut kept = Vec::new();
            for (i, v) in values.iter().enumerate() {
                if !contains(&other, v, env)? {
                    kept.push(i);
                }
            }
            kept
        }
        // Logs and returns its input as it is: logged the same way.
        "trace" => {
            super::functions::call(name, args, values, env)?;
            (0..n).collect()
        }
        _ => (0..n).collect(),
    })
}

/// The positions in `candidates` whose items are not equal to an earlier
/// one's, as dedup keeps first occurrences (each comparison a step).
fn first_occurrences(candidates: Vec<usize>, values: &[Value], env: &mut Env) -> Result<Vec<usize>, String> {
    let mut keep: Vec<usize> = Vec::with_capacity(candidates.len());
    for i in candidates {
        let found = keep.iter().position(|&j| equal(&values[j], &values[i]) == Some(true));
        env.step(found.map_or(keep.len(), |p| p + 1))?;
        if found.is_none() {
            keep.push(i);
        }
    }
    Ok(keep)
}

/// When `expr` is a path step, its items paired with the type a choice
/// element declares in its JSON key (`Date` for `effectiveDate`, `None`
/// for an ordinary element). JSON alone cannot tell a day-precision
/// `dateTime` from a `date`, or a `code` from a `string`; the key can.
pub fn typed_members(
    expr: Option<&Expr>,
    focus: &[Value],
    env: &mut Env,
) -> Result<Option<Vec<(Value, Option<String>)>>, String> {
    let Some(Expr::Member { base, name }) = expr else { return Ok(None) };
    let target = resolve_base(base.as_deref(), focus, env)?;
    let leading = base.is_none();
    if leading
        && ((target.len() == 1 && resource_type_of(&target[0]) == Some(name.as_str()))
            || (target.is_empty() && resource_type_of(env.root) == Some(name.as_str())))
    {
        return Ok(Some(
            member_access(&target, name, true, env.root).into_iter().map(|v| (v, None)).collect(),
        ));
    }
    let mut out = Vec::new();
    for item in &target {
        let Some(obj) = item.as_object() else { continue };
        let (declared, v) = match obj.get(name.as_str()) {
            Some(v) => (None, v),
            None => match choice_entry(obj, name) {
                Some((suffix, v)) => (Some(suffix.to_string()), v),
                None => continue,
            },
        };
        match v {
            Value::Array(arr) => out.extend(arr.iter().map(|x| (x.clone(), declared.clone()))),
            v => out.push((v.clone(), declared)),
        }
    }
    env.step(out.len())?;
    Ok(Some(out))
}

fn resource_type_of(v: &Value) -> Option<&str> {
    v.get("resourceType").and_then(|t| t.as_str())
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// `l op right`, with the left operand already evaluated.
fn binary(
    op: BinOp,
    l: Vec<Value>,
    right: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    // Boolean operators short-circuit on three-valued logic, so their
    // right operand is evaluated lazily.
    match op {
        BinOp::And => {
            let l = as_tri(&l)?;
            if l == Some(false) {
                return Ok(vec![Value::Bool(false)]);
            }
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (_, Some(false)) => vec![Value::Bool(false)],
                (Some(true), Some(true)) => vec![Value::Bool(true)],
                _ => vec![],
            });
        }
        BinOp::Or => {
            let l = as_tri(&l)?;
            if l == Some(true) {
                return Ok(vec![Value::Bool(true)]);
            }
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (_, Some(true)) => vec![Value::Bool(true)],
                (Some(false), Some(false)) => vec![Value::Bool(false)],
                _ => vec![],
            });
        }
        BinOp::Implies => {
            let l = as_tri(&l)?;
            if l == Some(false) {
                return Ok(vec![Value::Bool(true)]);
            }
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (_, Some(true)) => vec![Value::Bool(true)],
                (Some(true), Some(false)) => vec![Value::Bool(false)],
                _ => vec![],
            });
        }
        BinOp::Xor => {
            let l = as_tri(&l)?;
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (Some(a), Some(b)) => vec![Value::Bool(a != b)],
                _ => vec![],
            });
        }
        _ => {}
    }

    let r = eval(right, focus, env)?;

    match op {
        BinOp::Union => union(l, r, false, env),

        BinOp::Eq | BinOp::NotEq => {
            env.step(l.len().min(r.len()))?;
            let equal = collection_equality(&l, &r)?;
            Ok(opt_bool(if op == BinOp::Eq { equal } else { equal.map(|b| !b) }))
        }
        BinOp::Equiv | BinOp::NotEquiv => {
            // Each item may be compared with every item on the other side.
            env.step(l.len().saturating_mul(r.len()))?;
            let equivalent = collection_equivalence(&l, &r);
            Ok(vec![Value::Bool(if op == BinOp::Equiv { equivalent } else { !equivalent })])
        }

        BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte => {
            let (Some(a), Some(b)) = (singleton(&l)?, singleton(&r)?) else {
                return Ok(vec![]);
            };
            let Some(ord) = compare(a, b)? else {
                return Ok(vec![]);
            };
            let result = match op {
                BinOp::Lt => ord == Ordering::Less,
                BinOp::Gt => ord == Ordering::Greater,
                BinOp::Lte => ord != Ordering::Greater,
                _ => ord != Ordering::Less,
            };
            Ok(vec![Value::Bool(result)])
        }

        BinOp::In => {
            if l.is_empty() {
                return Ok(vec![]);
            }
            let Some(item) = singleton(&l)? else {
                return Ok(vec![]);
            };
            env.step(r.len())?;
            Ok(vec![Value::Bool(r.iter().any(|v| equal(item, v) == Some(true)))])
        }
        BinOp::Contains => {
            if r.is_empty() {
                return Ok(vec![]);
            }
            let Some(item) = singleton(&r)? else {
                return Ok(vec![]);
            };
            env.step(l.len())?;
            Ok(vec![Value::Bool(l.iter().any(|v| equal(v, item) == Some(true)))])
        }

        BinOp::Concat => {
            // '&' treats an empty operand as an empty string, which is the
            // whole reason it exists next to '+'.
            let a = l.first().map(string_of).unwrap_or_default();
            let b = r.first().map(string_of).unwrap_or_default();
            if l.len() > 1 || r.len() > 1 {
                return Err("'&' expects single values".into());
            }
            env.build_string("'&'", a.len().saturating_add(b.len()))?;
            Ok(vec![Value::from(format!("{}{}", a, b))])
        }

        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::IntDiv | BinOp::Mod => {
            let (Some(a), Some(b)) = (singleton(&l)?, singleton(&r)?) else {
                return Ok(vec![]);
            };
            // `+` on two strings joins them, like `&`.
            if let (BinOp::Add, Some(x), Some(y)) = (op, a.as_str(), b.as_str()) {
                env.build_string("'+'", x.len().saturating_add(y.len()))?;
            }
            arithmetic(op, a, b)
        }

        BinOp::And | BinOp::Or | BinOp::Xor | BinOp::Implies => unreachable!("handled above"),
    }
}

/// `l | r`: the items of both without duplicates, first occurrences kept.
/// `l_distinct` says `l` is itself a union's result, so only `r` needs
/// checking: a chain `'a' | 'b' | …` then compares each item with the ones
/// before it once, instead of all of them again at every step.
fn union(l: Vec<Value>, r: Vec<Value>, l_distinct: bool, env: &mut Env) -> Result<Vec<Value>, String> {
    let out = if l_distinct { l } else { dedup(l, env)? };
    add_distinct(out, r, env)
}

/// `out` (which holds no duplicates) with each item of `more` it does not
/// hold yet. Every comparison is a step: on long collections they add up
/// to the square of the length.
fn add_distinct(mut out: Vec<Value>, more: Vec<Value>, env: &mut Env) -> Result<Vec<Value>, String> {
    for v in more {
        let found = out.iter().position(|seen| equal(seen, &v) == Some(true));
        env.step(found.map_or(out.len(), |i| i + 1))?;
        if found.is_none() {
            out.push(v);
        }
    }
    Ok(out)
}

fn arithmetic(op: BinOp, a: &Value, b: &Value) -> Result<Vec<Value>, String> {
    // A date or time plus a duration moves the instant, keeping precision.
    if matches!(op, BinOp::Add | BinOp::Sub) {
        if let (Some(text), true) = (a.as_str(), types::is_quantity(b)) {
            if let Some(temporal) = types::as_temporal(text) {
                let (amount, unit) = types::quantity_parts(b).unwrap_or((0.0, String::new()));
                let raw = types::quantity_raw_unit(b).unwrap_or_default();
                // Only durations are meaningful here, and a calendar year or
                // month has to be written as one: UCUM 'a' and 'mo' are fixed
                // lengths that do not line up with calendar arithmetic.
                if types::ucum_for_time_unit(&raw).is_none()
                    && matches!(unit.as_str(), "a" | "mo")
                {
                    return Err(format!(
                        "A date needs a calendar duration: write `{} year(s)` or \
                         `{} month(s)` rather than the UCUM unit '{}'",
                        amount, amount, raw
                    ));
                }
                let signed = if op == BinOp::Sub { -amount } else { amount };
                // A result past the years a date can hold is empty, as an
                // integer overflow is.
                return match temporal.add_duration(signed, &unit) {
                    Ok(moved) => Ok(moved.map(|t| Value::from(t.to_literal())).into_iter().collect()),
                    Err(types::NotADuration) => Err(format!(
                        "'{}' is not a duration, so it cannot be added to a date",
                        raw
                    )),
                };
            }
        }
    }

    // String concatenation with '+'.
    if let (Some(x), Some(y)) = (a.as_str(), b.as_str()) {
        return match op {
            BinOp::Add => Ok(vec![Value::from(format!("{}{}", x, y))]),
            _ => Err(format!("Operator {:?} is not defined for strings", op)),
        };
    }

    if types::is_quantity(a) || types::is_quantity(b) {
        return quantity_arithmetic(op, a, b);
    }

    let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) else {
        return Err(format!(
            "Operator {:?} expects numbers, got {} and {}",
            op,
            types::type_name(a),
            types::type_name(b)
        ));
    };
    let both_int = a.is_i64() && b.is_i64();

    // Integers are exact, and a result the type cannot hold is empty (the
    // specification's rule for overflow), not a number rounded through
    // f64 or saturated at the bound.
    if let (true, Some(x), Some(y)) = (both_int, a.as_i64(), b.as_i64()) {
        if !matches!(op, BinOp::Div) {
            return Ok(integer_arithmetic(op, x, y).map(Value::from).into_iter().collect());
        }
    }

    // FHIRPath decimals are exact; f64 is not. Rounding the result to the
    // scale exact decimal arithmetic would have produced keeps
    // `1.8 - 1.2 = 0.6` true instead of 0.5999999999999999.
    let scale = |places: u32, v: f64| round_to(v, places);
    let (sa, sb) = (decimals(a), decimals(b));

    let result = match op {
        BinOp::Add => scale(sa.max(sb), x + y),
        BinOp::Sub => scale(sa.max(sb), x - y),
        BinOp::Mul => scale(sa + sb, x * y),
        BinOp::Div => {
            if y == 0.0 {
                return Ok(vec![]);
            }
            // '/' always produces a decimal, even for two integers.
            return Ok(vec![Value::from(x / y)]);
        }
        BinOp::IntDiv => {
            if y == 0.0 {
                return Ok(vec![]);
            }
            let q = (x / y).trunc();
            return Ok(vec![if both_int {
                Value::from(q as i64)
            } else {
                Value::from(q)
            }]);
        }
        BinOp::Mod => {
            if y == 0.0 {
                return Ok(vec![]);
            }
            let m = scale(sa.max(sb), x % y);
            return Ok(vec![if both_int {
                Value::from(m as i64)
            } else {
                Value::from(m)
            }]);
        }
        _ => unreachable!(),
    };

    Ok(vec![if both_int {
        Value::from(result as i64)
    } else {
        Value::from(result)
    }])
}

/// `+ - * div mod` on two integers; `None` for division by zero or a
/// result out of range. FHIRPath's Integer is 32-bit; operands already
/// beyond that (an `integer64` from the resource) are held to 64 bits.
fn integer_arithmetic(op: BinOp, x: i64, y: i64) -> Option<i64> {
    let result = match op {
        BinOp::Add => x.checked_add(y)?,
        BinOp::Sub => x.checked_sub(y)?,
        BinOp::Mul => x.checked_mul(y)?,
        BinOp::IntDiv => x.checked_div(y)?,
        BinOp::Mod => x.checked_rem(y)?,
        _ => return None,
    };
    let fits32 = |n: i64| i32::try_from(n).is_ok();
    if fits32(x) && fits32(y) && !fits32(result) {
        return None;
    }
    Some(result)
}

fn quantity_arithmetic(op: BinOp, a: &Value, b: &Value) -> Result<Vec<Value>, String> {
    // Scaling a quantity by a plain number keeps the unit.
    if let (true, Some(n)) = (types::is_quantity(a), b.as_f64()) {
        let (av, au) = types::quantity_parts(a).unwrap();
        return match op {
            BinOp::Mul => Ok(vec![types::make_quantity(av * n, &au)]),
            BinOp::Div if n != 0.0 => Ok(vec![types::make_quantity(av / n, &au)]),
            BinOp::Div => Ok(vec![]),
            _ => Err(format!("Operator {:?} needs two quantities", op)),
        };
    }
    if let (Some(n), true) = (a.as_f64(), types::is_quantity(b)) {
        let (bv, bu) = types::quantity_parts(b).unwrap();
        return match op {
            BinOp::Mul => Ok(vec![types::make_quantity(n * bv, &bu)]),
            _ => Err(format!("Operator {:?} needs two quantities", op)),
        };
    }

    let (Some((av, au)), Some((bv, bu))) = (types::quantity_parts(a), types::quantity_parts(b))
    else {
        return Err(format!(
            "Operator {:?} is not defined between a {} and a {}",
            op,
            types::type_name(a),
            types::type_name(b)
        ));
    };

    match op {
        BinOp::Add | BinOp::Sub => {
            // Convert the right operand into the left's unit; units outside
            // the conversion table give the empty collection rather than a
            // wrong number.
            let Some(converted) = types::convert_quantity(bv, &bu, &au) else {
                return Ok(vec![]);
            };
            let v = if op == BinOp::Add {
                av + converted
            } else {
                av - converted
            };
            Ok(vec![types::make_quantity(v, &au)])
        }
        BinOp::Mul | BinOp::Div => Err(format!(
            "Operator {:?} between two quantities produces a compound unit, \
             which needs a full UCUM engine",
            op
        )),
        _ => Err(format!("Operator {:?} is not defined for quantities", op)),
    }
}

fn negate(v: &Value) -> Result<Value, String> {
    if let Value::Number(n) = v {
        return Ok(Value::Number(types::negated(n)));
    }
    if let Some((val, unit)) = types::quantity_parts(v) {
        return Ok(types::make_quantity(-val, &unit));
    }
    Err(format!("Cannot negate a {}", types::type_name(v)))
}

// ---------------------------------------------------------------------------
// Equality, equivalence and ordering
// ---------------------------------------------------------------------------

/// `=` over collections. `None` means the result is the empty collection:
/// either operand was empty, or an item comparison was indeterminate.
fn collection_equality(l: &[Value], r: &[Value]) -> Result<Option<bool>, String> {
    if l.is_empty() || r.is_empty() {
        return Ok(None);
    }
    if l.len() != r.len() {
        return Ok(Some(false));
    }
    let mut indeterminate = false;
    for (a, b) in l.iter().zip(r.iter()) {
        match equal(a, b) {
            Some(false) => return Ok(Some(false)),
            Some(true) => {}
            None => indeterminate = true,
        }
    }
    Ok(if indeterminate { None } else { Some(true) })
}

/// `~` over collections: order-insensitive, and empty ~ empty is true.
fn collection_equivalence(l: &[Value], r: &[Value]) -> bool {
    if l.len() != r.len() {
        return false;
    }
    let mut used = vec![false; r.len()];
    for a in l {
        let Some(pos) = r
            .iter()
            .enumerate()
            .position(|(i, b)| !used[i] && equivalent(a, b))
        else {
            return false;
        };
        used[pos] = true;
    }
    true
}

/// Single-value `=`. `None` when the comparison is indeterminate (dates of
/// differing precision that agree so far).
pub fn equal(a: &Value, b: &Value) -> Option<bool> {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => Some(x == y),
            // Beyond what a double holds (`1e400`): only the same text is
            // known to be the same number.
            _ => Some(a == b),
        },
        (Value::Bool(x), Value::Bool(y)) => Some(x == y),
        (Value::String(x), Value::String(y)) => {
            if let (Some(dx), Some(dy)) = (types::as_temporal(x), types::as_temporal(y)) {
                return types::temporal_equal(&dx, &dy);
            }
            Some(x == y)
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(false);
            }
            let mut indeterminate = false;
            for (i, j) in x.iter().zip(y.iter()) {
                match equal(i, j) {
                    Some(false) => return Some(false),
                    None => indeterminate = true,
                    Some(true) => {}
                }
            }
            if indeterminate {
                None
            } else {
                Some(true)
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            // Quantities compare by magnitude and normalised unit, so
            // `1 week` equals `1 'wk'` despite different JSON.
            if types::is_quantity(a) && types::is_quantity(b) {
                let (av, au) = types::quantity_parts(a)?;
                let (bv, bu) = types::quantity_parts(b)?;
                // A calendar year or month is not a fixed number of seconds,
                // so it cannot be measured against the UCUM unit of the same
                // name: `1 month = 1 'mo'` is unknowable, not false.
                let calendar = |v: &Value| {
                    types::quantity_raw_unit(v)
                        .is_some_and(|u| types::ucum_for_time_unit(&u).is_some())
                };
                if matches!(au.as_str(), "a" | "mo") && calendar(a) != calendar(b) {
                    return None;
                }
                // Units outside the conversion table are unknowable.
                return types::convert_quantity(bv, &bu, &au).map(|c| av == c);
            }
            if x.len() != y.len() {
                return Some(false);
            }
            for (k, v) in x {
                match y.get(k) {
                    Some(other) => match equal(v, other) {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => return None,
                    },
                    None => return Some(false),
                }
            }
            Some(true)
        }
        (Value::Null, Value::Null) => Some(true),
        _ => Some(false),
    }
}

/// Single-value `~`: strings compare case- and whitespace-insensitively,
/// decimals to the lesser precision, and there is no indeterminate result.
pub fn equivalent(a: &Value, b: &Value) -> bool {
    // Quantities are equivalent when they convert to the same magnitude at
    // the coarser of the two precisions.
    if types::is_quantity(a) && types::is_quantity(b) {
        let (Some((av, au)), Some((bv, bu))) = (types::quantity_parts(a), types::quantity_parts(b))
        else {
            return false;
        };
        let Some(converted) = types::convert_quantity(bv, &bu, &au) else {
            return au == bu && av == bv;
        };
        let places = decimals(a.get("value").unwrap_or(&Value::Null))
            .min(decimals(b.get("value").unwrap_or(&Value::Null)));
        return round_to(av, places) == round_to(converted, places);
    }

    match (a, b) {
        (Value::String(x), Value::String(y)) => {
            if let (Some(tx), Some(ty)) = (types::as_temporal(x), types::as_temporal(y)) {
                return types::temporal_equivalent(&tx, &ty);
            }
            normalize_ws(x) == normalize_ws(y)
        }
        (Value::Number(_), Value::Number(_)) => {
            let (x, y) = (a.as_f64().unwrap_or(f64::NAN), b.as_f64().unwrap_or(f64::NAN));
            let places = decimals(a).min(decimals(b));
            round_to(x, places) == round_to(y, places)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(i, j)| equivalent(i, j))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|other| equivalent(v, other)))
        }
        _ => equal(a, b).unwrap_or(false),
    }
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Digits after the point `v` was written with (`6.30`: 2), for rounding.
fn decimals(v: &Value) -> u32 {
    match v {
        Value::Number(n) => types::written_scale(n).max(0) as u32,
        _ => 0,
    }
}

fn round_to(v: f64, places: u32) -> f64 {
    let f = 10f64.powi(places.min(i32::MAX as u32) as i32);
    let rounded = (v * f).round() / f;
    // Past the digits a double holds the rounding means nothing, and its
    // arithmetic would overflow: the value stays as it is.
    if rounded.is_finite() { rounded } else { v }
}

/// Ordering for `<`, `>`, `<=`, `>=`. `None` means indeterminate.
pub fn compare(a: &Value, b: &Value) -> Result<Option<Ordering>, String> {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => Ok(x.partial_cmp(&y)),
            // A number beyond what a double holds (`1e400`, readable now that
            // numbers keep their text) cannot be placed.
            _ => Ok(None),
        },
        (Value::String(x), Value::String(y)) => {
            if let (Some(dx), Some(dy)) = (types::as_temporal(x), types::as_temporal(y)) {
                return Ok(types::temporal_compare(&dx, &dy));
            }
            Ok(Some(x.cmp(y)))
        }
        _ if types::is_quantity(a) && types::is_quantity(b) => {
            let (av, au) = types::quantity_parts(a).unwrap();
            let (bv, bu) = types::quantity_parts(b).unwrap();
            match types::convert_quantity(bv, &bu, &au) {
                Some(converted) => Ok(av.partial_cmp(&converted)),
                // Units outside the conversion table: unknowable.
                None => Ok(None),
            }
        }
        _ => Err(format!(
            "Cannot order a {} against a {}",
            types::type_name(a),
            types::type_name(b)
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers shared with the function library
// ---------------------------------------------------------------------------

/// The single value of a collection. `None` for the empty collection; an
/// error for more than one, which is what the FHIRPath spec requires of
/// operators and functions that expect a singleton.
pub fn singleton(vals: &[Value]) -> Result<Option<&Value>, String> {
    match vals.len() {
        0 => Ok(None),
        1 => Ok(Some(&vals[0])),
        n => Err(format!(
            "Expected a single value but the expression produced {}",
            n
        )),
    }
}

/// Three-valued boolean view of a collection: `None` is the empty collection.
///
/// Implements the spec's *singleton evaluation of collections*: a single item
/// converts to Boolean when it has a Boolean conversion (`1`/`0`,
/// `'true'`/`'false'`), and any other single item stands in for `true`.
pub fn as_tri(vals: &[Value]) -> Result<Option<bool>, String> {
    match singleton(vals)? {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        // Only Integer→Decimal→Quantity and Date→DateTime convert implicitly
        // in FHIRPath; nothing converts implicitly to Boolean. So the spec's
        // fallback applies and any other single item stands for `true` —
        // which is why `(0).not()` is false, not true.
        Some(_) => Ok(Some(true)),
    }
}

/// `where`-style criteria: the empty collection counts as false.
pub fn as_condition(vals: &[Value]) -> Result<bool, String> {
    Ok(as_tri(vals)?.unwrap_or(false))
}

fn opt_bool(b: Option<bool>) -> Vec<Value> {
    b.map(|b| vec![Value::Bool(b)]).unwrap_or_default()
}

/// Remove equivalent duplicates, keeping first occurrences.
pub fn dedup(vals: Vec<Value>, env: &mut Env) -> Result<Vec<Value>, String> {
    let out = Vec::with_capacity(vals.len());
    add_distinct(out, vals, env)
}

/// Whether `needle` is in `haystack`, each comparison counted as a step.
pub fn contains(haystack: &[Value], needle: &Value, env: &mut Env) -> Result<bool, String> {
    let found = haystack.iter().position(|v| equal(v, needle) == Some(true));
    env.step(found.map_or(haystack.len(), |i| i + 1))?;
    Ok(found.is_some())
}

/// String rendering used by `&`, `toString()` and `join()`.
pub fn string_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Evaluate `expr` once per item of `focus` with `$this`/`$index` bound.
pub fn eval_per_item(
    expr: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<(Value, Vec<Value>)>, String> {
    let mut out = Vec::with_capacity(focus.len());
    for (i, item) in focus.iter().enumerate() {
        // A step per item, whatever the criteria cost: nested lambdas
        // multiply these, and that is what the step budget is for.
        env.step(1)?;
        let single = [item.clone()];
        let before = env.spent();
        let holder = env.holder_for(focus, i);
        let res = env.with_item(item, i, holder, |env| eval(expr, &single, env))?;
        // Counted item by item: a projection that copies something big for
        // each item stops at the budget, not once all of them are built.
        let inside = env.spent() - before;
        env.spend_on(&res, inside)?;
        out.push((item.clone(), res));
    }
    Ok(out)
}

/// Depth-first walk of an item's direct children, in document order.
pub fn children_of(value: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            // resourceType is metadata of the JSON encoding, not an element.
            if k == "resourceType" {
                continue;
            }
            match v {
                Value::Array(arr) => out.extend(arr.iter().cloned()),
                v => out.push(v.clone()),
            }
        }
    }
    out
}

/// Every node under `value`, depth first, each a copy of its subtree: the
/// whole is about the tree's size times its depth, and chained
/// `descendants()` multiply that again, so each copy is counted.
pub fn descendants_of(value: &Value, env: &mut Env) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut stack: Vec<Value> = children_of(value);
    stack.reverse();
    while let Some(v) = stack.pop() {
        env.step(1)?;
        env.spend(approx_size(&v))?;
        let mut kids = children_of(&v);
        kids.reverse();
        stack.extend(kids);
        out.push(v);
    }
    Ok(out)
}

/// The `resolve()` index of one scope, kept from one evaluation to the
/// next. The rules of a validation all resolve in the same Bundle, and
/// building the index walks all of it: built again for each rule on each
/// entry, a 5000-entry Bundle was walked 5000 times.
#[derive(Default)]
pub struct References<'a> {
    scope: Option<&'a Value>,
    index: Option<ResolveIndex<'a>>,
}

/// Where each resource in the evaluation scope can be found. Holds
/// references into the scope, so one index serves every `resolve()` of an
/// evaluation without copying the Bundle.
struct ResolveIndex<'a> {
    /// Bundle entries by their fullUrl, as written.
    by_full_url: HashMap<&'a str, &'a Value>,
    /// `Type/id` of each resource (not the contained ones), and of each
    /// entry whose fullUrl ends in it: every match, on every server.
    by_identity: HashMap<String, Vec<&'a Value>>,
    /// Resources that contain other resources, for `#id` references.
    containers: Vec<&'a Value>,
    /// Bundle entries, their fullUrl (if any) and resource: the server a
    /// relative reference inside one of them points to.
    entries: Vec<(Option<&'a str>, &'a Value)>,
    /// The same, by the resource's address: the entry a resolved resource
    /// sits in, for the holder of what resolve() returns.
    entry_urls: HashMap<usize, Option<&'a str>>,
}

impl<'a> ResolveIndex<'a> {
    /// The index of `scope`, and how many values building it visited.
    fn build(scope: &'a Value) -> (Self, usize) {
        let mut index = ResolveIndex {
            by_full_url: HashMap::new(),
            by_identity: HashMap::new(),
            containers: Vec::new(),
            entries: Vec::new(),
            entry_urls: HashMap::new(),
        };
        let mut visited = 0;
        index.collect(scope, &mut visited);
        (index, visited)
    }

    fn collect(&mut self, value: &'a Value, visited: &mut usize) {
        *visited += 1;
        match value {
            Value::Object(map) => {
                if let (Some(rt), Some(id)) = (
                    map.get("resourceType").and_then(|v| v.as_str()),
                    map.get("id").and_then(|v| v.as_str()),
                ) {
                    self.by_identity.entry(format!("{}/{}", rt, id)).or_default().push(value);
                }
                if map.contains_key("resourceType") && map.get("contained").is_some_and(|c| c.is_array()) {
                    self.containers.push(value);
                }
                // A Bundle entry: the fullUrl names the resource beside it.
                if let (Some(url), Some(resource)) =
                    (map.get("fullUrl").and_then(|v| v.as_str()), map.get("resource"))
                {
                    self.by_full_url.entry(url).or_insert(resource);
                    if !url.starts_with("urn:") {
                        if let Some(identity) = identity_of(url) {
                            self.by_identity.entry(identity).or_default().push(resource);
                        }
                    }
                }
                if let Some(resource) = map.get("resource").filter(|r| is_resource(r)) {
                    let url = map.get("fullUrl").and_then(|v| v.as_str());
                    self.entries.push((url, resource));
                    self.entry_urls.entry(resource as *const Value as usize).or_insert(url);
                }
                for (key, v) in map {
                    // A contained resource is reached by `#id` from its
                    // container only, never by `Type/id`.
                    if key != "contained" {
                        self.collect(v, visited);
                    }
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    self.collect(v, visited);
                }
            }
            _ => {}
        }
    }

    /// The fullUrls of the Bundle entries `resource` (a copy of it, found by
    /// content: type and id first, then whole) sits in: those whose resource
    /// it is, else those whose resource contains it. More than one when the
    /// Bundle holds the same resource from several servers; `None` when it
    /// is in no entry (the resource evaluated is not inside the Bundle).
    /// The fullUrl of the entry `resource` (a value of the scope itself) is
    /// the resource of: `Some(None)` for an entry with none, `None` when it
    /// is in no entry.
    fn entry_of(&self, resource: &Value) -> Option<Option<Rc<str>>> {
        self.entry_urls.get(&(resource as *const Value as usize)).map(|url| url.map(Rc::from))
    }

    fn entries_holding(&self, resource: &Value) -> Option<Vec<Option<&'a str>>> {
        let key = (resource.get("resourceType"), resource.get("id"));
        let same = |r: &Value| (r.get("resourceType"), r.get("id")) == key && r == resource;
        let direct: Vec<Option<&'a str>> =
            self.entries.iter().filter(|(_, r)| same(r)).map(|(url, _)| *url).collect();
        if !direct.is_empty() {
            return Some(direct);
        }
        let containing: Vec<Option<&'a str>> = self
            .entries
            .iter()
            .filter(|(_, r)| r.get("contained").and_then(|c| c.as_array()).is_some_and(|c| c.iter().any(|x| same(x))))
            .map(|(url, _)| *url)
            .collect();
        (!containing.is_empty()).then_some(containing)
    }

    /// A relative reference (`Patient/1`, its type and id) from inside
    /// `holder`. `entry` is the fullUrl of the entry holding it when the
    /// path said which; otherwise the entries are found by content, and
    /// when identical copies sit under servers that would give different
    /// resources, nothing is given: no answer is better than another
    /// server's patient. Type and id alone only for a holder in no entry.
    fn resolve_relative(&self, identity: &str, holder: Option<&Value>, entry: Option<Option<&str>>) -> Option<&'a Value> {
        let on = |from: Option<&str>| {
            crate::parser::fhir::bundle::resolve_relative(
                identity,
                from,
                |url| self.by_full_url.get(url).copied(),
                |id| self.by_identity.get(id).cloned().unwrap_or_default(),
                // The same resource indexed twice (by its own id and by its
                // fullUrl), or identical copies: one resource.
                |a: &&'a Value, b: &&'a Value| std::ptr::eq(*a, *b) || a == b,
            )
        };
        if let Some(from) = entry {
            return on(from);
        }
        match holder.and_then(|h| self.entries_holding(h)) {
            None => on(None),
            Some(urls) => {
                let first = on(urls[0])?;
                urls[1..]
                    .iter()
                    .all(|url| on(*url).is_some_and(|other| std::ptr::eq(other, first)))
                    .then_some(first)
            }
        }
    }

    fn resolve(
        &self,
        reference: &str,
        item: &Value,
        holder: Option<&Value>,
        entry: Option<Option<&str>>,
        root: &Value,
    ) -> Option<(Value, Option<Option<Rc<str>>>)> {
        let target = reference.trim();

        // "#contained-id": a resource contained in the one the reference
        // sits in — the root, or the entry holding `item` in a Bundle. It
        // is in its container's entry.
        if let Some(id) = target.strip_prefix('#') {
            if let Some(found) = holder.and_then(|h| find_in_array(h.get("contained"), id)) {
                return Some((found, entry.map(|e| e.map(Rc::from))));
            }
            if let Some(found) = find_in_array(root.get("contained"), id) {
                return Some((found, None));
            }
            let holders: Vec<&'a Value> = self
                .containers
                .iter()
                .copied()
                .filter(|c| find_in_array(c.get("contained"), id).is_some())
                .collect();
            let holder = match holders.as_slice() {
                [one] => Some(*one),
                _ => holders.iter().copied().find(|c| contains_value(c, item)),
            };
            return holder.and_then(|c| {
                let entry = self.entry_of(c);
                find_in_array(c.get("contained"), id).map(|found| (found, entry))
            });
        }

        // A versioned reference matches with the version removed, then only
        // a resource at that version (FHIR's Bundle resolution rules): one
        // whose meta.versionId says otherwise is another version.
        let version = history_version(target);
        let target = strip_history(target);
        // An absolute reference (a URL or a urn) names an entry only by its
        // exact fullUrl: another server's `Practitioner/1` is not ours.
        let found = if target.starts_with("urn:") || target.contains("://") {
            self.by_full_url.get(target).copied()
        } else {
            // A relative one ("Patient/p1") names a resource by type and id.
            let identity = identity_of(target)?;
            if identity != target {
                return None;
            }
            // Inside an entry with a RESTful fullUrl it means that server's
            // resource only (FHIR's Bundle rule, as the Bundle view reads
            // it); type and id otherwise.
            self.resolve_relative(&identity, holder, entry)
        }?;
        if let Some(wanted) = version {
            let held = found.get("meta").and_then(|m| m.get("versionId")).and_then(Value::as_str);
            if held.is_some_and(|v| v != wanted) {
                return None;
            }
        }
        Some((found.clone(), self.entry_of(found)))
    }
}

/// `Type/id` at the end of a reference or fullUrl, if it has that shape.
fn identity_of(url: &str) -> Option<String> {
    let url = strip_history(url);
    let mut parts = url.rsplit('/');
    let id = parts.next()?;
    let rtype = parts.next()?;
    let is_type = rtype.starts_with(|c: char| c.is_ascii_uppercase())
        && rtype.chars().all(|c| c.is_ascii_alphanumeric());
    (is_type && !id.is_empty()).then(|| format!("{}/{}", rtype, id))
}

/// `Patient/1/_history/2` names Patient/1 (as it was at version 2).
fn strip_history(reference: &str) -> &str {
    match reference.find("/_history/") {
        Some(i) => &reference[..i],
        None => reference,
    }
}

/// The version a `…/_history/2` reference asks for.
fn history_version(reference: &str) -> Option<&str> {
    let (_, rest) = reference.split_once("/_history/")?;
    let v = rest.split(['/', '?', '#']).next().unwrap_or("");
    (!v.is_empty()).then_some(v)
}

/// Is `needle` somewhere inside `haystack` (or `haystack` itself)?
fn contains_value(haystack: &Value, needle: &Value) -> bool {
    if haystack == needle {
        return true;
    }
    match haystack {
        Value::Object(map) => map.values().any(|v| contains_value(v, needle)),
        Value::Array(arr) => arr.iter().any(|v| contains_value(v, needle)),
        _ => false,
    }
}

fn find_in_array(arr: Option<&Value>, id: &str) -> Option<Value> {
    arr?.as_array()?
        .iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
        .cloned()
}
