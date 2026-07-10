// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

pub fn print_usage() {
    println!(
        "Usage: trance <command> [args]\n\
         \n\
         Commands:\n\
           version | --version | -V   Print CLI version (no daemon needed)\n\
           about                      Version plus short project info\n\
           status [--json]        Show daemon state\n\
           enable | disable       Toggle idle screensaver\n\
           timeout <minutes>      Set idle timeout (1–240)\n\
           saver set <name|random>\n\
           saver list | list      List installed savers\n\
           inhibitors             List active system inhibitors blocking screensaver\n\
           preview <saver>        Preview a screensaver now\n\
           stop                   Stop preview or idle presentation\n\
           fps-overlay on|off|status  Toggle on-screen FPS overlay\n\
           render-scale <0.25-1.0>|default|status  Simulation grid density (zoom)\n\
           doctor [--fix|-f]      Run system diagnostics; --fix reloads/enables the user service\n\
           config get/set/list    Unified configuration manager\n\
           completion bash/zsh    Generate shell tab-completion scripts\n\
           clean                  Clean stale runs and log caches\n\
           bug-report             Generate sanitized bug reports\n\
           self-update            Check for package updates\n\
           interactive            Open interactive console panel\n"
    );
}
