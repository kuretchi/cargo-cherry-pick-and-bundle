use anyhow::{bail, Result};
use itertools::Itertools as _;
use proc_macro2::Span;
use std::{
    borrow::Cow,
    fs, iter,
    path::{Path, PathBuf},
};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
    visit::{self, Visit},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SelectItemMod {
    All,
    Partial,
    None,
}

pub fn bundle(
    crate_name: &str,
    path: &Path,
    mut select_item_mod: impl FnMut(&str) -> SelectItemMod,
    mut select_item_use: impl FnMut(&str) -> bool,
) -> Result<String> {
    let content = do_bundle(
        false,
        0,
        &path.join("src").join("lib.rs"),
        &mut |_, item_mod| select_item_mod(&item_mod.ident.to_string()),
        &mut |content, item_use| select_item_use(&spanned_str(content, item_use.span())),
    )?;

    Ok(format!("pub mod {} {{\n{}}}\n", crate_name, content))
}

fn do_bundle(
    all_selected: bool,
    depth: usize,
    file_path: &Path,
    select_item_mod: &mut impl FnMut(&str, &syn::ItemMod) -> SelectItemMod,
    select_item_use: &mut impl FnMut(&str, &syn::ItemUse) -> bool,
) -> Result<String> {
    #[derive(Default)]
    struct Visitor<'ast> {
        item_uses: Vec<&'ast syn::ItemUse>,
        item_mods: Vec<&'ast syn::ItemMod>,
    }

    impl<'ast> Visit<'ast> for Visitor<'ast> {
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            self.item_uses.push(item);
        }

        fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
            self.item_mods.push(item);
        }
    }

    let content = fs::read_to_string(file_path)?;
    let content = preprocess(depth, &content)?;
    let file = syn::parse_file(&content)?;

    let mut visitor = Visitor::default();
    visitor.visit_file(&file);

    let mut span_and_new_strs = vec![];

    for item_use in visitor.item_uses {
        if !all_selected && !select_item_use(&content, item_use) {
            span_and_new_strs.push((item_use.span(), Cow::Borrowed("")));
        }
    }
    for item_mod in visitor.item_mods {
        let mut select = None;
        let all_selected = all_selected || {
            let s = select_item_mod(&content, item_mod);
            select = Some(s);
            s == SelectItemMod::All
        };
        if all_selected || select.unwrap() == SelectItemMod::Partial {
            let semi_span = match item_mod.semi {
                Some(semi) => semi.span(),
                None => bail!("only external-file modules are supported now"),
            };
            let file_path = mod_file_path(file_path, &item_mod.ident.to_string())?;
            let content = do_bundle(
                all_selected,
                depth + 1,
                &file_path,
                select_item_mod,
                select_item_use,
            )?;
            span_and_new_strs.push((semi_span, Cow::Owned(format!(" {{\n{}}}", content))));
        } else {
            span_and_new_strs.push((item_mod.span(), Cow::Borrowed("")));
        }
    }

    span_and_new_strs.sort_by_key(|(span, _)| {
        let start = span.start();
        (start.line, start.column)
    });

    Ok(replace_spanned_strs(&content, span_and_new_strs))
}

fn mod_file_path(file_path: &Path, mod_ident: &str) -> Result<PathBuf> {
    let dir_path = {
        let parent = file_path.parent().unwrap();
        let file_name = file_path.file_name().unwrap();
        if file_name == "lib.rs" || file_name == "mod.rs" {
            Cow::Borrowed(parent)
        } else {
            Cow::Owned(parent.join(file_path.file_stem().unwrap()))
        }
    };
    let path0 = dir_path.join(format!("{}.rs", mod_ident));
    let path1 = dir_path.join(mod_ident).join("mod.rs");
    let exists = |path| fs::metadata(path).map_or(false, |m| m.is_file());

    match (exists(&path0), exists(&path1)) {
        (true, false) => Ok(path0),
        (false, true) => Ok(path1),
        (false, false) => bail!(
            "neither '{ident}.rs' nor '{ident}/mod.rs' exists",
            ident = mod_ident,
        ),
        (true, true) => bail!(
            "both '{ident}.rs' and '{ident}/mod.rs' exist",
            ident = mod_ident,
        ),
    }
}

fn preprocess(depth: usize, content: &str) -> Result<String> {
    let content = remove_test_modules(content)?;
    let content = remove_doc_comments(&content)?;
    let content = replace_crate_keywords(depth, &content)?;
    Ok(content)
}

