# idle bash completion
_idle_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="version v about status st enable on disable off timeout t saver list ls preview p stop fps-overlay fps render-scale scale doctor doc config cfg completion clean bug-report self-update update interactive i help"

    case "${prev}" in
        preview|p)
            local savers="beams bursts chaos cosmos glyphs gnats radar storm hearth ripple random shuffle --timeout -t"
            COMPREPLY=( $(compgen -W "${savers}" -- ${cur}) )
            return 0
            ;;
        saver)
            local saver_cmds="set list random shuffle"
            COMPREPLY=( $(compgen -W "${saver_cmds}" -- ${cur}) )
            return 0
            ;;
        config|cfg)
            local config_opts="get set list"
            COMPREPLY=( $(compgen -W "${config_opts}" -- ${cur}) )
            return 0
            ;;
        completion)
            local shell_opts="bash zsh fish nu"
            COMPREPLY=( $(compgen -W "${shell_opts}" -- ${cur}) )
            return 0
            ;;
        fps-overlay|fps)
            COMPREPLY=( $(compgen -W "on off status" -- ${cur}) )
            return 0
            ;;
        doctor|doc)
            COMPREPLY=( $(compgen -W "--fix -f --json -j" -- ${cur}) )
            return 0
            ;;
        *)
            ;;
    esac

    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
    return 0
}
complete -F _idle_completion idle
