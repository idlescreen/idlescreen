// SPDX-License-Identifier: MIT

//! Shell autocompletion script generator module.
//!
//! This module outputs autocomplete definitions for Bash and Zsh
//! so users can leverage Tab completion for subcommands and options.
//!
//! To register:
//! - Bash: `source <(trance completion bash)`
//! - Zsh: `source <(trance completion zsh)`

use anyhow::{Result, anyhow, bail};

pub fn handle_completion(args: &[String]) -> Result<()> {
    if args.is_empty() {
        bail!("usage: trance completion bash | zsh");
    }

    match args[0].as_str() {
        // Output Bash completion script
        "bash" => {
            let script = include_str!("completions/trance.bash");
            println!("{script}");
            Ok(())
        }
        // Output Zsh completion script
        "zsh" => {
            let script = include_str!("completions/trance.zsh");
            println!("{script}");
            Ok(())
        }
        _ => Err(anyhow!(
            "unsupported shell '{}'; please specify 'bash' or 'zsh'",
            args[0]
        )),
    }
}
