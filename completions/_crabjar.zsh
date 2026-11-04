# Zsh completion for crabjar CLI
# Install: place in ~/.zsh/completions/_crabjar or add to fpath

#compdef crabjar

_crabjar() {
    local -a commands
    commands=(
        "state:Manage state-docs"
        "knowledge:Manage knowledge store"
        "dotfile:Manage dotfile promotions"
        "workspace:Show workspace configuration"
        "guard:Trust layer and approval management"
        "exec:Execution pipeline"
        "bitwarden:Bitwarden vault operations"
        "doctor:System diagnostics"
        "backend:Backend configuration"
        "tool:Tool registry management"
        "mcp:MCP server interfaces"
        "metrics:Prometheus metrics management"
        "habitat:Habitat contract management"
        "attempts:Execution attempt tracking"
        "decide:Semantic decision layer"
        "slow-friend:Slow friend drift detection"
        "memory:Persistent memory operations"
        "ha:Home Assistant integration"
        "learn:Learning loop analysis"
        "session:Session search and management"
        "fleet:Fleet coordination"
        "checkpoint:Execution checkpoints"
        "approval:Approval workflow management"
        "help:Show usage information"
    )

    _describe -t commands 'crabjar commands' commands
}

_crabjar_state() {
    local -a subcommands
    subcommands=(
        "list:List all state-docs"
        "show:Show a specific state-doc"
        "annotate:Add a note annotation"
        "question:Add a question annotation"
        "resolve:Resolve an annotation"
        "staleness:Check document staleness"
        "rebuild-index:Rebuild the index"
        "create:Create a new state-doc"
        "update:Update an existing state-doc"
    )

    _describe -t subcommands 'state subcommands' subcommands
}

_crabjar_knowledge() {
    local -a subcommands
    subcommands=(
        "add:Add a fact"
        "get:Get a specific fact"
        "list:List all facts"
        "search:Search by keyword"
        "add-json:Add structured JSON data"
    )

    _describe -t subcommands 'knowledge subcommands' subcommands
}

_crabjar_guard() {
    local -a subcommands
    subcommands=(
        "grant:Grant execution permission"
        "revoke:Revoke execution permission"
        "list-pending:List pending approvals"
        "approve:Approve an action"
        "reject:Reject an action"
        "trust-layer:Check current trust layer"
        "upgrade-trust:Upgrade agent trust level"
    )

    _describe -t subcommands 'guard subcommands' subcommands
}

_crabjar_tool() {
    local -a subcommands
    subcommands=(
        "list:List registered tools"
        "search:Search by capability"
        "install:Install a tool"
        "clawhub-search:Search ClawHub for skills"
        "clawhub-install:Install skill from ClawHub"
        "scan:Scan and register tools in directory"
        "update:Update installed tools"
        "remove:Remove an installed tool"
        "info:Show tool information"
    )

    _describe -t subcommands 'tool subcommands' subcommands
}

_crabjar_bitwarden() {
    local -a subcommands
    subcommands=(
        "get:Get a secret"
        "set:Set a secret"
        "list:List secrets"
        "delete:Delete a secret"
        "unlock:Unlock vault"
        "login:Login to Bitwarden"
        "logout:Logout from Bitwarden"
        "sync:Sync vault data"
        "items:Manage items"
        "secrets:Manage secrets"
        "templates:Manage templates"
        "vaults:Manage vaults"
        "collections:Manage collections"
    )

    _describe -t subcommands 'bitwarden subcommands' subcommands
}

case "$words[1]" in
    state)
        if [[ $CURRENT -eq 2 ]]; then
            _crabjar_state
        fi
        ;;
    knowledge)
        if [[ $CURRENT -eq 2 ]]; then
            _crabjar_knowledge
        fi
        ;;
    guard)
        if [[ $CURRENT -eq 2 ]]; then
            _crabjar_guard
        fi
        ;;
    tool)
        if [[ $CURRENT -eq 2 ]]; then
            _crabjar_tool
        fi
        ;;
    bitwarden)
        if [[ $CURRENT -eq 2 ]]; then
            _crabjar_bitwarden
        fi
        ;;
    *)
        _crabjar
        ;;
esac
