use crate::comments::append_join_space_to_last_segment;
use docolint_types::TextSegment;

pub(super) fn extract_line_doc_group(
    pieces: &[(usize, &str)],
    unit_id: usize,
    next_unit_id: &mut usize,
) -> Option<Vec<TextSegment>> {
    let fragment = build_line_doc_fragment(pieces)?;
    extract_fragment(&fragment, unit_id, next_unit_id)
}

pub(super) fn extract_block_doc(
    comment_start: usize,
    raw: &str,
    unit_id: usize,
    next_unit_id: &mut usize,
) -> Option<Vec<TextSegment>> {
    let fragment = build_block_doc_fragment(comment_start, raw)?;
    extract_fragment(&fragment, unit_id, next_unit_id)
}

struct Fragment {
    text: String,
    source_offsets: Vec<Option<usize>>,
}

struct XmlExtractor<'a> {
    fragment: &'a Fragment,
    segments: Vec<TextSegment>,
    next_unit_id: &'a mut usize,
    pending_space_unit_id: Option<usize>,
    last_text_end_index: Option<usize>,
}

impl<'a> XmlExtractor<'a> {
    fn new(fragment: &'a Fragment, next_unit_id: &'a mut usize) -> Self {
        Self {
            fragment,
            segments: Vec::new(),
            next_unit_id,
            pending_space_unit_id: None,
            last_text_end_index: None,
        }
    }

    fn fresh_unit_id(&mut self) -> usize {
        let unit_id = *self.next_unit_id;
        *self.next_unit_id += 1;
        unit_id
    }

    fn push_char_data(&mut self, node: tree_sitter::Node<'_>, current_unit_id: &mut Option<usize>) {
        let Some((text, offset, first, last, had_leading_ws)) = self.trimmed_text(node) else {
            return;
        };

        let unit_id = *current_unit_id.get_or_insert_with(|| self.fresh_unit_id());
        let has_whitespace_gap = self.last_text_end_index.is_some_and(|previous_end| {
            previous_end < first
                && self.fragment.text[previous_end + 1..first]
                    .bytes()
                    .any(|byte| byte.is_ascii_whitespace())
        });
        let needs_join_space = (self.pending_space_unit_id == Some(unit_id)
            || has_whitespace_gap
            || (had_leading_ws
                && self
                    .segments
                    .last()
                    .is_some_and(|segment| segment.unit_id == unit_id)))
            && !text.starts_with(char::is_whitespace);
        if needs_join_space {
            append_join_space_to_last_segment(&mut self.segments, unit_id);
        }
        self.pending_space_unit_id = None;
        self.segments.push(TextSegment {
            text,
            is_markup: false,
            offset,
            unit_id,
        });
        self.last_text_end_index = Some(last);
    }

    fn trimmed_text(
        &self,
        node: tree_sitter::Node<'_>,
    ) -> Option<(String, usize, usize, usize, bool)> {
        let start = node.start_byte().checked_sub(WRAPPER_PREFIX.len())?;
        let end = node.end_byte().checked_sub(WRAPPER_PREFIX.len())?;
        if end > self.fragment.text.len() || start >= end {
            return None;
        }

        let bytes = self.fragment.text.as_bytes();
        let mut first = None;
        let mut last = None;
        for index in start..end {
            if bytes[index].is_ascii_whitespace() {
                continue;
            }
            if self.fragment.source_offsets[index].is_none() {
                continue;
            }
            first.get_or_insert(index);
            last = Some(index);
        }

        let (first, last) = (first?, last?);
        let text = self.fragment.text[first..=last].to_string();
        let offset = self.fragment.source_offsets[first]?;
        Some((text, offset, first, last, first > start))
    }

    fn schedule_space(&mut self, unit_id: Option<usize>) {
        self.pending_space_unit_id = unit_id;
    }

    fn traverse_children(
        &mut self,
        node: tree_sitter::Node<'_>,
        current_unit_id: &mut Option<usize>,
    ) {
        let mut cursor = node.walk();
        if !cursor.goto_first_child() {
            return;
        }

        loop {
            let child = cursor.node();
            self.traverse_node(child, current_unit_id);
            if !cursor.goto_next_sibling() {
                break;
            }
        }

        cursor.goto_parent();
    }

    fn traverse_node(&mut self, node: tree_sitter::Node<'_>, current_unit_id: &mut Option<usize>) {
        match node.kind() {
            "document" | "content" | "ERROR" => self.traverse_children(node, current_unit_id),
            "CharData" | "CData" => self.push_char_data(node, current_unit_id),
            "element" => self.traverse_element(node, current_unit_id),
            _ => self.traverse_children(node, current_unit_id),
        }
    }

