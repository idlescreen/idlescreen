// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

pub fn print_usage() {
    println!(
        "Usage: trance <command> [args]\n\
         \n\
         Global flags (GNU style):\n\
           -h, --help              Show this help\n\
           -V, --version           Print version (same as: version / v)\n\
         \n\
         Commands (short aliases in parentheses):\n\
           version (v)             Print CLI version (no daemon needed)\n\
           about                   Version plus short project info\n\
           status (st) [--json]    Show daemon state\n\
           enable (on)             Turn idle screensaver on\n\
           disable (off)           Turn idle screensaver off\n\
           timeout (t) <minutes>   Set idle timeout (1–240)\n\
           saver set <name|random>\n\
           saver list | list (ls)  List installed savers\n\
           inhibitors              List active inhibitors\n\
           preview (p) <saver>     Preview a screensaver now\n\
           stop                    Stop preview or idle presentation\n\
           fps-overlay (fps) on|off|status\n\
           render-scale (scale) <0.25-1.0>|default|status\n\
           doctor (doc) [--fix|-f] Diagnostics; --fix reloads user service\n\
           config (cfg) get|set|list\n\
           completion bash|zsh     Shell tab-completion scripts\n\
           clean                   Clean stale runs and log caches\n\
           bug-report              Sanitized diagnostics for bug reports\n\
           self-update (update)    Check for package updates (apt/dnf)\n\
           interactive (i)         Interactive console panel\n\
           help                    Show this help\n\
         \n\
         Note: use --help not -help; use --version not -version.\n"
    );
}
