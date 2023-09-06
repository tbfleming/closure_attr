use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::mem::take;
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
            Err(_) => Err(Error::new(
                span, // The span in e and currently in input are garbled.
                "expected clone, clone mut, ref, ref mut, rcweak, or arcweak",
            ))?,
        };
        let mut ty = ty.to_string();
        if input.lookahead1().peek(Token![mut]) {
            input.parse::<Token![mut]>()?;
            ty += " mut";
        }
        let parse_ident = || {
            match Ident::parse_any(input) {
                Ok(v) => Ok(v),
                Err(_) => Err(Error::new(
                    span, // The span in e and currently in input are garbled.
                    format!("expected identifier after {}", ty),
                )),
            }
        };
        match ty.as_str() {
            "clone" => Ok(Capture::Clone(parse_ident()?)),
            "clone mut" => Ok(Capture::CloneMut(parse_ident()?)),
            "ref" => Ok(Capture::Ref(parse_ident()?)),
            "ref mut" => Ok(Capture::RefMut(parse_ident()?)),
            "rcweak" => Ok(Capture::RcWeak(parse_ident()?)),
            "arcweak" => Ok(Capture::ArcWeak(parse_ident()?)),
            _ => Err(Error::new(
                span,
                "expected clone, clone mut, ref, ref mut, rcweak, or arcweak",
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
        let Expr::Closure(closure) = expr else {
            return syn::visit_mut::visit_expr_mut(self, expr);
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
                                    let e1 = take(self.errors);
                                    let e2 = Error::new(
                                        a.span(),
                                        "closure attribute must have arguments",
                                    )
                                    .to_compile_error();
                                    *self.errors = quote! {#e1 #e2};
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
                                            let e1 = take(self.errors);
                                            let e2 = e.to_compile_error();
                                            *self.errors = quote! {#e1 #e2};
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
            let e1 = take(self.errors);
            let e2 = Error::new(closure.span(), "closure must be declared with `move`")
                .to_compile_error();
            *self.errors = quote! {#e1 #e2};
        }

        let mut locals = quote! {};
        let mut upgrade = quote! {};
        for cap in captures {
            match cap {
                Capture::Clone(ident) => locals = quote! {#locals let #ident = #ident.clone();},
                Capture::CloneMut(ident) => {
                    locals = quote! {#locals let mut #ident = #ident.clone();}
                }
                Capture::Ref(ident) => locals = quote! {#locals let #ident = &#ident;},
                Capture::RefMut(ident) => locals = quote! {#locals let #ident = &mut #ident;},
                Capture::RcWeak(ident) => {
                    locals = quote! {#locals let #ident = ::std::rc::Rc::downgrade(&#ident);};
                    upgrade = quote! {#upgrade let #ident = #ident.upgrade()?;};
                }
                Capture::ArcWeak(ident) => {
                    locals = quote! {#locals let #ident = ::std::sync::Arc::downgrade(&#ident);};
                    upgrade = quote! {#upgrade let #ident = #ident.upgrade()?;};
                }
            }
        }
        if !upgrade.is_empty() {
            let body = closure.body.clone();
            closure.body = Box::new(Expr::Verbatim(quote! {
                (|| {
                    #upgrade
                    Some(#body)
                })().unwrap_or_default()
            }));
        }
        *expr = Expr::Verbatim(quote! {
            {
                #locals
                #closure
            }
        });
    }
}

pub fn with_closure(attr: TokenStream2, item: TokenStream2) -> TokenStream2 {
    let mut errors = quote! {};
    if !attr.is_empty() {
        let e = Error::new(
            proc_macro2::Span::call_site(),
            "with_closure attribute takes no arguments",
        )
        .to_compile_error();
        errors = quote! {#errors #e};
    }
    let mut item: syn::Item = syn::parse2(item).unwrap();
    let mut visitor = Visitor {
        errors: &mut errors,
    };
    visitor.visit_item_mut(&mut item);
    quote! {#errors #item}
}
