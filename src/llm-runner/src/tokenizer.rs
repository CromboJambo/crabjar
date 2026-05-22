use tokenizers::Tokenizer as TokenizerImpl;
use tiktoken_rs::bpe_for_model;
use crate::error::RunnerError;
use tracing::debug;

/// Tokenizer for prompt encoding.
///
/// supports BPE models (GPT-2, GPT-3) and tokenizers library.
pub struct Tokenizer {
    pub model: String,
    pub bpe: Option<tiktoken_rs::Bpe>,
    pub tokenizer: Option<TokenizerImpl>,
}

impl Tokenizer {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            bpe: None,
            tokenizer: None,
        }
    }

    /// Initialize BPE from model name.
    pub fn init_bpe(&mut self) -> Result<(), RunnerError> {
        let bpe = bpe_for_model(&self.model).map_err(RunnerError::Tokenizer)?;
        self.bpe = Some(bpe);
        debug!(model = %self.model, "Tokenizer: BPE initialized");
        Ok(())
    }

    /// Encode prompt to token IDs.
    pub fn encode(&self, prompt: &str) -> Result<Vec<u32>, RunnerError> {
        if let Some(ref bpe) = self.bpe {
            bpe.encode(prompt).map_err(RunnerError::Tokenizer)
        } else if let Some(ref tok) = self.tokenizer {
            tok.encode(prompt, false).map_err(RunnerError::Tokenizer).map(|e| e.get_ids().to_vec())
        } else {
            Err(RunnerError::Tokenizer("tokenizer not initialized".to_string()))
        }
    }

    /// Decode token IDs to text.
    pub fn decode(&self, tokens: &[u32]) -> Result<String, RunnerError> {
        if let Some(ref tok) = self.tokenizer {
            tok.decode(tokens, false).map_err(RunnerError::Tokenizer)
        } else {
            Err(RunnerError::Tokenizer("tokenizer not initialized".to_string()))
        }
    }

    /// Get token count.
    pub fn token_count(&self, prompt: &str) -> Result<usize, RunnerError> {
        if let Some(ref bpe) = self.bpe {
            Ok(bpe.encode(prompt).map_err(RunnerError::Tokenizer)?.len())
        } else if let Some(ref tok) = self.tokenizer {
            Ok(tok.encode(prompt, false).map_err(RunnerError::Tokenizer)?.get_ids().len())
        } else {
            Err(RunnerError::Tokenizer("tokenizer not initialized".to_string()))
        }
    }
}
