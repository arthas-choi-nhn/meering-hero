use std::collections::HashMap;

/// Post-processor for STT results.
/// Removes filler words and applies domain-specific corrections.
pub struct PostProcessor {
    corrections: HashMap<String, String>,
}

impl PostProcessor {
    pub fn new() -> Self {
        Self {
            corrections: HashMap::new(),
        }
    }

    pub fn with_corrections(corrections: HashMap<String, String>) -> Self {
        Self { corrections }
    }

    pub fn process(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove common Korean filler words
        result = Self::remove_fillers(&result);

        // Apply domain-specific corrections
        for (wrong, correct) in &self.corrections {
            result = result.replace(wrong, correct);
        }

        result
    }

    fn remove_fillers(text: &str) -> String {
        // Common Korean filler patterns
        // Only remove when they appear as standalone tokens (with spaces)
        let fillers = [" 음 ", " 어 ", " 그 ", " 아 ", " 에 "];
        let mut result = text.to_string();
        for filler in &fillers {
            result = result.replace(filler, " ");
        }
        // Clean up multiple spaces
        while result.contains("  ") {
            result = result.replace("  ", " ");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filler_removal() {
        let pp = PostProcessor::new();
        let result = pp.process(" 음 HAProxy 어 보안 룰 그 업데이트 ");
        assert_eq!(result, " HAProxy 보안 룰 업데이트 ");
    }

    #[test]
    fn test_corrections() {
        let mut corrections = HashMap::new();
        corrections.insert("에치에이프록시".to_string(), "HAProxy".to_string());
        corrections.insert("크라우드섹".to_string(), "CrowdSec".to_string());
        let pp = PostProcessor::with_corrections(corrections);
        let result = pp.process("에치에이프록시 크라우드섹 설정");
        assert_eq!(result, "HAProxy CrowdSec 설정");
    }
}
