//! The segment grid: every occurrence of one segment type in a message, one
//! row each, the populated fields as columns — the forty OBX of an ORU read
//! as a table instead of forty tree nodes.
//!
//! Column names come from the catalogue MSH-12 selects; a column appears
//! only when at least one occurrence has a value in it, so a sparse segment
//! stays narrow. Long values are cut for the table (the editor has them in
//! full); coded values carry their table meaning, as in the tree.

use serde::Serialize;

use super::message::Hl7Message;
use super::schema;
use super::value_tables::coded;

/// Longest value a grid cell shows before cutting it.
pub const MAX_CELL_CHARS: usize = 120;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SegmentCount {
    pub segment_type: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GridColumn {
    pub position: usize,
    /// Field name from the catalogue, empty for a position it does not know
    /// (a Z-segment, or a field past the standard's end).
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GridCell {
    pub value: String,
    /// The value was longer than [`MAX_CELL_CHARS`] and has been cut.
    pub truncated: bool,
    pub code_desc: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GridRow {
    /// Index of the segment in the message (tree node `seg{N}`, editor line N+1).
    pub segment_idx: usize,
    /// One cell per column, in column order.
    pub cells: Vec<GridCell>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SegmentGrid {
    pub segment_type: String,
    pub segment_name: String,
    pub version: String,
    pub columns: Vec<GridColumn>,
    pub rows: Vec<GridRow>,
}

/// How many times each segment type occurs, in order of first appearance.
pub fn segment_counts(msg: &Hl7Message) -> Vec<SegmentCount> {
    let mut out: Vec<SegmentCount> = Vec::new();
    for seg in &msg.segments {
        match out.iter_mut().find(|c| c.segment_type == seg.segment_type) {
            Some(c) => c.count += 1,
            None => out.push(SegmentCount { segment_type: seg.segment_type.clone(), count: 1 }),
        }
    }
    out
}

pub fn segment_grid(msg: &Hl7Message, segment_type: &str) -> SegmentGrid {
    let catalogue = schema::for_declared(&msg.version);
    let occurrences: Vec<(usize, _)> = msg
        .segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.segment_type == segment_type)
        .collect();

    let mut positions: Vec<usize> = occurrences
        .iter()
        .flat_map(|(_, s)| s.fields.iter())
        .filter(|f| !f.span.as_str(&msg.raw).is_empty())
        .map(|f| f.position)
        .collect();
    positions.sort_unstable();
    positions.dedup();

    let columns = positions
        .iter()
        .map(|&p| {
            let spec = catalogue.field(segment_type, p);
            GridColumn {
                position: p,
                name: spec.map(|f| f.name.clone()).unwrap_or_default(),
                data_type: spec.map(|f| f.data_type.clone()).unwrap_or_default(),
            }
        })
        .collect();

    let rows = occurrences
        .iter()
        .map(|(idx, seg)| GridRow {
            segment_idx: *idx,
            cells: positions
                .iter()
                .map(|&p| match seg.fields.iter().find(|f| f.position == p) {
                    None => GridCell::default(),
                    Some(f) => {
                        let value = f.span.as_str(&msg.raw);
                        let (_, code_desc) =
                            coded(catalogue.table_for_field(segment_type, p), value, &msg.delimiters);
                        let cut = value.chars().count() > MAX_CELL_CHARS;
                        GridCell {
                            value: if cut { value.chars().take(MAX_CELL_CHARS).collect() } else { value.to_string() },
                            // Only a value actually cut here: the lexer's
                            // 100-byte fold flag put "…" after values shown whole.
                            truncated: cut,
                            code_desc,
                        }
                    }
                })
                .collect(),
        })
        .collect();

    SegmentGrid {
        segment_type: segment_type.to_string(),
        segment_name: catalogue.segment(segment_type).map(|s| s.name.clone()).unwrap_or_default(),
        version: msg.version.clone(),
        columns,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;

    const ORU: &str = "MSH|^~\\&|LIS|LAB|HIS|HOSP|20260101||ORU^R01|1|P|2.5\r\
        PID|1||123^^^H^MR||Doe^Jane||19800101|F\r\
        OBR|1|A|B|58410-2^CBC^LN\r\
        OBX|1|NM|718-7^Hemoglobin^LN||11.9|g/dL|12.0-16.0|L|||F\r\
        OBX|2|NM|777-3^Platelets^LN||245|10*3/uL|150-400|N|||F\r\
        NTE|1||comment\r\
        OBX|3|ST|X^Note^L||see comment||||||F\r";

    fn parse(s: &str) -> Hl7Message {
        Hl7Lexer::new().parse(s.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn counts_follow_first_appearance() {
        let counts = segment_counts(&parse(ORU));
        let v: Vec<_> = counts.iter().map(|c| (c.segment_type.as_str(), c.count)).collect();
        assert_eq!(v, vec![("MSH", 1), ("PID", 1), ("OBR", 1), ("OBX", 3), ("NTE", 1)]);
    }

    #[test]
    fn a_grid_has_a_row_per_occurrence_and_only_populated_columns() {
        let msg = parse(ORU);
        let g = segment_grid(&msg, "OBX");
        assert_eq!(g.segment_name, "Observation/Result");
        assert_eq!(g.rows.iter().map(|r| r.segment_idx).collect::<Vec<_>>(), vec![3, 4, 6]);
        let cols: Vec<_> = g.columns.iter().map(|c| c.position).collect();
        assert_eq!(cols, vec![1, 2, 3, 5, 6, 7, 8, 11], "OBX-4, 9, 10 are empty everywhere");
        assert_eq!(g.columns[0].name, "Set ID - OBX");
        let value_col = cols.iter().position(|&p| p == 5).unwrap();
        assert_eq!(g.rows[1].cells[value_col].value, "245");
        // the third OBX has no units: an empty OBX-6 cell, not a shifted row
        let units = cols.iter().position(|&p| p == 6).unwrap();
        assert_eq!(g.rows[2].cells[units].value, "");
        assert_eq!(g.rows.iter().all(|r| r.cells.len() == cols.len()), true);
    }

    #[test]
    fn coded_cells_carry_their_meaning() {
        let g = segment_grid(&parse(ORU), "OBX");
        let flag = g.columns.iter().position(|c| c.position == 8).unwrap();
        assert_eq!(g.rows[0].cells[flag].value, "L");
        assert!(g.rows[0].cells[flag].code_desc.as_deref().unwrap_or("").to_lowercase().contains("low"));
    }

    #[test]
    fn long_values_are_cut_and_flagged() {
        let long = "x".repeat(MAX_CELL_CHARS + 10);
        let msg = parse(&format!("MSH|^~\\&|A|B|C|D|20260101||ORU^R01|1|P|2.5\rOBX|1|ED|X||{}\r", long));
        let g = segment_grid(&msg, "OBX");
        let v = g.columns.iter().position(|c| c.position == 5).unwrap();
        assert!(g.rows[0].cells[v].truncated);
        assert_eq!(g.rows[0].cells[v].value.chars().count(), MAX_CELL_CHARS);

        // Longer than the fold threshold, shorter than a cell: shown whole.
        let msg = parse(&format!("MSH|^~\\&|A|B|C|D|20260101||ORU^R01|1|P|2.5\rOBX|1|ED|X||{}\r", "x".repeat(115)));
        let g = segment_grid(&msg, "OBX");
        assert!(!g.rows[0].cells[v].truncated);
    }

    #[test]
    fn an_absent_segment_gives_an_empty_grid() {
        let g = segment_grid(&parse(ORU), "ZZZ");
        assert!(g.rows.is_empty() && g.columns.is_empty());
    }
}
