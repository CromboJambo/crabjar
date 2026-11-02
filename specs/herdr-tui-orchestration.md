# Herdr TUI Orchestration Spec

## Overview

A developer workflow environment where herdr terminal multiplexer serves as the orchestration layer, managing multiple TUI applications (editor, git client, file manager) that can be driven both manually and by crabjar agents via the tuiwright MCP interface.

## Architecture

```
┌─────────────────────────────────────────┐
│         Crabjar Agent Layer             │
│  (decision-making, task orchestration)  │
└──────────────────┬──────────────────────┘
                   │ MCP tool calls
┌──────────────────▼──────────────────────┐
│         tuiwright MCP Server            │
│  (pane management, key input, output)   │
└──────────────────┬──────────────────────┘
                   │ SSH / local exec
┌──────────────────▼──────────────────────┐
│           Herdr Sessions                │
│  ┌────────┬────────┬────────┐           │
│  │ Helix  │LazyGit │ Yazi   │ ...       │
│  │ (edit) │(git)   │(files) │           │
│  └────────┴────────┴────────┘           │
└─────────────────────────────────────────┘
```

## Components

### 1. Herdr Session Templates

Predefined layouts for common workflows:

- **`dev`**: Split between editor (top) and terminal (bottom)
- **`review`**: Editor + lazygit side-by-side
- **`explore`**: Yazi file manager full screen
- **`multi-repo`**: Multiple panes, each with its own repo checkout

Template format:
```toml
[templates.dev]
layout = "vertical"
panes = [
  { name = "editor", app = "helix", ratio = 0.7 },
  { name = "terminal", app = "bash", ratio = 0.3 }
]

[templates.review]
layout = "horizontal"
panes = [
  { name = "files", app = "yazi", ratio = 0.3 },
  { name = "git", app = "lazygit", ratio = 0.7 }
]
```

### 2. tuiwright MCP Integration

The tuiwright MCP server provides the following tools for agent-driven TUI control:

- `tui_open(app, args)` — Launch a TUI application in a new pane
- `tui_send_keys(pane_id, keys)` — Send keystrokes to a specific pane
- `tui_snapshot(pane_id)` — Capture current screen state as text
- `tui_assert(pane_id, expected_text)` — Verify pane contains expected content
- `tui_close(pane_id)` — Close a pane/application
- `tui_wait_for(pane_id, pattern)` — Wait for specific output

### 3. Workflow Patterns

#### Manual Development
```bash
herdr session dev --template dev
# User interacts normally with helix + terminal
```

#### Agent-Assisted Review
```heml
// crabjar agent script
let sess = herdr_create_session("review", { template: "review" });
let git_pane = herdr_get_pane(sess, "git");

// Navigate to PR branch in lazygit
tui_send_keys(git_pane, ":checkout feature-branch<Enter>");
await tui_wait_for(git_pane, "feature-branch");

// Open diff view
tui_send_keys(git_pane, "d");  // lazygit: show diff
let diff = await tui_snapshot(git_pane);

// Analyze and provide feedback
let review = analyze_diff(diff);
herdr_comment_on_pr(review);
```

#### Automated Testing Workflow
```heml
let sess = herdr_create_session("test");
let term = herdr_get_pane(sess, "terminal");

tui_send_keys(term, "cargo test --workspace<Enter>");
let result = await tui_wait_for(term, ["test result: ok", "FAILED"]);

if (result.contains("ok")) {
    herdr_notify("All tests passed");
} else {
    let failures = capture_test_output(term);
    herdr_file_issue(failures);
}
```

## Crabjar Integration Points

### 1. Session Management API
```heml
// Create/destroy herdr sessions from crabjar tasks
let session = crabjar_herdr_create({
    name: "task-123-dev",
    template: "dev",
    repo: "/path/to/repo"
});

// Attach agent to existing session
crabjar_herdr_attach(session_id);
```

### 2. Task-to-Session Binding
Kanban tasks can spawn dedicated herdr sessions:
```heml
on kanban_task_assigned(task) {
    let session = crabjar_herdr_create({
        name: `task-${task.id}`,
        template: "dev",
        repo: task.repo_path
    });
    task.metadata.herdr_session = session.id;
}
```

### 3. Automated Testing via TUI
Use herdr + tuiwright to run integration tests against actual TUI workflows:
```heml
test("can edit and commit file") {
    let sess = crabjar_herdr_create({ template: "dev" });
    
    // Edit file in helix
    tui_send_keys(editor_pane, ":open test.txt<Enter>");
    tui_send_keys(editor_pane, "iHello World<Esc>:wq<Enter>");
    
    // Commit in lazygit
    tui_send_keys(git_pane, "a");  // stage all
    tui_send_keys(git_pane, "c");  // commit
    tui_send_keys(git_pane, "Test commit<Esc><Enter>");
    
    // Verify
    expect(tui_snapshot(git_pane)).to_contain("Test commit");
    
    crabjar_herdr_destroy(sess);
}
```

## Development Roadmap