    fn traverse_element(
        &mut self,
        node: tree_sitter::Node<'_>,
        current_unit_id: &mut Option<usize>,
    ) {
        let Some(name) = element_name(node, &self.fragment.text) else {
            self.traverse_children(node, current_unit_id);
            return;
        };

        match name.as_str() {
            "summary" | "remarks" | "returns" | "value" | "param" | "typeparam" | "exception"
            | "permission" | "example" => {
                let mut block_unit_id = Some(self.fresh_unit_id());
                self.traverse_children(node, &mut block_unit_id);
                *current_unit_id = None;
                self.pending_space_unit_id = None;
            }
            "para" => {
                let mut paragraph_unit_id = Some(self.fresh_unit_id());
                self.traverse_children(node, &mut paragraph_unit_id);
                *current_unit_id = None;
                self.pending_space_unit_id = None;
            }
            "list" => {
                let mut list_context = None;
                self.traverse_children(node, &mut list_context);
                *current_unit_id = None;
                self.pending_space_unit_id = None;
            }
            "item" | "listheader" => {
                let mut item_unit_id = Some(self.fresh_unit_id());
                self.traverse_children(node, &mut item_unit_id);
                *current_unit_id = None;
                self.pending_space_unit_id = None;
            }
            "description" | "b" | "i" | "u" | "a" => self.traverse_children(node, current_unit_id),
            "term" | "include" | "inheritdoc" => {
                *current_unit_id = None;
                self.pending_space_unit_id = None;
            }
            "code" => {
                self.schedule_space(*current_unit_id);
                *current_unit_id = None;
            }
            "br" => self.schedule_space(*current_unit_id),
            "c" | "paramref" | "typeparamref" => self.schedule_space(*current_unit_id),
            "see" => {
                if element_has_prose(node) {
                    self.traverse_children(node, current_unit_id);
                } else {
                    self.schedule_space(*current_unit_id);
                }
            }
            "seealso" => {
                if element_has_prose(node) {
                    self.traverse_children(node, current_unit_id);
                    *current_unit_id = None;
                } else {
                    *current_unit_id = None;
                }
                self.pending_space_unit_id = None;
            }
            _ => self.traverse_children(node, current_unit_id),
        }
    }
}

const WRAPPER_PREFIX: &str = "<doc>";
const WRAPPER_SUFFIX: &str = "</doc>";

fn extract_fragment(
    fragment: &Fragment,
    unit_id: usize,
    next_unit_id: &mut usize,
) -> Option<Vec<TextSegment>> {
    let wrapped = format!("{WRAPPER_PREFIX}{}{WRAPPER_SUFFIX}", fragment.text);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_xml::LANGUAGE_XML.into())
        .ok()?;
    let tree = parser.parse(&wrapped, None)?;
    let mut extractor = XmlExtractor::new(fragment, next_unit_id);

    let mut root_unit_id = Some(unit_id);
    extractor.traverse_node(tree.root_node(), &mut root_unit_id);
    if tree.root_node().has_error() {
        supplement_missing_error_fallback(fragment, &mut extractor.segments, unit_id);
    }

    if extractor.segments.is_empty() {
        extract_error_fallback(fragment, unit_id, next_unit_id)
    } else {
        Some(extractor.segments)
    }
}

