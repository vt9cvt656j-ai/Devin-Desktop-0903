//! Reading delimited text — CSV, TSV and their relatives — as a table rather than as characters.
//!
//! A `.csv` used to open in the code editor, which is technically correct and practically useless:
//! quoted fields containing commas, embedded newlines and a header row all render as noise, and
//! the one question you open a spreadsheet to answer — what are the columns and how many rows —
//! takes counting.
//!
//! Three things this gets right that a `split(',')` does not:
//!
//! * **Encoding.** A CSV exported from Excel on a Chinese Windows machine is GBK, not UTF-8.
//!   Decoding it as UTF-8 yields mojibake in every cell, which looks like corruption rather than
//!   a decoding choice. The BOM is honoured first, then a detector, and the answer is reported so
//!   the reader can see which was used.
//! * **Dialect.** Semicolon is the delimiter across most of Europe (where the comma is the decimal
//!   separator), and tab for `.tsv`. It is sniffed from the data, not assumed.
//! * **Quoting.** Real files contain `"Smith, John"` and fields with newlines inside them. Parsing
//!   is delegated to the `csv` crate rather than hand-rolled, because hand-rolled is where this
//!   goes wrong.

use serde::Serialize;
use std::path::Path;

/// Rows sent to the UI. The grid renders one element per cell, so this is a rendering bound; the
/// true row count is counted past it and reported separately.
const PREVIEW_ROWS: usize = 5_000;
/// Never read more than this off disk for a preview — a CSV can be tens of gigabytes.
const READ_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ColumnKind {
    Number,
    Date,
    Text,
}

#[derive(Serialize, Debug)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
}

#[derive(Serialize, Debug)]
pub struct TablePreview {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    /// Rows in the file, past the preview bound. A floor when `count_is_partial`.
    pub total_rows: u64,
    pub truncated: bool,
    /// True when the read limit stopped us before the end, so `total_rows` is a floor.
    pub count_is_partial: bool,
    /// Shown to the reader: they are the only one who can tell whether it guessed right.
    pub delimiter: String,
    pub encoding: String,
    pub has_header: bool,
    pub note: Option<String>,
}

/// BOM first — it is a statement, not a guess. Otherwise chardetng, which is what Firefox uses.
fn decode(bytes: &[u8]) -> (String, &'static str) {
    if let Some((encoding, skip)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[skip..]);
        return (text.into_owned(), encoding.name());
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (text, _, _) = encoding.decode(bytes);
    (text.into_owned(), encoding.name())
}

/// Pick the delimiter by consistency, not by frequency: the right one appears the same number of
/// times in every line. A comma that only shows up inside one quoted address would otherwise win
/// on raw count alone.
fn sniff_delimiter(sample: &str, path: &Path) -> u8 {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("tsv") || e.eq_ignore_ascii_case("tab"))
    {
        return b'\t';
    }
    let lines: Vec<&str> = sample.lines().filter(|l| !l.trim().is_empty()).take(20).collect();
    if lines.is_empty() {
        return b',';
    }
    let mut best = (b',', 0f64, 0usize);
    for candidate in [b',', b'\t', b';', b'|'] {
        let counts: Vec<usize> = lines
            .iter()
            .map(|line| {
                // Count only outside quotes, or a quoted "a,b" inflates the comma's score.
                let (mut n, mut in_quotes) = (0usize, false);
                for byte in line.bytes() {
                    match byte {
                        b'"' => in_quotes = !in_quotes,
                        b if b == candidate && !in_quotes => n += 1,
                        _ => {}
                    }
                }
                n
            })
            .collect();
        let first = counts[0];
        if first == 0 {
            continue;
        }
        let consistent = counts.iter().filter(|c| **c == first).count();
        let score = consistent as f64 / counts.len() as f64;
        // Prefer the more consistent delimiter; break ties on the wider table.
        if score > best.1 || (score == best.1 && first > best.2) {
            best = (candidate, score, first);
        }
    }
    best.0
}

fn looks_numeric(value: &str) -> bool {
    let v = value.trim().replace([',', ' ', '%'], "");
    if v.is_empty() {
        return false;
    }
    let v = v.strip_prefix(['$', '¥', '€', '£']).unwrap_or(&v);
    v.parse::<f64>().is_ok()
}

fn looks_like_date(value: &str) -> bool {
    let v = value.trim();
    if v.len() < 6 || v.len() > 32 {
        return false;
    }
    let digits = v.chars().filter(char::is_ascii_digit).count();
    let seps = v.matches(['-', '/', ':']).count();
    digits >= 4 && seps >= 2
}

fn classify(samples: &[String]) -> ColumnKind {
    let considered: Vec<&String> = samples.iter().filter(|s| !s.trim().is_empty()).take(50).collect();
    if considered.is_empty() {
        return ColumnKind::Text;
    }
    let numeric = considered.iter().filter(|s| looks_numeric(s)).count();
    if numeric * 10 >= considered.len() * 9 {
        return ColumnKind::Number;
    }
    let dates = considered.iter().filter(|s| looks_like_date(s)).count();
    if dates * 10 >= considered.len() * 9 {
        return ColumnKind::Date;
    }
    ColumnKind::Text
}

