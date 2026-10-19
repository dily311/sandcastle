use tiktoken::encoding_for_model;

pub fn count_tokens(model: &str, text: &str) -> Option<usize> {
    let encoding = encoding_for_model(model)?;
    Some(encoding.count(text))
}