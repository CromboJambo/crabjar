#[cfg(test)]
mod tests {
    use super::*;
    use codeburn_provider::SessionData;
    use codeburn_provider::ProvenanceEntry;

    #[test]
    fn pricing_engine_new_defaults() {
        let engine = PricingEngine::new();
        assert!(engine.pricing_data.is_empty());
    }

    #[test]
    fn pricing_engine_default_works() {
        let engine = PricingEngine::default();
        assert!(engine.pricing_data.is_empty());
    }

    #[test]
    fn built_in_aliases_clopus() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("claude-3-opus"));
        assert_eq!(aliases["claude-3-opus"], "claude_opus");
    }

    #[test]
    fn built_in_aliases_claude_sonnet() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("claude-3-sonnet"));
        assert_eq!(aliases["claude-3-sonnet"], "claude_sonnet");
    }

    #[test]
    fn built_in_aliases_claude_haiku() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("claude-3-haiku"));
        assert_eq!(aliases["claude-3-haiku"], "claude_haiku");
    }

    #[test]
    fn built_in_aliases_gpt4o() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gpt-4o"));
        assert_eq!(aliases["gpt-4o"], "gpt_4o");
    }

    #[test]
    fn built_in_aliases_gpt4o_mini() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gpt-4o-mini"));
        assert_eq!(aliases["gpt-4o-mini"], "gpt_4o_mini");
    }

    #[test]
    fn built_in_aliases_gpt4() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gpt-4"));
        assert_eq!(aliases["gpt-4"], "gpt_4");
    }

    #[test]
    fn built_in_aliases_gpt35() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gpt-3.5-turbo"));
        assert_eq!(aliases["gpt-3.5-turbo"], "gpt_35");
    }

    #[test]
    fn built_in_aliases_gemini_pro() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gemini-pro"));
        assert_eq!(aliases["gemini-pro"], "gemini_pro");
    }

    #[test]
    fn built_in_aliases_gemini_15_pro() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gemini-1.5-pro"));
        assert_eq!(aliases["gemini-1.5-pro"], "gemini_15_pro");
    }

    #[test]
    fn built_in_aliases_gemini_15_flash() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("gemini-1.5-flash"));
        assert_eq!(aliases["gemini-1.5-flash"], "gemini_15_flash");
    }

    #[test]
    fn built_in_aliases_mistral_large() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("mistral-large"));
        assert_eq!(aliases["mistral-large"], "mistral_large");
    }

    #[test]
    fn built_in_aliases_mistral_7b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("mistral-7b"));
        assert_eq!(aliases["mistral-7b"], "mistral_7b");
    }

    #[test]
    fn built_in_aliases_llama_3_70b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("llama-3-70b"));
        assert_eq!(aliases["llama-3-70b"], "llama_3_70b");
    }

    #[test]
    fn built_in_aliases_llama_3_8b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("llama-3-8b"));
        assert_eq!(aliases["llama-3-8b"], "llama_3_8b");
    }

    #[test]
    fn built_in_aliases_qwen_25_72b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen-2.5-72b"));
        assert_eq!(aliases["qwen-2.5-72b"], "qwen_25_72b");
    }

    #[test]
    fn built_in_aliases_qwen_25_32b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen-2.5-32b"));
        assert_eq!(aliases["qwen-2.5-32b"], "qwen_25_32b");
    }

    #[test]
    fn built_in_aliases_qwen_25_14b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen-2.5-14b"));
        assert_eq!(aliases["qwen-2.5-14b"], "qwen_25_14b");
    }

    #[test]
    fn built_in_aliases_qwen_25_7b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen-2.5-7b"));
        assert_eq!(aliases["qwen-2.5-7b"], "qwen_25_7b");
    }

    #[test]
    fn built_in_aliases_qwen3_4b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen3-4b"));
        assert_eq!(aliases["qwen3-4b"], "qwen3_4b");
    }

    #[test]
    fn built_in_aliases_qwen3_8b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen3-8b"));
        assert_eq!(aliases["qwen3-8b"], "qwen3_8b");
    }

    #[test]
    fn built_in_aliases_qwen3_32b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen3-32b"));
        assert_eq!(aliases["qwen3-32b"], "qwen3_32b");
    }

    #[test]
    fn built_in_aliases_qwen3_235b() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("qwen3-235b"));
        assert_eq!(aliases["qwen3-235b"], "qwen3_235b");
    }

    #[test]
    fn built_in_aliases_deepseek_v3() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("deepseek-v3"));
        assert_eq!(aliases["deepseek-v3"], "deepseek_v3");
    }

    #[test]
    fn built_in_aliases_deepseek_r1() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("deepseek-r1"));
        assert_eq!(aliases["deepseek-r1"], "deepseek_r1");
    }

    #[test]
    fn built_in_aliases_codex() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("codex"));
        assert_eq!(aliases["codex"], "openai_codex");
    }

    #[test]
    fn built_in_aliases_cursor() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("cursor"));
        assert_eq!(aliases["cursor"], "cursor");
    }

    #[test]
    fn built_in_aliases_claude_desktop() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("claude-desktop"));
        assert_eq!(aliases["claude-desktop"], "claude_desktop");
    }

    #[test]
    fn built_in_aliases_claude_code() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("claude-code"));
        assert_eq!(aliases["claude-code"], "claude_code");
    }

    #[test]
    fn built_in_aliases_pi() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("pi"));
        assert_eq!(aliases["pi"], "pi");
    }

    #[test]
    fn built_in_aliases_omp() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("omp"));
        assert_eq!(aliases["omp"], "omp");
    }

    #[test]
    fn built_in_aliases_copilot() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("copilot"));
        assert_eq!(aliases["copilot"], "github_copilot");
    }

    #[test]
    fn built_in_aliases_local_model() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("local-model"));
        assert_eq!(aliases["local-model"], "local_model");
    }

    #[test]
    fn built_in_aliases_lm_studio() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains_key("lm-studio"));
        assert_eq!(aliases["lm-studio"], "lm_studio");
    }

    #[test]
    fn built_in_aliases_non_empty() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.len() > 0);
    }

    #[test]
    fn pricing_metrics_default_values() {
        let metrics = PricingMetrics {
            total_cost: 0.0,
            daily: Vec::new(),
            by_project: BTreeMap::new(),
            by_model: BTreeMap::new(),
            by_activity: BTreeMap::new(),
            by_tool: BTreeMap::new(),
            by_mcp: BTreeMap::new(),
            by_shell: BTreeMap::new(),
            top_sessions: Vec::new(),
            efficiency: 0.0,
            style: "unknown".to_string(),
        };
        assert_eq!(metrics.total_cost, 0.0);
        assert_eq!(metrics.style, "unknown");
    }

    #[test]
    fn pricing_metrics_serialize_works() {
        let metrics = PricingMetrics {
            total_cost: 100.0,
            daily: Vec::new(),
            by_project: BTreeMap::new(),
            by_model: BTreeMap::new(),
            by_activity: BTreeMap::new(),
            by_tool: BTreeMap::new(),
            by_mcp: BTreeMap::new(),
            by_shell: BTreeMap::new(),
            top_sessions: Vec::new(),
            efficiency: 0.0,
            style: "unknown".to_string(),
        };
        let serialized = serde_json::to_string(&metrics).unwrap();
        assert!(serialized.contains("100"));
    }

    #[test]
    fn pricing_metrics_deserialize_works() {
        let serialized = serde_json::to_string(&PricingMetrics {
            total_cost: 100.0,
            daily: Vec::new(),
            by_project: BTreeMap::new(),
            by_model: BTreeMap::new(),
            by_activity: BTreeMap::new(),
            by_tool: BTreeMap::new(),
            by_mcp: BTreeMap::new(),
            by_shell: BTreeMap::new(),
            top_sessions: Vec::new(),
            efficiency: 0.0,
            style: "unknown".to_string(),
        }).unwrap();
        let deserialized: PricingMetrics = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.total_cost, 100.0);
    }

    #[test]
    fn pricing_entry_serialize_works() {
        let entry = PricingEntry {
            model: "claude".to_string(),
            input_price: 0.01,
            output_price: 0.02,
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("claude"));
    }

    #[test]
    fn pricing_entry_deserialize_works() {
        let serialized = serde_json::to_string(&PricingEntry {
            model: "claude".to_string(),
            input_price: 0.01,
            output_price: 0.02,
        }).unwrap();
        let deserialized: PricingEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.model, "claude");
    }

    #[test]
    fn pricing_entry_zero_prices() {
        let entry = PricingEntry {
            model: "local".to_string(),
            input_price: 0.0,
            output_price: 0.0,
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("0"));
    }

    #[test]
    fn pricing_entry_large_prices() {
        let entry = PricingEntry {
            model: "opus".to_string(),
            input_price: 15.0,
            output_price: 75.0,
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("15"));
    }

    #[test]
    fn pricing_engine_clone_works() {
        let engine = PricingEngine::new();
        let cloned = engine.clone();
        assert_eq!(cloned.cache_path, engine.cache_path);
    }
}