/// A first row is a header when it is all text, all non-empty, all distinct, and at least one
/// column below it is not text. A file of strings throughout is ambiguous, and there the answer is
/// "yes" because a spreadsheet almost always has one.
fn detect_header(first: &[String], rest: &[Vec<String>]) -> bool {
    if first.is_empty() || first.iter().any(|c| c.trim().is_empty()) {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    if !first.iter().all(|c| seen.insert(c.trim().to_lowercase())) {
        return false;
    }
    if first.iter().any(|c| looks_numeric(c)) {
        return false;
    }
    // Everything above is a reason to say NO. Having survived them — all cells non-empty, all
    // distinct, none numeric — the answer is yes. An earlier version also required a typed column
    // below and then OR'd it with true, which computed the check and threw it away; a file of
    // strings throughout is genuinely ambiguous, and "yes" is the right guess there because a
    // spreadsheet almost always has a header. Better to say so than to dress it as analysis.
    true
}

pub fn read_table(path: &Path, max_rows: Option<usize>) -> Result<TablePreview, String> {
    let preview_rows = max_rows.unwrap_or(PREVIEW_ROWS);
    let meta = std::fs::metadata(path).map_err(|e| format!("读取失败：{e}"))?;
    let read_len = meta.len().min(READ_LIMIT);
    let partial_read = meta.len() > READ_LIMIT;

    let bytes = {
        use std::io::Read;
        let file = std::fs::File::open(path).map_err(|e| format!("打开失败：{e}"))?;
        let mut buf = Vec::with_capacity(read_len as usize);
        file.take(read_len).read_to_end(&mut buf).map_err(|e| e.to_string())?;
        buf
    };
    let (text, encoding) = decode(&bytes);
    let delimiter = sniff_delimiter(&text[..text.len().min(64 * 1024)], path);

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true) // ragged rows are common in the wild and must not abort the read
        .has_headers(false)
        .from_reader(text.as_bytes());

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut total: u64 = 0;
    let mut malformed: u64 = 0;
    for record in reader.records() {
        match record {
            Ok(record) => {
                total += 1;
                if rows.len() < preview_rows + 1 {
                    rows.push(record.iter().map(|f| f.to_string()).collect());
                }
            }
            Err(_) => malformed += 1,
        }
    }
    if rows.is_empty() {
        return Err("这个文件里没有可解析的表格行。".into());
    }

    let has_header = detect_header(&rows[0], &rows[1..]);
    let header = if has_header { rows.remove(0) } else { Vec::new() };
    if has_header {
        total = total.saturating_sub(1);
    }

    let width = rows.iter().map(Vec::len).max().unwrap_or(header.len()).max(header.len());
    let columns = (0..width)
        .map(|i| {
            let samples: Vec<String> = rows.iter().filter_map(|r| r.get(i).cloned()).collect();
            Column {
                name: header
                    .get(i)
                    .map(|h| h.trim().to_string())
                    .filter(|h| !h.is_empty())
                    // Spreadsheet lettering for an unnamed column reads better than "column 3".
                    .unwrap_or_else(|| column_letter(i)),
                kind: classify(&samples),
            }
        })
        .collect();

    // Ragged rows are padded so the grid stays rectangular; short rows are a data fact, not a
    // rendering problem to solve with a shrug.
    for row in &mut rows {
        row.resize(width, String::new());
    }
    // From the file's own count, not the buffer's: pulling the header out shifted the buffer by
    // one, so comparing lengths reported a full preview as complete when it was not.
    let truncated = total > preview_rows as u64;
    rows.truncate(preview_rows);

    let mut notes = Vec::new();
    if partial_read {
        notes.push(format!(
            "文件超过 {}MB，只读取了前面一段，行数是「至少这么多」",
            READ_LIMIT / 1024 / 1024
        ));
    }
    if malformed > 0 {
        notes.push(format!("有 {malformed} 行无法解析，已跳过"));
    }

    Ok(TablePreview {
        columns,
        rows,
        total_rows: total,
        truncated: truncated || partial_read,
        count_is_partial: partial_read,
        delimiter: match delimiter {
            b'\t' => "Tab".into(),
            b';' => ";".into(),
            b'|' => "|".into(),
            _ => ",".into(),
        },
        encoding: encoding.to_string(),
        has_header,
        note: (!notes.is_empty()).then(|| notes.join("；")),
    })
}

