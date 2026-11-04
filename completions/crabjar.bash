# Bash completion for crabjar CLI
# Install: source this file or place in ~/.bash_completion.d/crabjar

_crabjar_commands() {
    local commands="state knowledge dotfile workspace guard exec bitwarden doctor backend tool mcp metrics habitat attempts decide slow-friend memory ha learn session fleet checkpoint approval help"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_state_commands() {
    local commands="list show annotate question resolve staleness rebuild-index create update"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_knowledge_commands() {
    local commands="add get list search add-json"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_dotfile_commands() {
    local commands="promote"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_workspace_commands() {
    local commands="status"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_guard_commands() {
    local commands="grant revoke list-pending approve reject trust-layer upgrade-trust"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_bitwarden_commands() {
    local commands="get set list delete unlock login logout sync items secrets templates vaults collections"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_doctor_commands() {
    local commands="check"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_backend_commands() {
    local commands="set get list test"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_tool_commands() {
    local commands="list search install clawhub-search clawhub-install scan update remove info"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_mcp_commands() {
    local commands="list call tools resources schemas describe"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_metrics_commands() {
    local commands="list get set reset export import"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_habitat_commands() {
    local commands="contract list show update create delete validate"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_attempts_commands() {
    local commands="status rewind new list show"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_decide_commands() {
    local commands="evaluate feedback metrics promote-rule learnings list-criteria validate"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_slow_friend_commands() {
    local commands="status analyze drift report"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_memory_commands() {
    local commands="add get list delete search annotate"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_ha_commands() {
    local commands="status entities services call-service get-state list-entities list-services"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_learn_commands() {
    local commands="analyze training-candidates failure-patterns"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_session_commands() {
    local commands="search related failures"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_fleet_commands() {
    local commands="submit-kanban-task register capabilities status"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_checkpoint_commands() {
    local commands="save restore list show delete"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar_approval_commands() {
    local commands="list approve reject status"
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
}

_crabjar() {
    local cur prev words cword
    _init_completion || return

    # Complete first-level commands
    if [[ $cword -eq 1 ]]; then
        _crabjar_commands
        return
    fi

    # Complete second-level commands based on first command
    case "${words[1]}" in
        state)
            if [[ $cword -eq 2 ]]; then
                _crabjar_state_commands
            fi
            ;;
        knowledge)
            if [[ $cword -eq 2 ]]; then
                _crabjar_knowledge_commands
            fi
            ;;
        dotfile)
            if [[ $cword -eq 2 ]]; then
                _crabjar_dotfile_commands
            fi
            ;;
        workspace)
            if [[ $cword -eq 2 ]]; then
                _crabjar_workspace_commands
            fi
            ;;
        guard)
            if [[ $cword -eq 2 ]]; then
                _crabjar_guard_commands
            fi
            ;;
        bitwarden)
            if [[ $cword -eq 2 ]]; then
                _crabjar_bitwarden_commands
            fi
            ;;
        doctor)
            if [[ $cword -eq 2 ]]; then
                _crabjar_doctor_commands
            fi
            ;;
        backend)
            if [[ $cword -eq 2 ]]; then
                _crabjar_backend_commands
            fi
            ;;
        tool)
            if [[ $cword -eq 2 ]]; then
                _crabjar_tool_commands
            fi
            ;;
        mcp)
            if [[ $cword -eq 2 ]]; then
                _crabjar_mcp_commands
            fi
            ;;
        metrics)
            if [[ $cword -eq 2 ]]; then
                _crabjar_metrics_commands
            fi
            ;;
        habitat)
            if [[ $cword -eq 2 ]]; then
                _crabjar_habitat_commands
            fi
            ;;
        attempts)
            if [[ $cword -eq 2 ]]; then
                _crabjar_attempts_commands
            fi
            ;;
        decide)
            if [[ $cword -eq 2 ]]; then
                _crabjar_decide_commands
            fi
            ;;
        slow-friend)
            if [[ $cword -eq 2 ]]; then
                _crabjar_slow_friend_commands
            fi
            ;;
        memory)
            if [[ $cword -eq 2 ]]; then
                _crabjar_memory_commands
            fi
            ;;
        ha)
            if [[ $cword -eq 2 ]]; then
                _crabjar_ha_commands
            fi
            ;;
        learn)
            if [[ $cword -eq 2 ]]; then
                _crabjar_learn_commands
            fi
            ;;
        session)
            if [[ $cword -eq 2 ]]; then
                _crabjar_session_commands
            fi
            ;;
        fleet)
            if [[ $cword -eq 2 ]]; then
                _crabjar_fleet_commands
            fi
            ;;
        checkpoint)
            if [[ $cword -eq 2 ]]; then
                _crabjar_checkpoint_commands
            fi
            ;;
        approval)
            if [[ $cword -eq 2 ]]; then
                _crabjar_approval_commands
            fi
            ;;
    esac
}

complete -F _crabjar crabjar
