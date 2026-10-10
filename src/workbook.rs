use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Workbook, Worksheet};
use std::path::Path;

#[derive(Clone, Default)]
pub struct Cell {
    pub raw: String,
}

pub struct Sheet {
    pub rows: usize,
    pub cols: usize,
    cells: Vec<Vec<Cell>>,
}

impl Sheet {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self { rows, cols, cells: vec![vec![Cell::default(); cols]; rows] }
    }

    pub fn get(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    pub fn set(&mut self, row: usize, col: usize, value: String) {
        if row >= self.rows || col >= self.cols { return; }
        self.cells[row][col].raw = value;
    }

    pub fn display(&self, row: usize, col: usize) -> String {
        let raw = &self.cells[row][col].raw;
        if let Some(formula) = raw.strip_prefix('=') {
            match evaluate_simple_formula(formula, self) {
                Some(v) => format_number(v),
                None => raw.clone(),
            }
        } else {
            raw.clone()
        }
    }
}

pub fn col_name(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 { break; }
        n = n / 26 - 1;
    }
    s
}

pub fn cell_name(row: usize, col: usize) -> String {
    format!("{}{}", col_name(col), row + 1)
}

fn format_number(v: f64) -> String {
    if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") }
}

fn cell_num(reference: &str, sheet: &Sheet) -> Option<f64> {
    let mut letters = String::new();
    let mut digits = String::new();
    for ch in reference.chars() {
        if ch.is_ascii_alphabetic() && digits.is_empty() { letters.push(ch.to_ascii_uppercase()); }
        else if ch.is_ascii_digit() { digits.push(ch); }
        else { return None; }
    }
    if letters.is_empty() || digits.is_empty() { return None; }
    let mut col = 0usize;
    for b in letters.bytes() { col = col * 26 + (b - b'A' + 1) as usize; }
    let row: usize = digits.parse().ok()?;
    let raw = &sheet.get(row.checked_sub(1)?, col.checked_sub(1)?).raw;
    if raw.starts_with('=') {
        evaluate_simple_formula(&raw[1..], sheet)
    } else {
        raw.parse().ok()
    }
}

fn evaluate_simple_formula(formula: &str, sheet: &Sheet) -> Option<f64> {
    for op in ['+', '-', '*', '/'] {
        // Find an operator that is not the first character.
        if let Some(i) = formula.char_indices().skip(1).find_map(|(i, c)| if c == op { Some(i) } else { None }) {
            let left = formula[..i].trim();
            let right = formula[i + 1..].trim();
            let a = cell_num(left, sheet).or_else(|| left.parse().ok())?;
            let b = cell_num(right, sheet).or_else(|| right.parse().ok())?;
            return match op {
                '+' => Some(a + b), '-' => Some(a - b), '*' => Some(a * b),
                '/' if b != 0.0 => Some(a / b), _ => None,
            };
        }
    }
    cell_num(formula.trim(), sheet).or_else(|| formula.trim().parse().ok())
}

pub fn load(path: &Path) -> Result<Sheet, String> {
    if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("csv")).unwrap_or(false) {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let rows: Vec<Vec<String>> = text.lines().map(|line| {
            line.split(',').map(|s| s.trim_matches('"').to_string()).collect()
        }).collect();
        let nr = rows.len().max(40);
        let nc = rows.iter().map(Vec::len).max().unwrap_or(16).max(16);
        let mut sheet = Sheet::new(nr, nc);
        for (r, row) in rows.iter().enumerate() {
            for (c, value) in row.iter().enumerate() { sheet.set(r, c, value.clone()); }
        }
        return Ok(sheet);
    }

    let mut workbook = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let range = workbook.worksheet_range_at(0).ok_or("В книге нет листов")?
        .map_err(|e| e.to_string())?;
    let (height, width) = range.get_size();
    let mut sheet = Sheet::new(height.max(40), width.max(16));
    for (pos, value) in range.cells() {
        let text = match value {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Float(n) => format_number(*n),
            Data::Int(n) => n.to_string(),
            Data::Bool(b) => b.to_string(),
            other => other.to_string(),
        };
        sheet.set(pos.0, pos.1, text);
    }
    Ok(sheet)
}

pub fn save(path: &Path, sheet: &Sheet) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet: &mut Worksheet = workbook.add_worksheet();
    worksheet.set_name("Лист1").map_err(|e| e.to_string())?;

    for r in 0..sheet.rows {
        for c in 0..sheet.cols {
            let raw = &sheet.get(r, c).raw;
            if raw.is_empty() { continue; }
            if let Some(formula) = raw.strip_prefix('=') {
                worksheet.write_formula(r as u32, c as u16, formula)
                    .map_err(|e| e.to_string())?;
            } else if let Ok(n) = raw.parse::<f64>() {
                worksheet.write_number(r as u32, c as u16, n)
                    .map_err(|e| e.to_string())?;
            } else {
                worksheet.write_string(r as u32, c as u16, raw)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    workbook.save(path).map_err(|e| e.to_string())
}