fn remove_test_modules(content: &str) -> Result<String> {
    struct OuterAttributes(Vec<syn::Attribute>);

    impl Parse for OuterAttributes {
        fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
            Ok(OuterAttributes(syn::Attribute::parse_outer(input)?))
        }
    }

    #[derive(Default)]
    struct Visitor {
        spans: Vec<Span>,
    }

    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_mod(&mut self, item_mod: &'ast syn::ItemMod) {
            thread_local! {
                static CFG_TEST_ATTR: syn::Attribute = {
                    let mut attrs = syn::parse_str::<OuterAttributes>("#[cfg(test)]").unwrap();
                    attrs.0.pop().unwrap()
                };
            }

            if item_mod
                .attrs
                .iter()
                .any(|attr| CFG_TEST_ATTR.with(|a| attr == a))
            {
                self.spans.push(item_mod.span());
            } else {
                visit::visit_item_mod(self, item_mod);
            }
        }
    }

    let mut visitor = Visitor::default();
    visitor.visit_file(&syn::parse_file(content)?);

    Ok(remove_spanned_strs(content, visitor.spans))
}

fn remove_doc_comments(content: &str) -> Result<String> {
    #[derive(Default)]
    struct Visitor {
        spans: Vec<Span>,
    }

    impl<'ast> Visit<'ast> for Visitor {
        fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
            thread_local! {
                static DOC_PATH: syn::Path = syn::parse_str("doc").unwrap();
            }

            if DOC_PATH.with(|p| &attr.path == p) {
                self.spans.push(attr.span());
            }
        }
    }

    let mut visitor = Visitor::default();
    visitor.visit_file(&syn::parse_file(content)?);

    Ok(remove_spanned_strs(content, visitor.spans))
}

fn replace_crate_keywords(depth: usize, content: &str) -> Result<String> {
    #[derive(Default)]
    struct Visitor {
        spans: Vec<Span>,
    }

    impl<'ast> Visit<'ast> for Visitor {
        fn visit_ident(&mut self, ident: &'ast syn::Ident) {
            if ident == "crate" {
                self.spans.push(ident.span());
            }
        }
    }

    let mut visitor = Visitor::default();
    visitor.visit_file(&syn::parse_file(content)?);

    let new_path = iter::repeat("super").take(depth).join("::");
    let new_path = || Cow::Borrowed(new_path.as_str());

    Ok(replace_spanned_strs(
        content,
        visitor.spans.into_iter().map(|span| (span, new_path())),
    ))
}

fn remove_spanned_strs(content: &str, spans: impl IntoIterator<Item = Span>) -> String {
    replace_spanned_strs(
        content,
        spans.into_iter().map(|span| (span, Cow::Borrowed(""))),
    )
}

fn replace_spanned_strs<'a>(
    content: &str,
    span_and_new_strs: impl IntoIterator<Item = (Span, Cow<'a, str>)>,
) -> String {
    let mut acc = String::new();
    let mut lines = (1..).zip(content.lines()).peekable();
    let mut start = 0;

    for (span, new_str) in span_and_new_strs {
        for (_, line) in lines.take_while_ref(|&(i, _)| i != span.start().line) {
            acc.push_str(&line[start..]);
            acc.push('\n');
            start = 0;
        }
        let &(_, start_line) = lines.peek().unwrap();
        acc.push_str(&start_line[..span.start().column]);
        acc.push_str(&*new_str);
        lines
            .take_while_ref(|&(i, _)| i != span.end().line)
            .for_each(drop);
        start = span.end().column;
    }
    for (_, line) in lines {
        acc.push_str(&line[start..]);
        acc.push('\n');
        start = 0;
    }

    acc
}

fn spanned_str(content: &str, span: Span) -> String {
    let mut acc = String::new();
    let mut lines = (1..)
        .zip(content.lines())
        .skip_while(|&(i, _)| i != span.start().line);

    if span.start().line == span.end().line {
        let (_, line) = lines.next().unwrap();
        acc.push_str(&line[span.start().column..span.end().column]);
    } else {
        let (_, start_line) = lines.next().unwrap();
        acc.push_str(&start_line[span.start().column..]);
        acc.push('\n');
        for (_, line) in lines.take_while_ref(|&(i, _)| i != span.end().line) {
            acc.push_str(line);
            acc.push('\n');
        }
        let (_, end_line) = lines.next().unwrap();
        acc.push_str(&end_line[..span.end().column]);
    }

    acc
}
