//! Wrappers are lightweight proxies for references to R datatypes.
//! They do not contain an Robj (see array.rs for an example of this).

use crate::*;
#[doc(hidden)]
use libR_sys::*;
#[doc(hidden)]
use std::collections::HashMap;

/// Wrapper for creating symbols.
///
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let symbol = r!(Symbol("xyz"));
/// assert_eq!(symbol.as_symbol(), Some(Symbol("xyz")));
/// assert!(symbol.is_symbol());
/// ```
/// Note that creating a symbol from a string is expensive
/// and so you may want to cache them.
///
#[derive(Debug, PartialEq, Clone)]
pub struct Symbol<'a>(pub &'a str);

/// Wrapper for creating character objects.
/// These are used only as the contents of a character
/// vector.
///
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let chr = r!(Character("xyz"));
/// assert_eq!(chr.as_character(), Some(Character("xyz")));
/// ```
///
#[derive(Debug, PartialEq, Clone)]
pub struct Character<'a>(pub &'a str);

/// Wrapper for creating raw (byte) objects.
///
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let bytes = r!(Raw(&[1, 2, 3]));
/// assert_eq!(bytes.len(), 3);
/// assert_eq!(bytes.as_raw(), Some(Raw(&[1, 2, 3])));
/// ```
///
#[derive(Debug, PartialEq, Clone)]
pub struct Raw<'a>(pub &'a [u8]);

/// Wrapper for creating language objects.
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let call_to_xyz = r!(Lang(&[r!(Symbol("xyz")), r!(1), r!(2)]));
/// assert_eq!(call_to_xyz.is_language(), true);
/// assert_eq!(call_to_xyz.len(), 3);
/// assert_eq!(call_to_xyz.as_lang(), Some(Lang(vec![r!(Symbol("xyz")), r!(1), r!(2)])));
/// ```
///
/// Note: You can use the [lang!] macro for this.
#[derive(Debug, PartialEq, Clone)]
pub struct Lang<T>(pub T);

/// Wrapper for creating pair list (LISTSXP) objects.
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let hashmap : std::collections::HashMap<_, _> = (0..100)
///     .map(|i| (Some(format!("n{}", i)), r!(i))).collect();
/// let pairlist = Pairlist{names_and_values: hashmap};
/// let expr = r!(pairlist);
/// assert_eq!(expr.len(), 100);
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Pairlist<NV> {
    pub names_and_values: NV,
}

/// Wrapper for creating list (VECSXP) objects.
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let list = r!(List(&[r!(0), r!(1), r!(2)]));
/// assert_eq!(list.is_list(), true);
/// assert_eq!(list.as_list(), Some(List(vec![r!(0), r!(1), r!(2)])));
/// assert_eq!(format!("{:?}", list), r#"r!(List([r!(0), r!(1), r!(2)]))"#);
/// ```
///
/// Note: you can use the [list!] macro for named lists.
#[derive(Debug, PartialEq, Clone)]
pub struct List<T>(pub T);

/// Wrapper for creating expression objects.
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let expr = r!(Expr(&[r!(1.), r!("xyz")]));
/// assert_eq!(expr.len(), 2);
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Expr<T>(pub T);

/// Wrapper for creating environments.
#[derive(Debug, PartialEq, Clone)]
pub struct Env<P, NV> {
    pub parent: P,
    pub names_and_values: NV,
}