### Phase 1: Foundation (Current)
- [x] herdr terminal multiplexer functional
- [x] tuiwright MCP server operational
- [ ] Session template system
- [ ] Basic crabjar integration API

### Phase 2: Agent Workflows
- [ ] Automated code review workflow
- [ ] Test-driven development patterns
- [ ] Multi-repo management workflows
- [ ] Error recovery and retry logic

### Phase 3: Advanced Features
- [ ] Session recording and replay
- [ ] Parallel workflow execution
- [ ] Cross-session coordination
- [ ] Custom TUI application support

## Success Metrics

- Developer can complete common workflows (edit, test, commit, review) 50% faster with agent assistance
- Agent-driven TUI interactions have <5% error rate
- Session templates reduce setup time to zero for new tasks
- Integration tests cover all critical workflow paths

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| TUI apps change keybindings | Version-pin TUI apps; maintain compatibility matrix |
| Agent misinterprets screen state | Use structured assertions, not just visual matching |
| Herdr session leaks | Auto-cleanup on task completion/failure |
| Slow interaction latency | Batch operations where possible; optimize SSH connections |

## Related Projects

- **herdr**: Terminal multiplexer with agent-driven pane management
- **tuiwright**: MCP server for TUI automation
- **crabjar**: Agent orchestration framework
- **helix**: Modern text editor (primary editing tool)
- **lazygit**: Git TUI client
- **yazi**: Fast file manager

## Open Questions — Decisions

### 1. Fork TUI apps for agent integration?
**Decision:** Start with upstream; fork only when integration pain becomes a real blocker.
- Helix (Rust) is the most likely candidate if deep integration is needed — same language, active plugin ecosystem
- lazygit and yazi (Go) are well-maintained and stable — low ROI to fork initially
- Criterion for forking: upstream changes break agent workflows more than twice per quarter

### 2. Authentication flows in TUIs
**Decision:** Leverage existing lab infrastructure + prompt interception fallback
1. **lab-key:** Pre-distribute SSH keys across fleet — agents use lab-key to provision access before spawning sessions
2. **ssh-bastion:** All remote connections route through bastion host for centralized auth and logging
3. **Prompt interception:** tuiwright detects auth prompts and responds programmatically (last resort)

Implementation:
```heml
// Before spawning herdr session on remote machine
lab_key_provision({
    target: "ftw3",
    user: "agent",
    key_type: "ed25519"
});

// Session connects via bastion automatically
let sess = herdr_create_session("remote-work", {
    host: "ftw3",
    via_bastion: true  // uses ssh-bastion config
});
```

Herdr sessions inherit the agent's SSH context — no interactive key entry needed for standard workflows.

### 3. Long-running operations pattern
**Decision:** Background execution with notification + status polling
- Launch operation in dedicated herdr pane
- Crabjar agent polls via `tui_snapshot()` at intervals (configurable, default 5s)
- On completion detected (via output pattern), agent processes results
- Timeout handling: if no completion signal within threshold, agent escalates or retries

Example:
```heml
let build_pane = herdr_spawn_pane("build", "cargo build --release");
let status = await tui_wait_for(build_pane, ["Finished", "error"], timeout_s: 300);
if (status == "Finished") {
    // proceed
} else {
    let log = tui_snapshot(build_pane);
    // analyze failure
}
```

### 4. Custom "herdr-aware" editor plugin?
**Decision:** Phase 2/3 feature — defer until core workflows are proven.
- Start with key-based interaction only (works today)
- If agent needs to know cursor position, open files, or buffer state → build helix plugin then
- Plugin would expose: current file path, cursor position, selected text, undo stack depth
- Interface: WebSocket sidecar that helix plugin communicates with; crabjar queries sidecar

Rationale: Don't add complexity until there's a demonstrated need from real workflows.

## Implementation Status

### Phase 1: Foundation (IN PROGRESS)

**Completed:**
- [x] Session template tool definitions added to crabjar-herdr-mcp (TemplateListRequest, TemplateCreateSessionRequest, TemplateSaveRequest)
- [x] MCP tool implementations for `template_list`, `template_create_session`, `template_save`
- [x] Initial session templates created: `dev`, `review`, `explore` in ~/.herdr/templates/
- [x] crabjar-herdr-mcp builds successfully with new template tools

**Remaining:**
- [ ] Implement `herdr session create` CLI command (underlying support for template_create_session)
- [ ] Integrate tuiwright MCP server into crabjar-herdr-mcp or deploy as separate service
- [ ] Add automated testing framework for herdr + tuiwright integration

### Phase 2: Agent Workflows (NOT STARTED)

**Planned:**
- [ ] Implement PR review workflow using lazygit template
- [ ] Implement code exploration workflow with search-driven navigation
- [ ] Build crabjar conductor integration for task-to-session mapping

### Phase 3: Advanced Features (NOT STARTED)

**Planned:**
- [ ] SSH remote session support via ssh-bastion
- [ ] State persistence and resume capability
- [ ] Custom "herdr-aware" editor plugin for buffer state access