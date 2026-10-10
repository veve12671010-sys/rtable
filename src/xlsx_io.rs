//! Чтение (xlsx/xlsm/xls/xlsb/ods) и запись (xlsx) файлов.
use crate::engine::{fmt_num, translate_formula, Pos, Sheet, Value};
use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Formula, Workbook};
use std::collections::HashMap;
use std::path::Path;

fn data_to_raw(d: &Data) -> Option<String> {
    match d {
        Data::Empty => None,
        Data::String(s) => {
            if s.starts_with('=') || s.starts_with('\'') {
                Some(format!("'{}", s))
            } else {
                Some(s.clone())
            }
        }
        Data::Float(f) => Some(fmt_num(*f)),
        Data::Int(i) => Some(i.to_string()),
        Data::Bool(b) => Some(if *b { "ИСТИНА".into() } else { "ЛОЖЬ".into() }),
        Data::DateTime(dt) => Some(fmt_num(dt.as_f64())),
        Data::DateTimeIso(s) | Data::DurationIso(s) => Some(s.clone()),
        Data::Error(e) => Some(format!("'{}", e)),
    }
}

pub fn load(path: &Path) -> Result<Vec<(String, Sheet)>, String> {
    let mut wb = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let names: Vec<String> = wb.sheet_names().to_vec();
    let mut out = vec![];
    for name in names {
        let range = wb.worksheet_range(&name).map_err(|e| e.to_string())?;
        let mut formulas: HashMap<Pos, String> = HashMap::new();
        if let Ok(fr) = wb.worksheet_formula(&name) {
            if let Some((sr, sc)) = fr.start() {
                for (r, c, f) in fr.cells() {
                    if !f.is_empty() {
                        let p = (r + sr as usize, c + sc as usize);
                        formulas.insert(p, format!("={}", translate_formula(f, false)));
                    }
                }
            }
        }
        let mut sh = Sheet::default();
        if let Some((sr, sc)) = range.start() {
            for (r, c, d) in range.cells() {
                let p = (r + sr as usize, c + sc as usize);
                if let Some(f) = formulas.remove(&p) {
                    sh.set_raw(p, f);
                } else if let Some(raw) = data_to_raw(d) {
                    sh.set_raw(p, raw);
                }
            }
        }
        for (p, f) in formulas {
            sh.set_raw(p, f);
        }
        sh.recalc();
        out.push((name, sh));
    }
    if out.is_empty() {
        out.push(("Лист1".to_string(), Sheet::default()));
    }
    Ok(out)
}

fn result_string(v: &Value) -> String {
    match v {
        Value::Num(n) => format!("{}", n),
        Value::Bool(b) => if *b { "TRUE".into() } else { "FALSE".into() },
        Value::Text(s) => s.clone(),
        Value::Empty => "0".into(),
        Value::Err(e) => match *e {
            "#ДЕЛ/0!" => "#DIV/0!".into(),
            "#ИМЯ?" => "#NAME?".into(),
            "#ССЫЛКА!" => "#REF!".into(),
            "#ЧИСЛО!" => "#NUM!".into(),
            _ => "#VALUE!".into(),
        },
    }
}

pub fn save(path: &Path, sheets: &[(String, Sheet)]) -> Result<(), String> {
    let mut wb = Workbook::new();
    for (name, sh) in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(|e| e.to_string())?;
        let mut keys: Vec<&Pos> = sh.cells.keys().collect();
        keys.sort();
        for &&(r, c) in &keys {
            let (row, col) = (r as u32, c as u16);
            let raw = sh.raw((r, c));
            let v = sh.value((r, c));
            if let Some(f) = raw.strip_prefix('=') {
                let formula = Formula::new(translate_formula(f, true)).set_result(result_string(&v));
                ws.write_formula(row, col, formula).map_err(|e| e.to_string())?;
            } else {
                match v {
                    Value::Num(n) => ws.write_number(row, col, n).map(|_| ()),
                    Value::Bool(b) => ws.write_boolean(row, col, b).map(|_| ()),
                    Value::Text(s) => ws.write_string(row, col, &s).map(|_| ()),
                    _ => Ok(()),
                }
                .map_err(|e| e.to_string())?;
            }
        }
    }
    wb.save(path).map_err(|e| e.to_string())
}