/// Wrapper for creating functions (CLOSSXP).
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let expr = R!(function(a = 1, b) {c <- a + b}).unwrap();
/// let func = expr.as_func().unwrap();
///
/// let expected_formals = Pairlist {
///     names_and_values: vec![(Some("a"), r!(1.0)), (Some("b"), missing_arg())] };
/// let expected_body = lang!(
///     "{", lang!("<-", sym!(c), lang!("+", sym!(a), sym!(b))));
/// assert_eq!(func.formals.as_pairlist().unwrap(), expected_formals);
/// assert_eq!(func.body, expected_body);
/// assert_eq!(func.env, global_env());
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Func<F, B, E> {
    pub formals: F,
    pub body: B,
    pub env: E,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Promise<C, E, V> {
    pub code: C,
    pub env: E,
    pub value: V,
    pub seen: bool
}

/// Wrapper for creating and reading Primitive functions.
///
/// ```
/// use extendr_api::*;
/// extendr_engine::start_r();
/// let robj = r!(Primitive("+"));
/// assert!(robj.is_primitive());
/// assert!(!r!(Primitive("not_a_primitive")).is_primitive());
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Primitive<'a>(pub &'a str);

impl<T> From<List<T>> for Robj
where
    T: IntoIterator,
    T::IntoIter: ExactSizeIterator,
    T::Item: Into<Robj>,
{
    /// Make a list object from an array of Robjs.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let list_of_ints = r!(List(&[1, 2]));
    /// assert_eq!(list_of_ints.len(), 2);
    /// ```
    fn from(val: List<T>) -> Self {
        make_vector(VECSXP, val.0)
    }
}

impl<T> From<Expr<T>> for Robj
where
    T: IntoIterator,
    T::IntoIter: ExactSizeIterator,
    T::Item: Into<Robj>,
{
    /// Make an expression object from an array of Robjs.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let list_of_ints = r!(Expr(&[1, 2]));
    /// assert_eq!(list_of_ints.len(), 2);
    /// ```
    fn from(val: Expr<T>) -> Self {
        make_vector(EXPRSXP, val.0)
    }
}

impl<'a> From<Raw<'a>> for Robj {
    /// Make a raw object from bytes.
    fn from(val: Raw<'a>) -> Self {
        single_threaded(|| unsafe {
            let val = val.0;
            let sexp = Rf_allocVector(RAWSXP, val.len() as R_xlen_t);
            R_PreserveObject(sexp);
            let ptr = RAW(sexp);
            for (i, &v) in val.iter().enumerate() {
                *ptr.offset(i as isize) = v;
            }
            Robj::Owned(sexp)
        })
    }
}

impl<'a> From<Symbol<'a>> for Robj {
    /// Make a symbol object.
    fn from(name: Symbol) -> Self {
        single_threaded(|| unsafe { new_owned(make_symbol(name.0)) })
    }
}

impl<'a> From<Primitive<'a>> for Robj {
    /// Make a primitive object, or NULL if not available.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let builtin = r!(Primitive("+"));
    /// let special = r!(Primitive("if"));
    /// ```
    fn from(name: Primitive) -> Self {
        single_threaded(|| unsafe {
            let sym = make_symbol(name.0);
            let symvalue = new_sys(SYMVALUE(sym));
            if symvalue.is_primitive() {
                symvalue
            } else {
                r!(NULL)
            }
        })
    }
}

impl<T> From<Lang<T>> for Robj
where
    T: IntoIterator,
    T::IntoIter: DoubleEndedIterator,
    T::Item: Into<Robj>,
{
    /// Convert a wrapper to an R language object.
    fn from(val: Lang<T>) -> Self {
        single_threaded(|| unsafe {
            let mut res = R_NilValue;
            let mut num_protected = 0;
            for val in val.0.into_iter().rev() {
                let val = Rf_protect(val.into().get());
                res = Rf_protect(Rf_lcons(val, res));
                num_protected += 2;
            }
            let res = new_owned(res);
            Rf_unprotect(num_protected);
            res
        })
    }
}

