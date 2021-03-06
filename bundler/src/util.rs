use std::{hash::Hash, mem::replace};
use swc_atoms::js_word;
use swc_common::{Span, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut};

/// Helper for migration from [Fold] to [VisitMut]
pub(crate) trait MapWithMut: Sized {
    fn dummy() -> Self;

    fn take(&mut self) -> Self {
        replace(self, Self::dummy())
    }

    #[inline]
    fn map_with_mut<F>(&mut self, op: F)
    where
        F: FnOnce(Self) -> Self,
    {
        let dummy = Self::dummy();
        let v = replace(self, dummy);
        let v = op(v);
        let _dummy = replace(self, v);
    }
}

impl MapWithMut for ModuleItem {
    #[inline(always)]
    fn dummy() -> Self {
        ModuleItem::Stmt(Stmt::Empty(EmptyStmt { span: DUMMY_SP }))
    }
}

impl MapWithMut for Stmt {
    #[inline(always)]
    fn dummy() -> Self {
        Stmt::Empty(EmptyStmt { span: DUMMY_SP })
    }
}

impl MapWithMut for Expr {
    #[inline(always)]
    fn dummy() -> Self {
        Expr::Invalid(Invalid { span: DUMMY_SP })
    }
}

impl MapWithMut for Pat {
    #[inline(always)]
    fn dummy() -> Self {
        Pat::Invalid(Invalid { span: DUMMY_SP })
    }
}

impl<T> MapWithMut for Option<T> {
    #[inline(always)]
    fn dummy() -> Self {
        None
    }
}

impl<T> MapWithMut for Vec<T> {
    #[inline(always)]
    fn dummy() -> Self {
        Vec::new()
    }
}

impl<T> MapWithMut for Box<T>
where
    T: MapWithMut,
{
    #[inline(always)]
    fn dummy() -> Self {
        Box::new(T::dummy())
    }
}

impl MapWithMut for Ident {
    fn dummy() -> Self {
        Ident::new(js_word!(""), DUMMY_SP)
    }
}

impl MapWithMut for ObjectPatProp {
    fn dummy() -> Self {
        ObjectPatProp::Assign(AssignPatProp {
            span: DUMMY_SP,
            key: Ident::dummy(),
            value: None,
        })
    }
}

impl MapWithMut for PatOrExpr {
    fn dummy() -> Self {
        PatOrExpr::Pat(Box::new(Pat::Ident(Ident::dummy())))
    }
}

#[derive(Debug)]
pub(crate) struct CHashSet<V>
where
    V: Eq + Hash,
{
    inner: CloneMap<V, ()>,
}

impl<V> CHashSet<V>
where
    V: Eq + Hash,
{
    pub fn insert(&self, v: V) -> bool {
        self.inner.insert(v, ()).is_none()
    }
}

impl<V> Default for CHashSet<V>
where
    V: Eq + Hash,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CloneMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    #[cfg(feature = "concurrent")]
    inner: dashmap::DashMap<K, V>,
    #[cfg(not(feature = "concurrent"))]
    inner: std::cell::RefCell<std::collections::HashMap<K, V>>,
}

impl<K, V> Default for CloneMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<K, V> CloneMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    #[cfg(feature = "concurrent")]
    pub fn get(&self, k: &K) -> Option<V> {
        if let Some(v) = self.inner.get(k) {
            Some(v.value().clone())
        } else {
            None
        }
    }

    #[cfg(not(feature = "concurrent"))]
    pub fn get(&self, k: &K) -> Option<V> {
        if let Some(v) = self.inner.borrow().get(k) {
            Some(v.clone())
        } else {
            None
        }
    }

    #[cfg(feature = "concurrent")]
    pub fn insert(&self, k: K, v: V) -> Option<V> {
        self.inner.insert(k, v)
    }

    #[cfg(not(feature = "concurrent"))]
    pub fn insert(&self, k: K, v: V) -> Option<V> {
        self.inner.borrow_mut().insert(k, v)
    }
}

pub(crate) struct HygieneRemover;

impl VisitMut for HygieneRemover {
    noop_visit_mut_type!();

    fn visit_mut_span(&mut self, s: &mut Span) {
        *s = s.with_ctxt(SyntaxContext::empty())
    }
}

#[cfg(feature = "rayon")]
pub(crate) use rayon::join;

#[cfg(not(feature = "rayon"))]
pub(crate) fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA,
    B: FnOnce() -> RB,
{
    (oper_a(), oper_b())
}

#[cfg(feature = "rayon")]
pub(crate) use rayon::iter::IntoParallelIterator;

/// Fake trait
#[cfg(not(feature = "rayon"))]
pub(crate) trait IntoParallelIterator: Sized + IntoIterator {
    fn into_par_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

#[cfg(not(feature = "rayon"))]
impl<T> IntoParallelIterator for T where T: IntoIterator {}
