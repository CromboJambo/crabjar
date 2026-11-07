# EXPL-004: Structural Unification via Constraint Propagation

**Status:** Exploration  
**Date:** 2026-10-10  
**Author:** CromboJambo  

## Background

Euler's mathematical legacy demonstrates that seemingly unrelated objects can be connected through common underlying structure. His identity e^(iπ) + 1 = 0 ties together five fundamental constants, while his graph theory work (Königsberg bridge problem) showed how to discard irrelevant details and preserve relationships.

This suggests a computational approach: represent problems as networks of constraints where information flows between connected units, rather than processing linearly or hierarchically.

### Physical Analogue: Settling into Structure

A motivating thought experiment: pack alternating large and small squares into a container balanced on an axis that passes through two opposite corners (vertices) of the cube — not through face centers. Rotate the container; introduce small pieces first (placeholders), then large pieces, repeating. Centripetal forces and gravity settle each batch before the next is introduced. The gaps left by one settling stage become constraints on the next — negative space participates in determining where subsequent pieces can go.

The question: does this iterative relaxation converge to a stable, repeating local pattern? If so, the arrangement itself encodes relationships without needing explicit coordinates for every connection. The structure emerges from the dynamics of constraint satisfaction, not from pre-computed placement.

This physical process — establish constraints, introduce one class of pieces, let dynamics find stability, introduce next class, repeat — maps directly onto a computational model where subproblems inform each other through shared boundary conditions rather than top-down decomposition.

## Core Idea

Build a constraint propagation system on a toroidal grid where:
- Each cell contains: value, structure type, neighbor connections, constraints, derivation status
- Cells exchange information (values AND structures) with neighbors
- The torus topology eliminates boundary effects
- Solving one part makes other parts easier to solve

This differs from existing constraint propagation systems in that cells would exchange *structures* (recognizable patterns) not just values.

## Connection to crabjar

crabjar's structural tokenization work already pursues this principle: preserving meaningful relationships rather than flattening into sequences. This exploration extends that idea to problem-solving itself.

Potential applications:
- Agent orchestration as constraint satisfaction between task cells
- Dependency resolution via propagation rather than topological sort
- Parallel problem decomposition where subproblems inform each other

## Experiment Design

1. Build minimal grid of constraint/variable cells
2. Implement basic constraint propagation (value exchange first)
3. Add structure exchange (pattern recognition and reuse)
4. Compare against conventional solvers on:
   - Steps required to solution
   - Work duplication
   - Local update efficiency
   - Contradiction detection

## Success Criteria

- Demonstrable reduction in redundant computation vs conventional approaches
- Successful propagation of structural patterns between cells
- Clean handling of contradictions with provenance tracking

## Open Questions

- Is the Fibonacci/toroidal geometry actually beneficial, or is it the constraint propagation itself?
- Can structure exchange scale beyond toy problems?
- How does this compare to existing factor graph / belief propagation systems?
