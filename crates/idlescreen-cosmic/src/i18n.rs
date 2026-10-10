// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Localization for the applet.
//!
//! The applet ships a single `en` Fluent file, so the i18n-embed +
//! rust-embed + fluent stack collapses to: embed the file with
//! `include_str!`, parse `key = value` lines once, substitute `{$arg}`
//! placeholders. The `fl!` macro keeps the same call-site shape as
//! `i18n_embed_fl::fl!`. Unknown ids render as the id itself, matching
//! Fluent's fallback behavior.

use std::sync::LazyLock;

const FTL: &str = include_str!("../i18n/en/idle_applet.ftl");

static TABLE: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
    FTL.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            line.split_once('=').map(|(k, v)| (k.trim(), v.trim()))
        })
        .collect()
});

/// Look up `id` and substitute `{$name}` placeholders with `args`.
pub fn message(id: &str, args: &[(&str, String)]) -> String {
    let Some(tpl) = TABLE.iter().find(|(k, _)| *k == id).map(|(_, v)| *v) else {
        return id.to_string();
    };
    let mut out = tpl.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{${name}}}"), value);
    }
    out
}

/// Request a localized string by ID (i18n_embed_fl `fl!` equivalent).
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        $crate::i18n::message($message_id, &[])
    }};
    ($message_id:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        $crate::i18n::message(
            $message_id,
            &[$( (stringify!($name), $value.to_string()), )*],
        )
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn known_ids_resolve() {
        assert_eq!(crate::i18n::message("app-title", &[]), "IdleScreen");
    }

    #[test]
    fn args_substitute() {
        assert_eq!(
            crate::i18n::message("timeout-minutes", &[("mins", "5".to_string())]),
            "5 min"
        );
    }

    #[test]
    fn unknown_id_renders_id() {
        assert_eq!(crate::i18n::message("missing-key", &[]), "missing-key");
    }
}
