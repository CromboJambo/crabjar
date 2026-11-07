# JEV (jevkit) Evaluation for Crabjar Integration

## What is JEV?

JEV is a typed Rust library wrapping TypeSafe AI's "System One" API. It provides structured decision-making through three question types:

- **Choice**: Pick one from enumerated options
- **Score**: Rate on ordered levels (2-10 scale)
- **Noul**: Yes/no decisions

Each answer comes with a confidence score. Questions are defined as Rust enums using macros (`choice!`, `score!`, `noul!`), providing type safety without JSON handling in application code.

## How It Works

```rust
jevkit::choice! {
    /// Should this command be allowed?
    pub enum GuardDecision {
        /// Safe to execute
        Allow,
        /// Needs human approval
        Approve,
        /// Too risky
        Deny,
    }
}

let client = Client::from_env()?;  // Reads TYPESAFE_API_KEY
let response = client.send(
    &Request::new("Command: rm -rf /tmp/*")
        .choice::<GuardDecision>()
)?;

match response.choice::<GuardDecision>()?.decision().into_confident(0.8) {
    Ok(decision) => println!("guard decided: {:?}", decision.value),
    Err(unsure) => println!("confidence too low: {:.2}", unsure.confidence),
}
```

## Potential Integration Points in Crabjar

### 1. Guard Decision Enhancement
Current guard uses static risk lists and provenance lookup. JEV could provide:
- Semantic understanding of command intent beyond pattern matching
- Context-aware risk assessment (e.g., `rm -rf /tmp/*` is low-risk, `rm -rf /var/log/*` is higher)
- Confidence-based escalation (low confidence → human approval regardless of rule match)

### 2. Trust Layer Scoring
JEV's score questions could complement our binary trust tiers:
```rust
jevkit::score! {
    /// How trustworthy is this execution source?
    pub enum SourceTrust {
        Low,
        Medium,
        High,
        VeryHigh = "Proven user-approved",
    }
}
```

### 3. Policy Decision Support
For complex policy questions where static rules are insufficient:
- Should this agent have network access?
- Is this file pattern sensitive in this context?
- Does this command match the declared purpose?

## Advantages Over Current Approach

1. **Semantic understanding**: Goes beyond regex/pattern matching to understand intent
2. **Confidence calibration**: Built-in confidence scores enable calibrated risk management
3. **Type safety**: Enum-based questions prevent invalid decision values
4. **Low latency**: Designed for System One (fast) decisions, not deep reasoning

## Disadvantages / Concerns

1. **External dependency**: Requires TypeSafe AI API access and API key
2. **Latency**: Network round-trip vs local evaluation
3. **Cost**: Per-question billing model
4. **Vendor lock-in**: Ties crabjar to specific decision service
5. **Privacy**: Command text sent to external service

## Comparison to Alternatives

| Approach | Latency | Cost | Semantic Depth | Offline | Privacy |
|----------|---------|------|----------------|---------|---------|
| Static rules (current) | Microseconds | Free | Low | Yes | Perfect |
| JEV API | ~100ms | Per call | High | No | External |
| Local LLM | ~500ms+ | Compute | High | Yes | Perfect |
| Hybrid (JEV for edge cases) | Variable | Low | Medium-High | Partial | Good |

## Recommendation

**Hybrid approach**: Use JEV as a secondary decision layer for edge cases where static rules are ambiguous, not as the primary guard mechanism.

1. Keep current provenance-based guard as primary
2. Add optional JEV integration for semantic risk assessment when:
   - Command matches multiple risk categories
   - Provenance is weak or missing
   - Confidence threshold not met by local evaluation
3. Cache JEV decisions to reduce API calls and latency

This gives us the benefits of semantic understanding without sacrificing offline operation or creating a hard dependency on external services.

## Implementation Sketch

```rust
// In guard/src/decision.rs
pub enum DecisionBackend {
    StaticRules,      // Current approach
    JevApi,           // External JEV service
    Hybrid,           // Static first, JEV for edge cases
}

impl Guard {
    pub fn evaluate(&self, command: &str, context: &Context) -> GateResult {
        let static_result = self.static_rules.evaluate(command);
        
        if static_result.is_decisive() {
            return static_result;
        }
        
        // Edge case: use JEV for semantic assessment
        if self.use_jev && !static_result.confident {
            return self.jev_assess(command, context);
        }
        
        static_result
    }
}
```

## Next Steps

1. Obtain TypeSafe AI API key for testing
2. Implement JEV integration as optional backend in guard crate
3. Benchmark latency and accuracy vs static rules
4. Evaluate cost at expected query volumes
5. Decide on production deployment based on metrics
