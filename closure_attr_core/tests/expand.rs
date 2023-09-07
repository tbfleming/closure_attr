use closure_attr_core::with_closure;
use proc_macro2::TokenTree as TT;
use proc_macro2::{LineColumn, Spacing, TokenStream};
use quote::quote;

fn is_punct(token: Option<TT>, ch: char, spacing: Spacing) -> bool {
    let Some(token) = token else {
        return false;
    };
    if let TT::Punct(punct) = token {
        punct.as_char() == ch && punct.spacing() == spacing
    } else {
        false
    }
}

fn is_ident(token: Option<TT>, s: &str) -> bool {
    let Some(token) = token else {
        return false;
    };
    if let TT::Ident(ident) = token {
        ident.to_string() == s
    } else {
        false
    }
}

// Add span info to `compiler_error!`
fn annotate_errors(stream: TokenStream) -> String {
    let mut out = quote! {};
    let mut it = stream.clone().into_iter();
    loop {
        let found = (|| {
            let mut it2 = it.clone();
            if !is_punct(it2.next(), ':', Spacing::Joint)
                || !is_punct(it2.next(), ':', Spacing::Alone)
                || !is_ident(it2.next(), "core")
                || !is_punct(it2.next(), ':', Spacing::Joint)
                || !is_punct(it2.next(), ':', Spacing::Alone)
                || !is_ident(it2.next(), "compile_error")
                || !is_punct(it2.next(), '!', Spacing::Alone)
            {
                return false;
            }
            let Some(tt) = it2.next() else {
                return false;
            };
            let TT::Group(g) = tt else {
                return false;
            };
            let mut it3 = g.stream().into_iter();
            let Some(TT::Literal(lit)) = it3.next() else {
                return false;
            };
            let LineColumn {
                line: l1,
                column: c1,
            } = lit.span().start();
            let LineColumn {
                line: l2,
                column: c2,
            } = lit.span().end();
            out.extend(quote! {compile_error!{(#l1, #c1), (#l2, #c2), #lit}});
            it = it2;
            return true;
        })();
        if !found {
            let Some(tok) = it.next() else { break };
            out.extend(quote! {#tok});
        }
    }
    out.to_string()
}

#[test]
fn errors() {
    assert_eq!(
        annotate_errors(with_closure(
            quote! {foo},
            quote! {
                fn x() {}
            }
        )),
        quote! {
            compile_error!{ (1usize,0usize), (1usize,0usize), "with_closure attribute takes no arguments" }
            fn x() {}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure]||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,16usize), (2usize,26usize), "closure attribute must have arguments" }
            fn f() {| |();}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(7)]||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,26usize), (2usize,27usize), "expected clone, clone mut, ref, ref mut, move, move mut, or weak (1)" }
            fn f() {| |();}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(x)]||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,26usize), (2usize,27usize), "expected clone, clone mut, ref, ref mut, move, move mut, or weak (2)" }
            fn f() {| |();}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(mut)]||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,26usize), (2usize,29usize), "expected clone, clone mut, ref, ref mut, move, move mut, or weak (2)" }
            fn f() {| |();}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(weak mut x)]||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,26usize), (2usize,30usize), "expected clone, clone mut, ref, ref mut, move, move mut, or weak (2)" }
            fn f() {| |();}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone mut let)] move ||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,36usize), (2usize,39usize), "expected identifier, found keyword `let`" }
            fn f() {move | |();}
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone mut x)] ||();
            }"#.parse().unwrap()
        )),
        quote! {
            compile_error!{ (2usize,40usize), (2usize,44usize), "closure must be declared with `move`" }
            fn f() {
                {
                    let mut x = x.clone();
                    | | {#[allow(unreachable_code)]loop{break;let _ = &x;} ()}
                };
            }
        }
        .to_string()
    );

    assert_eq!(
        annotate_errors(with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone mut x y)] move ||();
            }"#
            .parse()
            .unwrap()
        )),
        quote! {
            compile_error!{ (2usize,38usize), (2usize,39usize), "expected `,`" }
            fn f() {move | |();}
        }
        .to_string()
    );
}

#[test]
fn no_change() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure()] move ||();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            move | | ();
        }}
        .to_string()
    );
}

#[test]
fn clone() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone c)] move ||();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let c = c.clone();
                move | | {#[allow(unreachable_code)]loop{break; let _=&c;} ()}
            };
        }}
        .to_string()
    );
}

#[test]
fn fn_in_mod() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"mod m{ fn f() {
                #[closure(clone c)] move ||();
            }}"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {mod m{ fn f() {
            {
                let c = c.clone();
                move | | {#[allow(unreachable_code)]loop{break; let _=&c;} ()}
            };
        }}}
        .to_string()
    );
}

#[test]
fn closure_in_var() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                let clos = #[closure(clone c)] move ||();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            let clos = {
                let c = c.clone();
                move | | {#[allow(unreachable_code)]loop{break; let _=&c;} ()}
            };
        }}
        .to_string()
    );
}