impl<'a, P, NV> From<Env<P, NV>> for Robj
where
    P: Into<Robj>,
    NV: IntoIterator + 'a,
    NV::Item: Into<(String, Robj)>,
{
    /// Convert a wrapper to an R environment object.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let hashmap : std::collections::HashMap<_, _> = (0..100)
    ///     .map(|i| (format!("n{}", i), r!(i))).collect();
    /// let env = Env{parent: global_env(), names_and_values: hashmap};
    /// let expr = r!(env);
    /// assert_eq!(expr.len(), 100);
    /// ```
    fn from(val: Env<P, NV>) -> Self {
        single_threaded(|| {
            let (parent, names_and_values) = (val.parent, val.names_and_values);
            let dict_len = 29;
            let res = call!("new.env", TRUE, parent.into(), dict_len).unwrap();
            for nv in names_and_values {
                let (n, v) = nv.into();
                unsafe { Rf_defineVar(r!(Symbol(n.as_str())).get(), v.get(), res.get()) }
            }
            res
        })
    }
}

impl<'a, NV> From<Pairlist<NV>> for Robj
where
    NV: IntoIterator + 'a,
    NV::Item: Into<(Option<String>, Robj)>,
{
    /// Convert a wrapper to a LISTSXP object.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let hashmap : std::collections::HashMap<_, _> = (0..100)
    ///     .map(|i| (Some(format!("n{}", i)), r!(i))).collect();
    /// let pairlist = Pairlist{names_and_values: hashmap};
    /// let expr = r!(pairlist);
    /// assert_eq!(expr.len(), 100);
    /// ```
    fn from(val: Pairlist<NV>) -> Self {
        single_threaded(|| unsafe {
            let names_and_values = val.names_and_values;
            let mut num_protects = 0;
            let mut res = R_NilValue;
            let names_and_values: Vec<_> = names_and_values.into_iter().collect();
            for nv in names_and_values.into_iter().rev() {
                let (name, val) = nv.into();
                let val = Rf_protect(val.get());
                res = Rf_protect(Rf_cons(val, res));
                num_protects += 2;
                if let Some(name) = name {
                    let name = r!(Symbol(name.as_str())).get();
                    SET_TAG(res, name);
                }
            }
            let res = new_owned(res);
            Rf_unprotect(num_protects as i32);
            res
        })
    }
}

fn make_symbol(name: &str) -> SEXP {
    let mut bytes = Vec::with_capacity(name.len() + 1);
    bytes.extend(name.bytes());
    bytes.push(0);
    unsafe { Rf_install(bytes.as_ptr() as *const i8) }
}

fn make_vector<T>(sexptype: u32, values: T) -> Robj
where
    T: IntoIterator,
    T::IntoIter: ExactSizeIterator,
    T::Item: Into<Robj>,
{
    single_threaded(|| unsafe {
        let values = values.into_iter();
        let sexp = Rf_allocVector(sexptype, values.len() as R_xlen_t);
        R_PreserveObject(sexp);
        for (i, val) in values.enumerate() {
            SET_VECTOR_ELT(sexp, i as R_xlen_t, val.into().get());
        }
        Robj::Owned(sexp)
    })
}

/// Allow you to skip the Symbol() in some cases.
impl<'a> From<&'a str> for Symbol<'a> {
    fn from(val: &'a str) -> Self {
        Self(val)
    }
}

impl Robj {
    /// Convert a symbol object to a Symbol wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let fred = sym!(fred);
    /// assert_eq!(fred.as_symbol(), Some(Symbol("fred")));
    /// ```
    pub fn as_symbol(&self) -> Option<Symbol> {
        if self.is_symbol() {
            unsafe {
                let printname = PRINTNAME(self.get());
                if TYPEOF(printname) as u32 == CHARSXP {
                    Some(Symbol(
                        to_str(R_CHAR(printname) as *const u8)
                    ))
                } else {
                    Some(Symbol(
                        "bad symbol"
                    ))
                }
            }
        } else {
            None
        }
    }

    /// Convert a character object to a Character wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let fred = r!(Character("fred"));
    /// assert_eq!(fred.as_character(), Some(Character("fred")));
    /// ```
    pub fn as_character(&self) -> Option<Character> {
        if self.sexptype() == CHARSXP {
            Some(Character(unsafe {
                to_str(R_CHAR(self.get()) as *const u8)
            }))
        } else {
            None
        }
    }