fn element_name(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return None;
    }

    let mut tag_node = None;
    loop {
        let child = cursor.node();
        if matches!(child.kind(), "STag" | "EmptyElemTag") {
            tag_node = Some(child);
            break;
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();

    let tag_node = tag_node?;
    let mut tag_cursor = tag_node.walk();
    if !tag_cursor.goto_first_child() {
        return None;
    }

    let mut name = None;
    loop {
        let child = tag_cursor.node();
        if child.kind() == "Name" {
            let start = child.start_byte().checked_sub(WRAPPER_PREFIX.len())?;
            let end = child.end_byte().checked_sub(WRAPPER_PREFIX.len())?;
            name = source.get(start..end).map(str::to_string);
            break;
        }
        if !tag_cursor.goto_next_sibling() {
            break;
        }
    }
    tag_cursor.goto_parent();

    name.map(|name| name.to_ascii_lowercase())
}

fn element_has_prose(node: tree_sitter::Node<'_>) -> bool {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return false;
    }

    loop {
        let child = cursor.node();
        if matches!(
            child.kind(),
            "content" | "CharData" | "CData" | "element" | "ERROR"
        ) {
            return true;
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }

    false
}

fn build_line_doc_fragment(pieces: &[(usize, &str)]) -> Option<Fragment> {
    let mut fragment = Fragment {
        text: String::new(),
        source_offsets: Vec::new(),
    };

    for (comment_start, raw) in pieces {
        append_line_doc_piece(&mut fragment, *comment_start, raw)?;
    }

    Some(fragment)
}

fn append_line_doc_piece(fragment: &mut Fragment, comment_start: usize, raw: &str) -> Option<()> {
    let mut running_offset = 0;

    for raw_line in raw.split_inclusive('\n') {
        let (line, has_newline) = raw_line
            .strip_suffix('\n')
            .map(|line| (line, true))
            .unwrap_or((raw_line, false));
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let stripped = trimmed.strip_prefix("///")?;
        let leading_ws = stripped.len() - stripped.trim_start().len();
        let content = &stripped[leading_ws..];
        let content_start = comment_start + running_offset + indent + 3 + leading_ws;
        push_mapped_text(fragment, content, content_start);

        if has_newline {
            fragment.text.push('\n');
            fragment
                .source_offsets
                .push(Some(comment_start + running_offset + line.len()));
        }

        running_offset += raw_line.len();
    }

    Some(())
}

fn build_block_doc_fragment(comment_start: usize, raw: &str) -> Option<Fragment> {
    let inner = raw.strip_prefix("/**")?.strip_suffix("*/")?;
    let mut fragment = Fragment {
        text: String::new(),
        source_offsets: Vec::new(),
    };
    let mut running_offset = 3;

    for raw_line in inner.split_inclusive('\n') {
        let (line, has_newline) = raw_line
            .strip_suffix('\n')
            .map(|line| (line, true))
            .unwrap_or((raw_line, false));
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let without_star = trimmed.strip_prefix('*').unwrap_or(trimmed);
        let star_delta = usize::from(trimmed.starts_with('*'));
        let leading_ws = without_star.len() - without_star.trim_start().len();
        let content = &without_star[leading_ws..];
        let content_start = comment_start + running_offset + indent + star_delta + leading_ws;
        push_mapped_text(&mut fragment, content, content_start);

        if has_newline {
            fragment.text.push('\n');
            fragment
                .source_offsets
                .push(Some(comment_start + running_offset + line.len()));
        }

        running_offset += raw_line.len();
    }

    Some(fragment)
}

fn push_mapped_text(fragment: &mut Fragment, text: &str, start_offset: usize) {
    fragment.text.push_str(text);
    for (index, _) in text.bytes().enumerate() {
        fragment.source_offsets.push(Some(start_offset + index));
    }
}

fn extract_error_fallback(
    fragment: &Fragment,
    unit_id: usize,
    _next_unit_id: &mut usize,
) -> Option<Vec<TextSegment>> {
    let bytes = fragment.text.as_bytes();
    let mut segments = Vec::new();
    let mut index = 0;
    let mut inside_tag = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'<' {
            inside_tag = true;
            index += 1;
            continue;
        }
        if inside_tag {
            if byte == b'>' {
                inside_tag = false;
            }
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && bytes[index] != b'<' {
            index += 1;
        }

        let slice = &fragment.text[start..index];
        let leading_ws = slice.len() - slice.trim_start().len();
        let trailing_ws = slice.len() - slice.trim_end().len();
        let trimmed = slice.trim();
        if trimmed.is_empty() {
            continue;
        }

        let text_start = start + leading_ws;
        let offset = fragment.source_offsets[text_start]?;
        let text_end = index - trailing_ws;
        segments.push(TextSegment {
            text: fragment.text[text_start..text_end].to_string(),
            is_markup: false,
            offset,
            unit_id,
        });
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

fn supplement_missing_error_fallback(
    fragment: &Fragment,
    segments: &mut Vec<TextSegment>,
    unit_id: usize,
) {
    let mut next_unit_id = unit_id;
    let Some(fallback_segments) = extract_error_fallback(fragment, unit_id, &mut next_unit_id)
    else {
        return;
    };

    for fallback in fallback_segments {
        let already_covered = segments.iter().any(|segment| {
            let segment_end = segment.offset + segment.text.len();
            (segment.offset..segment_end).contains(&fallback.offset)
        });
        if !already_covered {
            segments.push(fallback);
        }
    }

    segments.sort_by_key(|segment| segment.offset);
}
