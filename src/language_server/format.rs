use lsp_types::{
    Position, Range, TextDocumentIdentifier, FormattingOptions, TextEdit,
};

pub(crate) fn format_doc(doc: TextDocumentIdentifier, _options: FormattingOptions) -> Result<Vec<TextEdit>, String> {
    if doc.uri.scheme() != "file" {
        return Err("Invalid URI scheme".to_string());
    }

    let file_path = doc.uri.to_file_path().map_err(|_| "Invalid path")?;
    let file_contents = std::fs::read_to_string(&file_path).map_err(|_| "File read error")?;

    format(file_contents)
}

#[allow(dead_code)]
pub(crate) fn format(src: String) -> Result<Vec<TextEdit>, String> {
    let original = src;
    let formatted = crate::format::pretty(&original).map_err(|_| "Parse error")?;

    if original == formatted {
        Ok (vec![])
    } else {
        // Temporary solution - just replace the entire document text with
        // the formatted text in one go.
        // Better solution - compute edit path and send vec of smaller edits?

        let start_pos = Position { line: 0u64, character: 0u64 };
        let end_pos = get_final_position(&formatted);
        let whole_doc_range = Range { start: start_pos, end: end_pos };

        Ok (vec![TextEdit{ range: whole_doc_range, new_text: formatted }])
    }
}

#[allow(dead_code)]
fn get_final_position(text: &str) -> Position {
    let line_count = text.lines().fold(0u64, |acc, _| acc + 1u64);
    let last_line_index = text.rfind("\n").unwrap_or(0usize);
    let last_line_cols = text.len() - last_line_index;

    Position { line: line_count - 1u64, character: last_line_cols as u64 }
}