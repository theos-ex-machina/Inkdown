use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Options, Parser, Tag};
use regex::Regex;

const MATH_BLOCK_OPEN: char = '\u{E000}';
const MATH_INLINE_OPEN: char = '\u{E001}';
const PDF_OPEN: char = '\u{E002}';

static MATH_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)\$\$(.+?)\$\$").expect("math block re"));
// `regex` crate doesn't support lookbehind, so we approximate "not preceded
// by backslash" inside the replace closure below.
static MATH_INLINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$([^\s\$\\][^\$\n]*?[^\s\$\\]|[^\s\$\\])\$").expect("math inline re")
});

fn parser_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options
}

fn extract_math(src: &str) -> (String, Vec<String>, Vec<String>) {
    let mut blocks: Vec<String> = Vec::new();
    let mut inlines: Vec<String> = Vec::new();

    let with_blocks = MATH_BLOCK_RE.replace_all(src, |caps: &regex::Captures| {
        let idx = blocks.len();
        blocks.push(caps[1].to_string());
        format!("{0}MB{1}{0}", MATH_BLOCK_OPEN, idx)
    });

    let with_inlines = MATH_INLINE_RE.replace_all(&with_blocks, |caps: &regex::Captures| {
        let m = caps.get(0).unwrap();
        let start = m.start();
        // Skip if this `$...$` is escaped: `\$...$`
        if start > 0 && with_blocks.as_bytes()[start - 1] == b'\\' {
            return m.as_str().to_string();
        }
        let idx = inlines.len();
        inlines.push(caps[1].to_string());
        format!("{0}MI{1}{0}", MATH_INLINE_OPEN, idx)
    });

    (with_inlines.into_owned(), blocks, inlines)
}

/// Replace any pdf-fenced-code text content with placeholder tokens so we can
/// swap the whole `<pre><code class="language-pdf">...</code></pre>` for an
/// `<embed>` in inject_pdf().
fn extract_pdf_paths(src: &str) -> (String, Vec<String>) {
    let pdf_block_re = Regex::new(r"(?ms)^```pdf\s*\n(.*?)\n```\s*$").expect("pdf block re");
    let mut paths: Vec<String> = Vec::new();
    let replaced = pdf_block_re.replace_all(src, |caps: &regex::Captures| {
        let path = caps[1].trim().to_string();
        let idx = paths.len();
        paths.push(path);
        format!("```pdf\n{0}P{1}{0}\n```", PDF_OPEN, idx)
    });
    (replaced.into_owned(), paths)
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn inject_pdf(html: &str, pdfs: &[String]) -> String {
    let pdf_re = Regex::new(&format!(
        r#"(?s)<pre><code class="language-pdf">\s*{0}P(\d+){0}\s*</code></pre>"#,
        regex::escape(&PDF_OPEN.to_string())
    ))
    .expect("pdf inject re");
    pdf_re
        .replace_all(html, |caps: &regex::Captures| {
            let idx: usize = caps[1].parse().unwrap_or(usize::MAX);
            let src = pdfs.get(idx).cloned().unwrap_or_default();
            format!(
                "<embed class=\"pdf-embed\" src=\"{}\" type=\"application/pdf\" />",
                html_escape(src.trim())
            )
        })
        .into_owned()
}

fn inject_math(html: &str, blocks: &[String], inlines: &[String]) -> String {
    let mut out = html.to_string();

    let block_re = Regex::new(&format!(
        r"{0}MB(\d+){0}",
        regex::escape(&MATH_BLOCK_OPEN.to_string())
    ))
    .expect("math block inject re");
    out = block_re
        .replace_all(&out, |caps: &regex::Captures| {
            let idx: usize = caps[1].parse().unwrap_or(usize::MAX);
            let body = blocks.get(idx).cloned().unwrap_or_default();
            format!(
                "<div class=\"math-block\">{}</div>",
                html_escape(body.trim())
            )
        })
        .into_owned();

    let inline_re = Regex::new(&format!(
        r"{0}MI(\d+){0}",
        regex::escape(&MATH_INLINE_OPEN.to_string())
    ))
    .expect("math inline inject re");
    out = inline_re
        .replace_all(&out, |caps: &regex::Captures| {
            let idx: usize = caps[1].parse().unwrap_or(usize::MAX);
            let body = inlines.get(idx).cloned().unwrap_or_default();
            format!("<span class=\"math-inline\">{}</span>", html_escape(&body))
        })
        .into_owned();

    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    BlockQuote,
    CodeBlock,
    OrderedList,
    UnorderedList,
    Table,
}

impl BlockKind {
    fn open_tag_prefix(self) -> &'static str {
        match self {
            BlockKind::Paragraph => "<p",
            BlockKind::Heading1 => "<h1",
            BlockKind::Heading2 => "<h2",
            BlockKind::Heading3 => "<h3",
            BlockKind::Heading4 => "<h4",
            BlockKind::Heading5 => "<h5",
            BlockKind::Heading6 => "<h6",
            BlockKind::BlockQuote => "<blockquote",
            BlockKind::CodeBlock => "<pre",
            BlockKind::OrderedList => "<ol",
            BlockKind::UnorderedList => "<ul",
            BlockKind::Table => "<table",
        }
    }
}

