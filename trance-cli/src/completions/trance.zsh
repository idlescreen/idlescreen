#compdef trance

_trance() {
    local line
    _arguments -C \
        '1: :->cmd' \
        '*:: :->args'

    case "$state" in
        cmd)
            _values "trance command" \
                "version[Print CLI version]" \
                "about[Version and short project info]" \
                "status[Show daemon state]" \
                "enable[Toggle idle screensaver on]" \
                "disable[Toggle idle screensaver off]" \
                "timeout[Set idle timeout]" \
                "saver[Control active screensaver]" \
                "list[List installed savers]" \
                "preview[Preview a screensaver now]" \
                "stop[Stop preview or idle presentation]" \
                "fps-overlay[Toggle on-screen FPS overlay]" \
                "render-scale[Simulation grid density]" \
                "doctor[Run system diagnostics]" \
                "config[Unified configuration controller]" \
                "completion[Shell completion scripts]" \
                "clean[Clean stale runs and logs]" \
                "bug-report[Generate sanitized diagnostics report]" \
                "self-update[Verify system updates]" \
                "interactive[Start text-based control panel]" \
                "help[Print usage information]"
            ;;
        args)
            case "$line[1]" in
                preview)
                    _values "screensavers" "beams" "bursts" "chaos" "cosmos" "glyphs" "gnats" "radar" "storm"
                    ;;
                config)
                    _values "config actions" "get" "set" "list"
                    ;;
                completion)
                    _values "shells" "bash" "zsh"
                    ;;
            esac
            ;;
    esac
}
_trance "$@"
