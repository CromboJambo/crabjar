# P2-3: Expand Built-in Skill Ecosystem

**Date:** November 2, 2026  
**Status:** Active  
**Priority:** Productivity (P2)  
**North Star Test:** Does this move crabjar closer to being a drop-in replacement for Hermes Agent? Yes — users migrating from Hermes expect access to the full skill ecosystem.

## Overview

CrabJar currently ships with 32 local skills, while Hermes has 300+ built-in skills across multiple domains. This task expands crabjar's skill ecosystem by porting high-value Hermes skills and establishing categorization, documentation standards, and testing infrastructure.

## Scope

### In Scope
- Port essential Hermes skills to CrabJar format (SKILL.md + supporting files)
- Establish skill categorization aligned with Hermes' category structure
- Create documentation standards for SKILL.md consistency
- Build automated testing framework for skill behavior validation
- Focus on highest-impact categories first (GitHub, development workflow, productivity)

### Out of Scope
- Porting all 300+ Hermes skills (focus on essential subset)
- Remote skill discovery infrastructure (already complete — P1-9)
- Custom plugin development (separate task)

## Skill Categories to Port

### Priority 1: GitHub Workflow Skills (8 skills)
These are the most commonly used skills for developers and provide immediate value.

1. **github-auth** — GitHub auth setup: HTTPS tokens, SSH keys, gh CLI login
2. **github-code-review** — Review PRs: diffs, inline comments via gh or REST
3. **github-issue-to-pr** — Carry a GitHub issue to a verified PR with honest CI state
4. **github-issues** — Create, triage, label, assign GitHub issues via gh or REST
5. **github-pr-workflow** — GitHub PR lifecycle: branch, commit, open, CI, merge
6. **github-repo-management** — Clone/create/fork repos; manage remotes, releases
7. **github-workflow-management** — Centralized hub for all GitHub interactions
8. **oss-pr-description** — PR descriptions for OSS maintainers (what+why only)

### Priority 2: Development Workflow Skills (10 skills)
Essential for software development workflows.

9. **plan** — Write a markdown plan to .hermes/plans/; no execution
10. **software-engineering-workflow** — Full lifecycle of professional software development
11. **systematic-debugging** — 4-phase root cause debugging: understand before fixing
12. **adversarial-code-review** — Structured methodology for finding real logic bugs
13. **comprehensive-codebase-review** — Deep architectural review of complex codebases
14. **rust-triage** — Systematic triage for Rust projects
15. **rust-debugging** — Debugging Rust abstraction layer bugs, trait implementations
16. **project-maintenance** — Clean up clutter and historical noise in a repo
17. **open-source-pr-workflow** — Submit to large OSS projects without 'poisoning'
18. **test-coverage** — Improve Rust crate test coverage

### Priority 3: Productivity Skills (6 skills)
Document creation, presentations, spreadsheets.

19. **microsoft-365-cli** — Use Microsoft 365 services from terminal via ICS subscriptions
20. **nano-pdf** — Edit text in existing PDFs via natural-language prompts
21. **ocr-and-documents** — Extract text from PDFs/scans (pymupdf, marker-pdf)
22. **session-librarian** — Organize sessions by prompt: find, rename, archive, prune
23. **knowledge-base-management** — Centralized hub for structuring accumulated knowledge
24. **engineering-documentation-standards** — Guidelines for writing technical docs

### Priority 4: Research Skills (5 skills)
Academic research, paper discovery, literature review.

25. **empirical-math-research** — Test mathematical conjectures computationally
26. **reddit-reading** — Read Reddit: subreddits, search, threads, users
27. **web-research-workflow** — Systematic web research methodology
28. **literature-review** — Academic literature review workflow
29. **paper-discovery** — Discover relevant academic papers

## Implementation Approach

### Phase 1: Infrastructure (Week 1)
- [ ] Create skill categorization structure in `.agents/skills/`
- [ ] Define SKILL.md documentation standards template
- [ ] Build automated testing framework for skill validation
- [ ] Create skill import tooling to streamline porting from Hermes format

### Phase 2: Priority 1 Skills (Weeks 2-3)
- [ ] Port 8 GitHub workflow skills
- [ ] Validate each skill against testing framework
- [ ] Document integration with existing crabjar tools

### Phase 3: Priority 2 Skills (Weeks 4-5)
- [ ] Port 10 development workflow skills
- [ ] Validate each skill against testing framework
- [ ] Ensure compatibility with crabjar's execution pipeline

### Phase 4: Priority 3 & 4 Skills (Weeks 6-7)
- [ ] Port 11 productivity and research skills
- [ ] Validate each skill against testing framework
- [ ] Complete documentation and categorization

## Deliverables

1. **Expanded skill ecosystem** — 29 additional skills (61 total, up from 32)
2. **Categorization structure** — Aligned with Hermes' category system
3. **Documentation standards** — Consistent SKILL.md format across all skills
4. **Testing framework** — Automated validation for skill behavior
5. **Import tooling** — Streamlined process for future skill additions

## Success Criteria

- All 29 new skills pass automated testing framework
- Skills are properly categorized and documented
- Users can migrate from Hermes without losing access to essential skills
- Skill import process is streamlined for future additions

## Dependencies

- P1-9: Remote skill discovery infrastructure (complete)
- Existing crabjar execution pipeline (guard → concierge → telemetry)
- Existing memory system for skill state persistence

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Skill format differences between Hermes and CrabJar | Create import tooling to handle conversion |
| Incomplete testing coverage | Build comprehensive test framework before porting |
| Scope creep (porting too many skills) | Focus on essential subset (29 skills) |
| Documentation inconsistency | Define standards template upfront |

## Notes

- This task is a productivity improvement, not a drop-in replacement blocker
- Users can already use crabjar without these skills, but the experience is significantly improved with them
- The skill ecosystem is a key differentiator for crabjar vs other agent frameworks
- Future work may include porting additional skills based on user demand

## Provenance

Generated from feature parity analysis comparing crabjar's 32 local skills against Hermes' 300+ built-in skills. Priority ordering based on usage frequency and impact assessment.