fn block_kind_from_tag(tag: &Tag<'_>) -> Option<BlockKind> {
    use pulldown_cmark::HeadingLevel as H;
    Some(match tag {
        Tag::Paragraph => BlockKind::Paragraph,
        Tag::Heading { level: H::H1, .. } => BlockKind::Heading1,
        Tag::Heading { level: H::H2, .. } => BlockKind::Heading2,
        Tag::Heading { level: H::H3, .. } => BlockKind::Heading3,
        Tag::Heading { level: H::H4, .. } => BlockKind::Heading4,
        Tag::Heading { level: H::H5, .. } => BlockKind::Heading5,
        Tag::Heading { level: H::H6, .. } => BlockKind::Heading6,
        Tag::BlockQuote(_) => BlockKind::BlockQuote,
        Tag::CodeBlock(_) => BlockKind::CodeBlock,
        Tag::List(Some(_)) => BlockKind::OrderedList,
        Tag::List(None) => BlockKind::UnorderedList,
        Tag::Table(_) => BlockKind::Table,
        _ => return None,
    })
}

fn compute_line_offsets(src: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn line_for(starts: &[usize], byte_pos: usize) -> u32 {
    let line = match starts.binary_search_by(|s| s.cmp(&byte_pos)) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    line as u32
}

fn collect_top_level_blocks(src: &str) -> Vec<(BlockKind, u32)> {
    let line_starts = compute_line_offsets(src);
    let mut depth = 0usize;
    let mut out: Vec<(BlockKind, u32)> = Vec::new();
    for (event, range) in Parser::new_ext(src, parser_options()).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    if let Some(kind) = block_kind_from_tag(&tag) {
                        out.push((kind, line_for(&line_starts, range.start)));
                    }
                }
                depth += 1;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    out
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Insert `data-line` attributes on the N-th occurrence of each tracked
/// top-level block tag, in document order. Best-effort; if the renderer
/// produced a tag we didn't predict, we just skip ahead.
fn inject_data_lines(html: &str, lines: &[(BlockKind, u32)]) -> String {
    let mut out = String::with_capacity(html.len() + lines.len() * 16);
    let mut cursor = 0usize;
    let bytes = html.as_bytes();
    for (kind, line) in lines {
        let needle = kind.open_tag_prefix();
        let Some(rel) = find_subslice(&bytes[cursor..], needle.as_bytes()) else {
            break;
        };
        let pos = cursor + rel;
        out.push_str(&html[cursor..pos]);
        out.push_str(needle);
        let after_tag = pos + needle.len();
        let next_byte = bytes.get(after_tag).copied().unwrap_or(b'\0');
        if next_byte == b'>' || next_byte == b' ' {
            out.push_str(&format!(" data-line=\"{}\"", line));
        }
        cursor = after_tag;
    }
    out.push_str(&html[cursor..]);
    out
}

/// Strip raw HTML events from the parser stream as a defense-in-depth
/// sanitizer. We don't allow arbitrary html, but we do allow `<br>` /
/// `<img ...>` / `<sub>` / `<sup>` so users can mix common inline tags.
fn sanitize_event(event: Event<'_>) -> Event<'_> {
    match event {
        Event::Html(h) | Event::InlineHtml(h) => {
            let s = h.as_ref();
            if is_safe_inline_html(s) {
                Event::InlineHtml(h)
            } else {
                Event::Text(html_escape(s).into())
            }
        }
        other => other,
    }
}

fn is_safe_inline_html(s: &str) -> bool {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    const ALLOWED: &[&str] = &[
        "<br>", "<br/>", "<br />", "</br>",
        "<sub>", "</sub>", "<sup>", "</sup>",
        "<details>", "</details>", "<summary>", "</summary>",
    ];
    if ALLOWED.contains(&lower.as_str()) {
        return true;
    }
    if lower.starts_with("<img ") && lower.ends_with('>') && !lower.contains("javascript:") {
        return true;
    }
    false
}

pub fn render(markdown_src: &str) -> String {
    let (math_pre, math_blocks, math_inlines) = extract_math(markdown_src);
    let (pre, pdfs) = extract_pdf_paths(&math_pre);

    let blocks = collect_top_level_blocks(&pre);

    let mut html = String::new();
    let parser = Parser::new_ext(&pre, parser_options()).map(sanitize_event);
    pulldown_cmark::html::push_html(&mut html, parser);

    let html = inject_pdf(&html, &pdfs);
    let html = inject_data_lines(&html, &blocks);
    inject_math(&html, &math_blocks, &math_inlines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_paragraph() {
        let html = render("hello world");
        assert!(html.contains("<p data-line=\"0\">hello world</p>"));
    }

    #[test]
    fn inline_math_becomes_span() {
        let html = render("the formula $x^2$ is famous");
        assert!(html.contains("<span class=\"math-inline\">"));
        assert!(html.contains("x^2"));
    }

    #[test]
    fn block_math_becomes_div() {
        let html = render("$$\nE = mc^2\n$$");
        assert!(html.contains("<div class=\"math-block\">"));
    }

    #[test]
    fn pdf_fence_becomes_embed() {
        let src = "```pdf\n../_assets/doc.pdf\n```";
        let html = render(src);
        assert!(
            html.contains("<embed class=\"pdf-embed\""),
            "missing embed in: {html}"
        );
    }

    #[test]
    fn raw_script_is_stripped() {
        let html = render("hi <script>alert(1)</script> bye");
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn img_html_is_preserved() {
        let html = render("look <img src=\"x.png\" width=\"100\"> here");
        assert!(html.contains("<img"));
    }
}