    /// Convert a raw object to a Character wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let bytes = r!(Raw(&[1, 2, 3]));
    /// assert_eq!(bytes.len(), 3);
    /// assert_eq!(bytes.as_raw(), Some(Raw(&[1, 2, 3])));
    /// ```
    pub fn as_raw(&self) -> Option<Raw> {
        if self.sexptype() == RAWSXP {
            Some(Raw(self.as_raw_slice().unwrap()))
        } else {
            None
        }
    }
    /// Convert a language object to a Lang wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let call_to_xyz = r!(Lang(&[r!(Symbol("xyz")), r!(1), r!(2)]));
    /// assert_eq!(call_to_xyz.is_language(), true);
    /// assert_eq!(call_to_xyz.len(), 3);
    /// assert_eq!(call_to_xyz.as_lang(), Some(Lang(vec![r!(Symbol("xyz")), r!(1), r!(2)])));
    /// assert_eq!(format!("{:?}", call_to_xyz), r#"r!(Lang([sym!(xyz), r!(1), r!(2)]))"#);
    /// ```
    pub fn as_lang(&self) -> Option<Lang<Vec<Robj>>> {
        if self.sexptype() == LANGSXP {
            let res: Vec<_> = self
                .as_pairlist_iter()
                .unwrap()
                .map(|robj| robj.to_owned())
                .collect();
            Some(Lang(res))
        } else {
            None
        }
    }

    /// Convert a pair list object (LISTSXP) to a Pairlist wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let names_and_values = vec![(Some("a".to_string()), r!(1)), (Some("b".to_string()), r!(2)), (None, r!(3))];
    /// let pairlist1 = Pairlist{ names_and_values };
    /// let names_and_values = vec![(Some("a"), r!(1)), (Some("b"), r!(2)), (None, r!(3))];
    /// let pairlist2 = Pairlist{ names_and_values };
    /// let robj = r!(pairlist1);
    /// assert_eq!(robj.as_pairlist().unwrap(), pairlist2);
    /// ```
    pub fn as_pairlist(&self) -> Option<Pairlist<Vec<(Option<&str>, Robj)>>> {
        if self.sexptype() == LISTSXP {
            let names = self.as_pairlist_tag_iter().unwrap();
            let values = self.as_pairlist_iter().unwrap();
            let names_and_values: Vec<_> = names.zip(values).collect();
            Some(Pairlist { names_and_values })
        } else {
            None
        }
    }

    /// Convert a list object (VECSXP) to a List wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let list = r!(List(&[r!(0), r!(1), r!(2)]));
    /// assert_eq!(list.is_list(), true);
    /// assert_eq!(list.as_list(), Some(List(vec![r!(0), r!(1), r!(2)])));
    /// assert_eq!(format!("{:?}", list), r#"r!(List([r!(0), r!(1), r!(2)]))"#);
    /// ```
    pub fn as_list(&self) -> Option<List<Vec<Robj>>> {
        if self.sexptype() == VECSXP {
            let res: Vec<_> = self
                .as_list_iter()
                .unwrap()
                .map(|robj| robj.to_owned())
                .collect();
            Some(List(res))
        } else {
            None
        }
    }

    /// Convert an expression object (EXPRSXP) to a Expr wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let expr = r!(Expr(&[r!(0), r!(1), r!(2)]));
    /// assert_eq!(expr.is_expr(), true);
    /// assert_eq!(expr.as_expr(), Some(Expr(vec![r!(0), r!(1), r!(2)])));
    /// assert_eq!(format!("{:?}", expr), r#"r!(Expr([r!(0), r!(1), r!(2)]))"#);
    /// ```
    pub fn as_expr(&self) -> Option<Expr<Vec<Robj>>> {
        if self.sexptype() == EXPRSXP {
            let res: Vec<_> = self
                .as_list_iter()
                .unwrap()
                .map(|robj| robj.to_owned())
                .collect();
            Some(Expr(res))
        } else {
            None
        }
    }