fn column_letter(mut index: usize) -> String {
    let mut out = String::new();
    loop {
        out.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    out
}

#[tauri::command]
pub fn read_table_file(path: String, max_rows: Option<usize>) -> Result<TablePreview, String> {
    read_table(Path::new(&path), max_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("michael-tabular-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// The failure that makes a viewer worse than the text editor: splitting on every comma.
    #[test]
    fn a_quoted_field_containing_the_delimiter_stays_one_cell() {
        let path = temp("quoted.csv", b"name,city\n\"Smith, John\",\"Berlin\"\n\"a \"\"quoted\"\" b\",x\n");
        let t = read_table(&path, None).unwrap();
        assert!(t.has_header);
        assert_eq!(t.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["name", "city"]);
        assert_eq!(t.rows[0][0], "Smith, John", "a comma inside quotes is data, not a column break");
        assert_eq!(t.rows[1][0], "a \"quoted\" b", "and doubled quotes are one literal quote");
        assert_eq!(t.total_rows, 2, "the header is not a row of data");
    }

    /// Excel on a Chinese Windows machine writes GBK. Decoded as UTF-8 every cell is mojibake.
    #[test]
    fn a_gbk_export_is_decoded_as_gbk_not_as_broken_utf8() {
        let (encoded, _, _) = encoding_rs::GBK.encode("城市,人口\n北京,2189\n上海,2487\n");
        let path = temp("gbk.csv", &encoded);
        let t = read_table(&path, None).unwrap();
        assert_eq!(t.columns[0].name, "城市", "got {:?} with encoding {}", t.columns[0].name, t.encoding);
        assert_eq!(t.rows[0][0], "北京");
        assert!(!t.encoding.eq_ignore_ascii_case("UTF-8"), "encoding reported as {}", t.encoding);
    }

    /// Semicolon is the delimiter across most of Europe, where the comma is the decimal separator.
    #[test]
    fn a_european_semicolon_file_is_not_read_as_one_column() {
        let path = temp("euro.csv", b"product;price;qty\nwidget;1,50;3\ngadget;2,75;10\n");
        let t = read_table(&path, None).unwrap();
        assert_eq!(t.delimiter, ";");
        assert_eq!(t.columns.len(), 3);
        assert_eq!(t.rows[0][1], "1,50", "the comma here is a decimal point, not a separator");
    }

    #[test]
    fn columns_are_typed_so_numbers_can_be_aligned_and_dates_recognised() {
        let path = temp(
            "typed.csv",
            b"id,when,label,amount\n1,2026-01-02 10:00:00,alpha,$1200.50\n2,2026-01-03 11:30:00,beta,$980\n",
        );
        let t = read_table(&path, None).unwrap();
        let kinds: Vec<&ColumnKind> = t.columns.iter().map(|c| &c.kind).collect();
        assert_eq!(kinds, vec![&ColumnKind::Number, &ColumnKind::Date, &ColumnKind::Text, &ColumnKind::Number]);
    }

    /// The preview is bounded; the count is not. Reporting the cap as the total is the same bug
    /// the archive panel shipped with.
    #[test]
    fn the_row_count_describes_the_file_not_the_preview() {
        let mut data = String::from("n\n");
        for i in 0..1200 {
            data.push_str(&format!("{i}\n"));
        }
        let path = temp("many.csv", data.as_bytes());
        let t = read_table(&path, Some(100)).unwrap();
        assert_eq!(t.rows.len(), 100, "the preview stays bounded");
        assert_eq!(t.total_rows, 1200, "and the total still describes the file");
        assert!(t.truncated);
        assert!(!t.count_is_partial, "the whole file was read, so the count is exact");
    }

    #[test]
    fn ragged_rows_are_padded_rather_than_dropped() {
        let path = temp("ragged.csv", b"a,b,c\n1,2\n3,4,5,6\n");
        let t = read_table(&path, None).unwrap();
        assert_eq!(t.columns.len(), 4, "the widest row sets the width");
        assert_eq!(t.rows[0], vec!["1", "2", "", ""]);
        assert_eq!(t.rows[1], vec!["3", "4", "5", "6"]);
    }

    #[test]
    fn a_file_whose_first_row_is_data_keeps_that_row() {
        let path = temp("noheader.csv", b"1,2,3\n4,5,6\n");
        let t = read_table(&path, None).unwrap();
        assert!(!t.has_header, "all-numeric first row is data");
        assert_eq!(t.total_rows, 2);
        assert_eq!(t.columns[0].name, "A", "unnamed columns get spreadsheet letters");
    }
}

#[cfg(test)]
mod probe {
    /// MICHAEL_TABLE_PROBE=<file> cargo test --lib tabular::probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn probe_a_real_csv() {
        let Ok(path) = std::env::var("MICHAEL_TABLE_PROBE") else { return };
        match super::read_table(std::path::Path::new(&path), None) {
            Ok(t) => {
                eprintln!(
                    "OK rows={} cols={} delim={} enc={} header={} truncated={}",
                    t.total_rows, t.columns.len(), t.delimiter, t.encoding, t.has_header, t.truncated
                );
                eprintln!("   cols: {:?}", t.columns.iter().map(|c| format!("{}:{:?}", c.name, c.kind)).collect::<Vec<_>>());
                if let Some(r) = t.rows.first() { eprintln!("   row1: {r:?}"); }
            }
            Err(e) => eprintln!("ERR {e}"),
        }
    }
}
