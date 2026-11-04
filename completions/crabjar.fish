# Fish completion for crabjar CLI
# Install: place in ~/.config/fish/completions/crabjar.fish

complete -c crabjar -n "__fish_use_subcommand" -a "state" -d "Manage state-docs"
complete -c crabjar -n "__fish_use_subcommand" -a "knowledge" -d "Manage knowledge store"
complete -c crabjar -n "__fish_use_subcommand" -a "dotfile" -d "Manage dotfile promotions"
complete -c crabjar -n "__fish_use_subcommand" -a "workspace" -d "Show workspace configuration"
complete -c crabjar -n "__fish_use_subcommand" -a "guard" -d "Trust layer and approval management"
complete -c crabjar -n "__fish_use_subcommand" -a "exec" -d "Execution pipeline"
complete -c crabjar -n "__fish_use_subcommand" -a "bitwarden" -d "Bitwarden vault operations"
complete -c crabjar -n "__fish_use_subcommand" -a "doctor" -d "System diagnostics"
complete -c crabjar -n "__fish_use_subcommand" -a "backend" -d "Backend configuration"
complete -c crabjar -n "__fish_use_subcommand" -a "tool" -d "Tool registry management"
complete -c crabjar -n "__fish_use_subcommand" -a "mcp" -d "MCP server interfaces"
complete -c crabjar -n "__fish_use_subcommand" -a "metrics" -d "Prometheus metrics management"
complete -c crabjar -n "__fish_use_subcommand" -a "habitat" -d "Habitat contract management"
complete -c crabjar -n "__fish_use_subcommand" -a "attempts" -d "Execution attempt tracking"
complete -c crabjar -n "__fish_use_subcommand" -a "decide" -d "Semantic decision layer"
complete -c crabjar -n "__fish_use_subcommand" -a "slow-friend" -d "Slow friend drift detection"
complete -c crabjar -n "__fish_use_subcommand" -a "memory" -d "Persistent memory operations"
complete -c crabjar -n "__fish_use_subcommand" -a "ha" -d "Home Assistant integration"
complete -c crabjar -n "__fish_use_subcommand" -a "learn" -d "Learning loop analysis"
complete -c crabjar -n "__fish_use_subcommand" -a "session" -d "Session search and management"
complete -c crabjar -n "__fish_use_subcommand" -a "fleet" -d "Fleet coordination"
complete -c crabjar -n "__fish_use_subcommand" -a "checkpoint" -d "Execution checkpoints"
complete -c crabjar -n "__fish_use_subcommand" -a "approval" -d "Approval workflow management"
complete -c crabjar -n "__fish_use_subcommand" -a "help" -d "Show usage information"

# State subcommands
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "list show annotate question resolve staleness rebuild-index create update"
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "list" -d "List all state-docs"
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "show" -d "Show a specific state-doc"
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "annotate" -d "Add a note annotation"
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "question" -d "Add a question annotation"
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "resolve" -d "Resolve an annotation"
complete -c crabjar -n "__fish_seen_subcommand_from state" -a "staleness" -d "Check document staleness"

# Knowledge subcommands
complete -c crabjar -n "__fish_seen_subcommand_from knowledge" -a "add get list search add-json"
complete -c crabjar -n "__fish_seen_subcommand_from knowledge" -a "add" -d "Add a fact"
complete -c crabjar -n "__fish_seen_subcommand_from knowledge" -a "get" -d "Get a specific fact"
complete -c crabjar -n "__fish_seen_subcommand_from knowledge" -a "list" -d "List all facts"
complete -c crabjar -n "__fish_seen_subcommand_from knowledge" -a "search" -d "Search by keyword"

# Guard subcommands
complete -c crabjar -n "__fish_seen_subcommand_from guard" -a "grant revoke list-pending approve reject trust-layer upgrade-trust"
complete -c crabjar -n "__fish_seen_subcommand_from guard" -a "list-pending" -d "List pending approvals"
complete -c crabjar -n "__fish_seen_subcommand_from guard" -a "approve" -d "Approve an action"
complete -c crabjar -n "__fish_seen_subcommand_from guard" -a "reject" -d "Reject an action"

# Tool subcommands
complete -c crabjar -n "__fish_seen_subcommand_from tool" -a "list search install clawhub-search clawhub-install scan update remove info"
complete -c crabjar -n "__fish_seen_subcommand_from tool" -a "list" -d "List registered tools"
complete -c crabjar -n "__fish_seen_subcommand_from tool" -a "search" -d "Search by capability"
complete -c crabjar -n "__fish_seen_subcommand_from tool" -a "install" -d "Install a tool"

# Bitwarden subcommands
complete -c crabjar -n "__fish_seen_subcommand_from bitwarden" -a "get set list delete unlock login logout sync items secrets templates vaults collections"
complete -c crabjar -n "__fish_seen_subcommand_from bitwarden" -a "get" -d "Get a secret"
complete -c crabjar -n "__fish_seen_subcommand_from bitwarden" -a "set" -d "Set a secret"
complete -c crabjar -n "__fish_seen_subcommand_from bitwarden" -a "list" -d "List secrets"

# Doctor subcommands
complete -c crabjar -n "__fish_seen_subcommand_from doctor" -a "check"
complete -c crabjar -n "__fish_seen_subcommand_from doctor" -a "check" -d "Run system health check"