    /// Convert an environment object (ENVSXP) to a Env wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let names_and_values : std::collections::HashMap<_, _> = (0..100).map(|i| (format!("n{}", i), r!(i))).collect();
    /// let env = Env{parent: global_env(), names_and_values};
    /// let expr = r!(env.clone());
    /// assert_eq!(expr.len(), 100);
    /// let env2 = expr.as_environment().unwrap();
    /// assert_eq!(env2.names_and_values.len(), 100);
    /// ```
    pub fn as_environment(&self) -> Option<Env<Robj, HashMap<&str, Robj>>> {
        if self.sexptype() == ENVSXP {
            unsafe {
                let parent = new_owned(ENCLOS(self.get()));
                let hashtab = new_owned(HASHTAB(self.get()));
                let frame = new_owned(FRAME(self.get()));
                let mut names_and_values = HashMap::new();
                if let Some(as_list_iter) = hashtab.as_list_iter() {
                    for frame in as_list_iter {
                        if let (Some(obj_iter), Some(tag_iter)) =
                            (frame.as_pairlist_iter(), frame.as_pairlist_tag_iter())
                        {
                            for (obj, tag) in obj_iter.zip(tag_iter) {
                                if !obj.is_unbound_value() && tag.is_some() {
                                    names_and_values.insert(tag.unwrap(), obj);
                                }
                            }
                        }
                    }
                } else if let (Some(obj_iter), Some(tag_iter)) =
                    (frame.as_pairlist_iter(), frame.as_pairlist_tag_iter())
                {
                    for (obj, tag) in obj_iter.zip(tag_iter) {
                        if !obj.is_unbound_value() && tag.is_some() {
                            names_and_values.insert(tag.unwrap(), obj);
                        }
                    }
                }
                Some(Env {
                    parent,
                    names_and_values,
                })
            }
        } else {
            None
        }
    }
    /// Convert a function object (CLOSXP) to a Func wrapper.
    /// ```
    /// use extendr_api::*;
    /// extendr_engine::start_r();
    /// let func = R!(function(a,b) a + b).unwrap();
    /// println!("{:?}", func.as_func());
    /// ```
    pub fn as_func(&self) -> Option<Func<Robj, Robj, Robj>> {
        if self.sexptype() == CLOSXP {
            unsafe {
                let sexp = self.get();
                let formals = new_owned(FORMALS(sexp));
                let body = new_owned(BODY(sexp));
                let env = new_owned(CLOENV(sexp));
                Some(Func { formals, body, env })
            }
        } else {
            None
        }
    }

    // /// Convert a primitive object (BUILTINSXP or SPECIALSXP) to a wrapper.
    // /// ```
    // /// use extendr_api::*;
    // /// extendr_engine::start_r();
    // /// let builtin = r!(Primitive("+"));
    // /// let special = r!(Primitive("if"));
    // /// assert_eq!(builtin.sexptype(), libR_sys::BUILTINSXP);
    // /// assert_eq!(special.sexptype(), libR_sys::SPECIALSXP);
    // /// ```
    // pub fn as_primitive(&self) -> Option<Primitive> {
    //     match self.sexptype() {
    //         BUILTINSXP | SPECIALSXP => {
    //             // Unfortunately, for now PRIMNAME is out of bounds.
    //             //Some(Primitive(unsafe {to_str(PRIMNAME(self.get()) as * const u8)}))
    //             None
    //         }
    //         _ => None,
    //     }
    // }

    /// Get a wrapper for a promise.
    pub fn as_promise(&self) -> Option<Promise<Robj, Robj, Robj>> {
        if self.is_promise() {
            unsafe {
                let sexp = self.get();
                Some(Promise {
                    code: new_owned(PRCODE(sexp)),
                    env: new_owned(PRENV(sexp)),
                    value: new_owned(PRVALUE(sexp)),
                    seen: PRSEEN(sexp) != 0
                })
            }
        } else {
            None
        }
    }
}
