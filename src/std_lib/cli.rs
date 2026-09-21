//! Terminal utilities, ANSI styling, and interactive CLI prompts (`amber:std/cli`).

/// Format tabular data into an ASCII table string
pub fn format_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if headers.is_empty() && rows.is_empty() {
        return String::new();
    }

    let num_cols = if !headers.is_empty() {
        headers.len()
    } else {
        rows.first().map(|r| r.len()).unwrap_or(0)
    };

    let mut col_widths = vec![0; num_cols];

    for (i, h) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(h.len());
    }

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    let mut out = String::new();

    // Top border
    out.push('┌');
    for (i, w) in col_widths.iter().enumerate() {
        out.push_str(&"─".repeat(*w + 2));
        if i + 1 < num_cols {
            out.push('┬');
        }
    }
    out.push_str("┐\n");

    // Header row
    if !headers.is_empty() {
        out.push('│');
        for (i, h) in headers.iter().enumerate() {
            out.push_str(&format!(" {:<width$} │", h, width = col_widths[i]));
        }
        out.push('\n');

        // Header separator
        out.push('├');
        for (i, w) in col_widths.iter().enumerate() {
            out.push_str(&"─".repeat(*w + 2));
            if i + 1 < num_cols {
                out.push('┼');
            }
        }
        out.push_str("┤\n");
    }

    // Data rows
    for row in rows {
        out.push('│');
        for i in 0..num_cols {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!(" {:<width$} │", cell, width = col_widths[i]));
        }
        out.push('\n');
    }

    // Bottom border
    out.push('└');
    for (i, w) in col_widths.iter().enumerate() {
        out.push_str(&"─".repeat(*w + 2));
        if i + 1 < num_cols {
            out.push('┴');
        }
    }
    out.push_str("┘\n");

    out
}
