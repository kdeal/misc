use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    config::{resolve_secret, Config},
    llm::SseReader,
};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerplexityModel {
    #[default]
    Sonar,
    SonarPro,
    SonarReasoning,
    SonarReasoningPro,
}

#[derive(Debug, Default, Serialize)]
pub struct PerplexityRequest {
    pub messages: Vec<super::Message>,
    pub model: PerplexityModel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Choice {
    pub delta: super::Message,
    pub message: super::Message,
    pub finish_reason: Option<String>,
    pub index: i32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub completion_tokens: i32,
    pub prompt_tokens: i32,
    pub total_tokens: i32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PerplexityResponse {
    pub choices: Vec<Choice>,
    pub created: i64,
    pub id: String,
    pub model: PerplexityModel,
    pub usage: Usage,
    pub citations: Option<Vec<String>>,
}

pub struct PerplexityStreamResponseIterator {
    sse_reader: SseReader,
    done: bool,
}

impl Iterator for PerplexityStreamResponseIterator {
    type Item = anyhow::Result<PerplexityResponse>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Get the next SSE event
        let event = match self.sse_reader.next() {
            Some(Ok(event)) => event,
            None => return None,
            Some(Err(e)) => return Some(Err(e)),
        };

        // OpenAI terminates with the sentinel "[DONE]". Perplexity is based off
        // OpenAI, so it might do this. I haven't seen it do this though.
        if event.data.trim() == "[DONE]" {
            self.done = true;
            return None;
        }

        // Parse the response
        let response = match serde_json::from_str::<PerplexityResponse>(&event.data) {
            Ok(response) => response,
            Err(e) => return Some(Err(anyhow!(e))),
        };

        // Check if this is the last chunk
        if response.choices[0].finish_reason.is_some() {
            self.done = true;
        }

        Some(Ok(response))
    }
}

pub struct PerplexityClient {
    api_key: String,
}

impl PerplexityClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn create_chat_completion(
        &self,
        request: PerplexityRequest,
    ) -> anyhow::Result<PerplexityResponse> {
        let response = ureq::post("https://api.perplexity.ai/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&request)?;

        let completion = response.into_json::<PerplexityResponse>()?;
        Ok(completion)
    }

    pub fn stream_chat_completion(
        &self,
        mut request: PerplexityRequest,
    ) -> anyhow::Result<PerplexityStreamResponseIterator> {
        request.stream = Some(true);

        let response = ureq::post("https://api.perplexity.ai/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&request)?;

        Ok(PerplexityStreamResponseIterator {
            sse_reader: SseReader::new(Box::new(response.into_reader())),
            done: false,
        })
    }
}

fn extract_supports_from_text(
    text: &str,
    sources: &[super::Source],
) -> (Vec<super::Support>, String) {
    let mut cleaned_text = String::new();
    let mut supports = vec![];
    let mut char_iter = text.chars().peekable();
    let num_sources: u8 = u8::try_from(sources.len()).expect("Should be able to get u8 from usize");
    let mut sentence_ends = vec![0];
    loop {
        // Push non references onto the new string
        // Can't use take_while because it consumes the [
        while let Some(peek_char) = char_iter.peek() {
            if peek_char == &'[' {
                break;
            }
            let next_char = char_iter
                .next()
                .expect("We peeked this, so it can't be None");
            cleaned_text.push(next_char);

            if next_char == '.' || next_char == '\n' {
                sentence_ends.push(cleaned_text.len());
            }
        }
        if char_iter.peek().is_none() {
            break;
        }

        let mut consumed_text = String::new();
        let mut source_indices = vec![];
        while let Some(char) = char_iter.next() {
            if char != '[' {
                consumed_text.push(char);
                break;
            }

            let mut source_num_str = String::new();
            while let Some(peek_char) = char_iter.peek() {
                if peek_char == &']' {
                    break;
                }
                let next_char = char_iter
                    .next()
                    .expect("We peeked this, so it can't be None");
                source_num_str.push(next_char);
            }

            let source_num = u8::from_str(&source_num_str).unwrap_or(u8::MAX);
            if source_num > num_sources {
                consumed_text.push('[');
                consumed_text.push_str(&source_num_str);
                break;
            }

            let maybe_next_char = char_iter.next();
            if let Some(next_char) = maybe_next_char {
                if next_char == ']' {
                    source_indices.push(source_num);
                } else {
                    consumed_text.push('[');
                    consumed_text.push_str(&source_num_str);
                    consumed_text.push(next_char);
                }
            } else {
                consumed_text.push('[');
                consumed_text.push_str(&source_num_str);
                break;
            }
        }

        if !source_indices.is_empty() {
            if cleaned_text.ends_with(" ") {
                cleaned_text.pop().unwrap_or(' ');
            }

            // TODO: Do this in a smarter way
            let sentence_end_index = sentence_ends[sentence_ends.len() - 2];
            // Find the next character after sentence ended
            let start_index = cleaned_text
                .chars()
                .enumerate()
                .skip(sentence_end_index)
                .find(|(_, ch)| ch.is_alphabetic())
                .map(|(i, _)| i)
                .unwrap_or(sentence_end_index);

            let current_index = cleaned_text.len();
            supports.push(super::Support {
                start_index,
                end_index: current_index,
                text: cleaned_text[start_index..current_index].to_string(),
                source_indices,
            });
        }
        cleaned_text.push_str(&consumed_text);
    }
    (supports, cleaned_text)
}

fn extract_title_from_url(url_str: &str) -> String {
    if let Ok(url) = Url::parse(url_str) {
        if let Some(host_str) = url.host_str() {
            return String::from_str(host_str).expect("Can create String from host_str");
        }
    };
    String::from_str(url_str).expect("Can create String from url_str")
}

impl super::LlmProvider for PerplexityClient {
    fn from_config(config: Config) -> anyhow::Result<Self> {
        let api_key_raw = config
            .perplexity_api_key
            .ok_or(anyhow!("Missing perplexity_api_key in config"))?;
        let api_key = resolve_secret(&api_key_raw)?;
        Ok(Self::new(api_key))
    }
}

impl super::GroundedChat for PerplexityClient {
    fn create_grounded_chat_completion(
        &self,
        request: super::GroundedChatRequest,
    ) -> anyhow::Result<super::GroundedChatResponse> {
        let model = match request.model_type {
            super::ModelType::Small => PerplexityModel::Sonar,
            super::ModelType::Large => PerplexityModel::SonarPro,
            super::ModelType::Thinking => PerplexityModel::SonarReasoningPro,
        };
        let request = PerplexityRequest {
            messages: vec![super::Message {
                role: super::Role::User,
                content: request.query,
            }],
            model,
            ..PerplexityRequest::default()
        };
        let response = self.create_chat_completion(request)?;
        let choice = response
            .choices
            .into_iter()
            .nth(0)
            .expect("It should always return a canidate");
        let sources: Vec<super::Source> = response
            .citations
            .unwrap_or(vec![])
            .into_iter()
            .map(|citation| {
                let title = extract_title_from_url(&citation);
                super::Source {
                    uri: citation,
                    title,
                }
            })
            .collect();
        let (supports, content) = extract_supports_from_text(&choice.message.content, &sources);
        Ok(super::GroundedChatResponse {
            message: super::Message {
                role: choice.message.role,
                content,
            },
            citations: super::CitationMetadata { sources, supports },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::extract_supports_from_text;

    fn create_test_sources() -> Vec<super::super::Source> {
        vec![
            super::super::Source {
                title: "Source 1".to_string(),
                uri: "http://example.com/1".to_string(),
            },
            super::super::Source {
                title: "Source 2".to_string(),
                uri: "http://example.com/2".to_string(),
            },
        ]
    }

    #[test]
    fn test_single_reference() {
        let sources = create_test_sources();
        let input = "This is a test sentence.[0] This has no reference.";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 1);
        assert_eq!(supports[0].text, "This is a test sentence.");
        assert_eq!(supports[0].start_index, 0);
        assert_eq!(supports[0].end_index, 24);
        assert_eq!(supports[0].source_indices, vec![0]);
        assert_eq!(
            cleaned_text,
            "This is a test sentence. This has no reference."
        );
    }

    #[test]
    fn test_multiple_references() {
        let sources = create_test_sources();
        let input = "This is supported by multiple sources.[0][1]";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        println!("{:?} - {}", supports, cleaned_text);
        assert_eq!(supports.len(), 1);
        assert_eq!(supports[0].text, "This is supported by multiple sources.");
        assert_eq!(supports[0].start_index, 0);
        assert_eq!(supports[0].end_index, 38);
        assert_eq!(supports[0].source_indices, vec![0, 1]);
        assert_eq!(cleaned_text, "This is supported by multiple sources.");
    }

    #[test]
    fn test_invalid_reference() {
        let sources = create_test_sources();
        let input = "This has an invalid reference.[99] This is normal text.";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 0);
        assert_eq!(
            cleaned_text,
            "This has an invalid reference.[99] This is normal text."
        );
    }

    #[test]
    fn test_multiple_sentences() {
        let sources = create_test_sources();
        let input = "First sentence.[0] Second sentence.[1] Third sentence with no reference.";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 2);
        assert_eq!(supports[0].text, "First sentence.");
        assert_eq!(supports[0].source_indices, vec![0]);
        assert_eq!(supports[1].text, "Second sentence.");
        assert_eq!(supports[1].source_indices, vec![1]);
        assert_eq!(
            cleaned_text,
            "First sentence. Second sentence. Third sentence with no reference."
        );
    }

    #[test]
    fn test_empty_text() {
        let sources = create_test_sources();
        let input = "";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 0);
        assert_eq!(cleaned_text, "");
    }

    #[test]
    fn test_no_references() {
        let sources = create_test_sources();
        let input = "This text has no references at all.";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 0);
        assert_eq!(cleaned_text, input);
    }

    #[test]
    fn test_consecutive_references() {
        let sources = create_test_sources();
        let input = "This is referenced.[0][1] This is also referenced.[0][1]";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 2);
        assert_eq!(supports[0].text, "This is referenced.");
        assert_eq!(supports[0].source_indices, vec![0, 1]);
        assert_eq!(supports[1].text, "This is also referenced.");
        assert_eq!(supports[1].source_indices, vec![0, 1]);
        assert_eq!(cleaned_text, "This is referenced. This is also referenced.");
    }

    #[test]
    fn test_mixed_valid_and_invalid_references() {
        let sources = create_test_sources();
        let input = "Valid reference.[0] Invalid reference.[99] Another valid one.[1]";
        let (supports, cleaned_text) = extract_supports_from_text(input, &sources);

        assert_eq!(supports.len(), 2);
        assert_eq!(supports[0].text, "Valid reference.");
        assert_eq!(supports[0].source_indices, vec![0]);
        assert_eq!(supports[1].text, "Another valid one.");
        assert_eq!(supports[1].source_indices, vec![1]);
        assert_eq!(
            cleaned_text,
            "Valid reference. Invalid reference.[99] Another valid one."
        );
    }
}
