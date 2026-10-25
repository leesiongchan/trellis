use super::*;
use std::net::Ipv4Addr;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
pub fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}
pub fn ecma_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub fn truncate_utf16(value: &str, max: usize) -> String {
    let mut used = 0;
    value
        .chars()
        .take_while(|c| {
            used += c.len_utf16();
            used <= max
        })
        .collect()
}
pub(super) fn length(value: &str, min: usize, max: usize) -> Result<(), AppError> {
    if (min..=max).contains(&value.chars().count()) {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
fn count<T>(items: &[T], min: usize, max: usize) -> Result<(), AppError> {
    if (min..=max).contains(&items.len()) {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
pub(super) fn required(value: &Value, names: &[&str]) -> Result<(), AppError> {
    if names.iter().all(|name| value.get(name).is_some()) {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
pub(super) fn question(value: &mut String) -> Result<(), AppError> {
    *value = value.trim_matches(ecma_whitespace).to_owned();
    length(value, 1, 500)
}
pub(super) fn request(input: &mut ResearchRequest) -> Result<(), AppError> {
    length(&input.exploration_id, 1, 200)?;
    length(&input.node_id, 1, 200)?;
    question(&mut input.question)?;
    if let Some(value) = &mut input.clarification {
        question(value)?;
    }
    count(&input.ancestors, 0, 50)?;
    for a in &input.ancestors {
        length(&a.title, 0, 160)?;
        length(&a.summary, 0, 6000)?;
    }
    if input
        .ancestors
        .iter()
        .map(|a| utf16_len(&a.summary))
        .sum::<usize>()
        > 6000
    {
        return Err(AppError::invalid_question());
    }
    Ok(())
}
fn timestamp(value: &str) -> Result<(), AppError> {
    if value.ends_with('Z') && OffsetDateTime::parse(value, &Rfc3339).is_ok() {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
fn paragraph(p: &Paragraph) -> Result<(), AppError> {
    length(&p.id, 1, 200)?;
    length(&p.text, 1, 6000)?;
    count(&p.evidence, 1, 5)?;
    for e in &p.evidence {
        length(&e.source_id, 1, 200)?;
        length(&e.quote, 1, 600)?;
    }
    Ok(())
}
pub(super) fn content(c: &mut TopicContent) -> Result<(), AppError> {
    length(&c.title, 1, 160)?;
    count(&c.summary, 1, 5)?;
    count(&c.sections, 1, 12)?;
    for p in &c.summary {
        paragraph(p)?;
    }
    for s in &c.sections {
        length(&s.heading, 1, 160)?;
        count(&s.paragraphs, 1, 12)?;
        for p in &s.paragraphs {
            paragraph(p)?;
        }
    }
    if let Some(value) = &c.analogy {
        length(value, 0, 3000)?;
    }
    if let Some(value) = &c.limitations {
        length(value, 0, 3000)?;
    }
    count(&c.sources, 1, 5)?;
    for s in &c.sources {
        length(&s.id, 1, 200)?;
        length(&s.title, 1, 500)?;
        length(&s.url, 0, 4096)?;
        if !public_source_url(&s.url) {
            return Err(AppError::invalid_question());
        }
        if let Some(publisher) = &s.publisher {
            length(publisher, 0, 500)?;
        }
        timestamp(&s.retrieved_at)?;
    }
    count(&c.branches, 0, 6)?;
    for b in &mut c.branches {
        length(&b.title, 1, 160)?;
        question(&mut b.question)?;
        length(&b.relationship, 1, 100)?;
        paragraph(&b.reason)?;
    }
    timestamp(&c.researched_at)
}
pub fn public_source_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if host == "localhost"
        || [".localhost", ".local", ".internal"]
            .iter()
            .any(|s| host.ends_with(s))
        || !host.contains('.')
        || host.contains(':')
    {
        return false;
    }
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        let [a, b, _, _] = ip.octets();
        if a == 0
            || a == 10
            || a == 127
            || a >= 224
            || (a == 169 && b == 254)
            || (a == 172 && (16..=31).contains(&b))
            || (a == 192 && (b == 168 || b == 0))
            || (a == 100 && (64..=127).contains(&b))
        {
            return false;
        }
    }
    true
}
