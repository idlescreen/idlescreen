# trance bash completion
_trance_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="version about status enable disable timeout saver list preview stop fps-overlay render-scale doctor config completion clean bug-report self-update interactive help"

    case "${prev}" in
        preview)
            local savers="beams bursts chaos cosmos glyphs gnats radar storm"
            COMPREPLY=( $(compgen -W "${savers}" -- ${cur}) )
            return 0
            ;;
        config)
            local config_opts="get set list"
            COMPREPLY=( $(compgen -W "${config_opts}" -- ${cur}) )
            return 0
            ;;
        completion)
            local shell_opts="bash zsh"
            COMPREPLY=( $(compgen -W "${shell_opts}" -- ${cur}) )
            return 0
            ;;
        *)
            ;;
    esac

    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
    return 0
}
complete -F _trance_completion trance