#[test]
fn closure_in_call() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                callit(#[closure(clone c)] move ||());
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            callit({
                let c = c.clone();
                move | | {#[allow(unreachable_code)]loop{break; let _=&c;} ()}
            });
        }}
        .to_string()
    );
}

#[test]
fn immediate_call() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                (#[closure(clone c)] move ||())();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            ({
                let c = c.clone();
                move | | {#[allow(unreachable_code)]loop{break; let _=&c;} ()}
            })();
        }}
        .to_string()
    );
}

#[test]
fn all_but_weak() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone c, clone mut cm, ref r, ref mut rm, move m, move mut mm)] move ||();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let c = c.clone();
                let mut cm = cm.clone();
                let r = &r;
                let rm = &mut rm;
                let m = m;
                let mut mm = mm;
                move | | {
                    #[allow(unreachable_code)]
                    loop {
                        break;
                        let _ = &c;
                        let _ = &cm;
                        let _ = &r;
                        let _ = &rm;
                        let _ = &m;
                        let _ = &mm;
                    }
                    ()
                }
            };
        }}
        .to_string()
    );
}

#[test]
fn all_but_weak_with_args() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone c, clone mut cm, ref r, ref mut rm, move m, move mut mm)] move |a, b:i32, mut c|();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let c = c.clone();
                let mut cm = cm.clone();
                let r = &r;
                let rm = &mut rm;
                let m = m;
                let mut mm = mm;
                move |a, b:i32, mut c| {
                    #[allow(unreachable_code)]
                    loop {
                        break;
                        let _ = &c;
                        let _ = &cm;
                        let _ = &r;
                        let _ = &rm;
                        let _ = &m;
                        let _ = &mm;
                    }
                    ()
                }
            };
        }}
        .to_string()
    );
}

#[test]
fn all_but_weak_with_ret() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone c, clone mut cm, ref r, ref mut rm, move m, move mut mm)] move |a, b:i32, mut c| {return 7;};
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let c = c.clone();
                let mut cm = cm.clone();
                let r = &r;
                let rm = &mut rm;
                let m = m;
                let mut mm = mm;
                move |a, b:i32, mut c| {
                    #[allow(unreachable_code)]
                    loop {
                        break;
                        let _ = &c;
                        let _ = &cm;
                        let _ = &r;
                        let _ = &rm;
                        let _ = &m;
                        let _ = &mm;
                    }
                    { return 7; }
                }
            };
        }}
        .to_string()
    );
}

#[test]
fn weak() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(weak r, weak a)] move ||();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let r = ::closure_attr::Downgrade::downgrade(&r);
                let a = ::closure_attr::Downgrade::downgrade(&a);
                move | |
                    (|| {
                        let r = r.upgrade()?;
                        let a = a.upgrade()?;
                        Some((||())())
                    })()
                    .unwrap_or_default()
            };
        }}
        .to_string()
    );
}

#[test]
fn weak_with_args() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(weak r, weak a)] move |a, b:i32, mut c|();
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let r = ::closure_attr::Downgrade::downgrade(&r);
                let a = ::closure_attr::Downgrade::downgrade(&a);
                move |a, b:i32, mut c|
                    (|| {
                        let r = r.upgrade()?;
                        let a = a.upgrade()?;
                        Some((||())())
                    })()
                    .unwrap_or_default()
            };
        }}
        .to_string()
    );
}

#[test]
fn weak_with_ret() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(weak r, weak a)] move |a, b:i32, mut c| {return 7;};
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let r = ::closure_attr::Downgrade::downgrade(&r);
                let a = ::closure_attr::Downgrade::downgrade(&a);
                move |a, b:i32, mut c|
                    (|| {
                        let r = r.upgrade()?;
                        let a = a.upgrade()?;
                        Some((||
                            { return 7; }
                        )())
                    })()
                    .unwrap_or_default()
            };
        }}
        .to_string()
    );
}

#[test]
fn embedded_closure() {
    assert_eq!(
        with_closure(
            quote! {},
            r#"fn f() {
                #[closure(clone i)]
                move || {
                    let inner = #[closure(clone i)]
                    move || {
                        return *i;
                    };
                    (inner, i)
                };
            }"#
            .parse()
            .unwrap()
        )
        .to_string(),
        quote! {fn f() {
            {
                let i = i.clone();
                move | | {
                    #[allow(unreachable_code)]
                    loop {
                        break;
                        let _ = &i;
                    }
                    {
                        let inner = {
                            let i = i.clone();
                            move | | {
                                #[allow(unreachable_code)]
                                loop {
                                    break;
                                    let _ = &i;
                                }
                                {
                                    return *i;
                                }
                            }
                        };
                        (inner, i)
                    }
                }
            };
        }}
        .to_string()
    );
}
