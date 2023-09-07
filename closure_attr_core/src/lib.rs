#![doc = include_str!("../README.md")]

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    visit_mut::VisitMut,
    AttrStyle, Error, Expr, Ident, Meta, Token,
};

enum Capture {
    Clone(Ident),
    CloneMut(Ident),
    Ref(Ident),
    RefMut(Ident),
    RcWeak(Ident),
    ArcWeak(Ident),
}

impl Parse for Capture {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let ty = match Ident::parse_any(input) {
            Ok(v) => v,
            Err(e) => Err(Error::new(
                e.span(),
                // The (1) and (2) tags aid testing and debugging.
                "expected clone, clone mut, ref, ref mut, rcweak, or arcweak (1)",
            ))?,
        };
        let mut ty = ty.to_string();
        if input.lookahead1().peek(Token![mut]) {
            input.parse::<Token![mut]>()?;
            ty += " mut";
        }
        match ty.as_str() {
            "clone" => Ok(Capture::Clone(Ident::parse(input)?)),
            "clone mut" => Ok(Capture::CloneMut(Ident::parse(input)?)),
            "ref" => Ok(Capture::Ref(Ident::parse(input)?)),
            "ref mut" => Ok(Capture::RefMut(Ident::parse(input)?)),
            "rcweak" => Ok(Capture::RcWeak(Ident::parse(input)?)),
            "arcweak" => Ok(Capture::ArcWeak(Ident::parse(input)?)),
            _ => Err(Error::new(
                span,
                "expected clone, clone mut, ref, ref mut, rcweak, or arcweak (2)",
            )),
        }
    }
}

struct Captures(Vec<Capture>);

impl Parse for Captures {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let v = input
            .parse_terminated(Capture::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();
        Ok(Captures(v))
    }
}

struct Visitor<'a> {
    errors: &'a mut TokenStream2,
}

impl<'a> VisitMut for Visitor<'a> {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        syn::visit_mut::visit_expr_mut(self, expr);

        let Expr::Closure(closure) = expr else {
            return;
        };

        let mut captures = Vec::new();
        closure.attrs = closure
            .attrs
            .drain(..)
            .filter(|a| {
                if let AttrStyle::Outer = a.style {
                    match &a.meta {
                        Meta::Path(p) => {
                            if let Some(ident) = p.get_ident() {
                                if ident == "closure" {
                                    self.errors.extend(
                                        Error::new(
                                            a.span(),
                                            "closure attribute must have arguments",
                                        )
                                        .to_compile_error(),
                                    );
                                    return false;
                                }
                            }
                        }
                        Meta::List(l) => {
                            if let Some(ident) = l.path.get_ident() {
                                if ident == "closure" {
                                    let mut ct = match syn::parse2::<Captures>(l.tokens.clone()) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            self.errors.extend(e.to_compile_error());
                                            return false;
                                        }
                                    };
                                    captures.append(&mut ct.0);
                                    return false;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                true
            })
            .collect();

        if captures.is_empty() {
            return;
        }
        if closure.capture.is_none() {
            self.errors.extend(
                Error::new(closure.span(), "closure must be declared with `move`")
                    .to_compile_error(),
            );
        }

        let span = closure.span();
        let mut locals = quote! {};
        let mut use_whole = quote! {};
        let mut upgrade = quote! {};
        for cap in captures {
            match cap {
                Capture::Clone(ident) => {
                    locals.extend(quote_spanned! {span=> let #ident = #ident.clone();});
                    use_whole.extend(quote_spanned! {span=> let _ = &#ident;});
                }
                Capture::CloneMut(ident) => {
                    locals.extend(quote_spanned! {span=> let mut #ident = #ident.clone();});
                    use_whole.extend(quote_spanned! {span=> let _ = &#ident;});
                }
                Capture::Ref(ident) => {
                    locals.extend(quote_spanned! {span=> let #ident = &#ident;});
                    use_whole.extend(quote_spanned! {span=> let _ = &#ident;});
                }
                Capture::RefMut(ident) => {
                    locals.extend(quote_spanned! {span=> let #ident = &mut #ident;});
                    use_whole.extend(quote_spanned! {span=> let _ = &#ident;});
                }
                Capture::RcWeak(ident) => {
                    locals.extend(
                        quote_spanned! {span=> let #ident = ::std::rc::Rc::downgrade(&#ident);},
                    );
                    use_whole.extend(quote_spanned! {span=> let _ = &#ident;});
                    upgrade.extend(quote_spanned! {span=> let #ident = #ident.upgrade()?;});
                }
                Capture::ArcWeak(ident) => {
                    locals.extend(
                        quote_spanned! {span=> let #ident = ::std::sync::Arc::downgrade(&#ident);},
                    );
                    use_whole.extend(quote_spanned! {span=> let _ = &#ident;});
                    upgrade.extend(quote_spanned! {span=> let #ident = #ident.upgrade()?;});
                }
            }
        }

        // Force capture of whole variables without preventing unused warnings.
        let body = closure.body.clone();
        closure.body = Box::new(Expr::Verbatim(quote_spanned! {span=>
            {
                #[allow(unreachable_code)]
                loop {
                    break;
                    #use_whole
                }
                #body
            }
        }));

        if !upgrade.is_empty() {
            let body = closure.body.clone();
            closure.body = Box::new(Expr::Verbatim(quote_spanned! {span=>
                (|| {
                    #upgrade
                    Some((||#body)())
                })().unwrap_or_default()
            }));
        }

        *expr = Expr::Verbatim(quote_spanned! {span=>
            {
                #locals
                #closure
            }
        });
    }
}

pub fn with_closure(attr: TokenStream2, item_tokens: TokenStream2) -> TokenStream2 {
    let mut errors = quote! {};
    if !attr.is_empty() {
        errors.extend(
            Error::new(
                proc_macro2::Span::call_site(),
                "with_closure attribute takes no arguments",
            )
            .to_compile_error(),
        );
    }
    let item = syn::parse2(item_tokens.clone());
    let mut item = match item {
        Ok(item) => item,
        Err(e) => {
            let e = e.to_compile_error();
            return quote! {#errors #e #item_tokens};
        }
    };
    let mut visitor = Visitor {
        errors: &mut errors,
    };
    visitor.visit_item_mut(&mut item);
    quote! {#errors #item}
}
