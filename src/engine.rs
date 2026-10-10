//! Движок таблицы: разбор и вычисление формул с русскими именами функций.
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub type Pos = (usize, usize); // (строка, столбец), с нуля
type R<T> = Result<T, &'static str>;

const E_DIV: &str = "#ДЕЛ/0!";
const E_VAL: &str = "#ЗНАЧ!";
const E_NAME: &str = "#ИМЯ?";
const E_REF: &str = "#ССЫЛКА!";
const E_NUM: &str = "#ЧИСЛО!";
const E_CYC: &str = "#ЦИКЛ!";
const E_SYN: &str = "#ОШИБКА!";

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Empty,
    Num(f64),
    Text(String),
    Bool(bool),
    Err(&'static str),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Empty => String::new(),
            Value::Num(n) => fmt_num(*n),
            Value::Text(s) => s.clone(),
            Value::Bool(b) => if *b { "ИСТИНА".into() } else { "ЛОЖЬ".into() },
            Value::Err(e) => (*e).to_string(),
        }
    }
}

pub fn fmt_num(n: f64) -> String {
    if !n.is_finite() {
        return E_NUM.into();
    }
    if n == 0.0 {
        return "0".into();
    }
    let a = n.abs();
    let r = format!("{:.14e}", n).parse::<f64>().unwrap_or(n);
    let s = if a >= 1e15 || a < 1e-9 { format!("{:e}", r) } else { format!("{}", r) };
    s.replace('.', ",")
}

pub fn col_name(c: usize) -> String {
    let mut c = c + 1;
    let mut s = Vec::new();
    while c > 0 {
        let m = (c - 1) % 26;
        s.push((b'A' + m as u8) as char);
        c = (c - 1) / 26;
    }
    s.iter().rev().collect()
}

pub fn cell_name(p: Pos) -> String {
    format!("{}{}", col_name(p.1), p.0 + 1)
}

fn parse_cell_ref(s: &str) -> Option<Pos> {
    let s: String = s.chars().filter(|&c| c != '$').collect();
    let letters: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let digits = &s[letters.len()..];
    if letters.is_empty() || letters.len() > 3 || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut col = 0usize;
    for ch in letters.chars() {
        col = col * 26 + (ch.to_ascii_uppercase() as u8 - b'A') as usize + 1;
    }
    let row: usize = digits.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, col - 1))
}

// ---------- имена функций ----------

/// (русское имя как в LibreOffice/Excel, английское имя как в файлах xlsx)
pub const FUNCS: &[(&str, &str)] = &[
    ("СУММ", "SUM"), ("СРЗНАЧ", "AVERAGE"), ("МИН", "MIN"), ("МАКС", "MAX"),
    ("СЧЁТ", "COUNT"), ("СЧЁТЗ", "COUNTA"), ("СЧЁТЕСЛИ", "COUNTIF"), ("СУММЕСЛИ", "SUMIF"),
    ("ПРОИЗВЕД", "PRODUCT"), ("ЕСЛИ", "IF"), ("ЕСЛИОШИБКА", "IFERROR"),
    ("И", "AND"), ("ИЛИ", "OR"), ("НЕ", "NOT"),
    ("ОКРУГЛ", "ROUND"), ("ОКРУГЛВВЕРХ", "ROUNDUP"), ("ОКРУГЛВНИЗ", "ROUNDDOWN"),
    ("ABS", "ABS"), ("КОРЕНЬ", "SQRT"), ("СТЕПЕНЬ", "POWER"), ("ОСТАТ", "MOD"),
    ("ЦЕЛОЕ", "INT"), ("ПИ", "PI"),
    ("ДЛСТР", "LEN"), ("СЦЕПИТЬ", "CONCATENATE"), ("ПРОПИСН", "UPPER"), ("СТРОЧН", "LOWER"),
    ("ЛЕВСИМВ", "LEFT"), ("ПРАВСИМВ", "RIGHT"),
];

fn norm(s: &str) -> String {
    s.to_uppercase().replace('Ё', "Е")
}

fn canon(name: &str) -> Option<&'static str> {
    let n = norm(name);
    FUNCS.iter().find(|(ru, en)| norm(ru) == n || *en == n).map(|(_, en)| *en)
}

fn ru_of(en: &str) -> &'static str {
    FUNCS.iter().find(|(_, e)| *e == en).map(|(r, _)| *r).unwrap_or("")
}

/// Перевод формулы между «русским» видом (СУММ(A1;2,5)) и файловым (SUM(A1,2.5)).
pub fn translate_formula(f: &str, to_en: bool) -> String {
    let ch: Vec<char> = f.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < ch.len() {
        let c = ch[i];
        if c == '"' {
            out.push(c);
            i += 1;
            while i < ch.len() {
                out.push(ch[i]);
                if ch[i] == '"' {
                    if i + 1 < ch.len() && ch[i + 1] == '"' {
                        out.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let st = i;
            while i < ch.len() && (ch[i].is_alphanumeric() || ch[i] == '_' || ch[i] == '$') {
                i += 1;
            }
            let word: String = ch[st..i].iter().collect();
            if i < ch.len() && ch[i] == '(' {
                match canon(&word) {
                    Some(en) => out.push_str(if to_en { en } else { ru_of(en) }),
                    None => out.push_str(&word),
                }
            } else {
                let n = norm(&word);
                if to_en && n == "ИСТИНА" {
                    out.push_str("TRUE");
                } else if to_en && n == "ЛОЖЬ" {
                    out.push_str("FALSE");
                } else if !to_en && n == "TRUE" {
                    out.push_str("ИСТИНА");
                } else if !to_en && n == "FALSE" {
                    out.push_str("ЛОЖЬ");
                } else {
                    out.push_str(&word);
                }
            }
            continue;
        }
        if c == ';' {
            out.push(if to_en { ',' } else { ';' });
        } else if c == ',' {
            if to_en {
                let prev = i > 0 && ch[i - 1].is_ascii_digit();
                let next = i + 1 < ch.len() && ch[i + 1].is_ascii_digit();
                out.push(if prev && next { '.' } else { ',' });
            } else {
                out.push(';');
            }
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

// ---------- лексер ----------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
    LP,
    RP,
    Sep,
    Colon,
    Pct,
}

fn lex(src: &str) -> R<Vec<Tok>> {
    let ch: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = vec![];
    while i < ch.len() {
        let c = ch[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < ch.len() && ch[i + 1].is_ascii_digit()) {
            let st = i;
            while i < ch.len() && ch[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < ch.len() && (ch[i] == '.' || ch[i] == ',') && ch[i + 1].is_ascii_digit() {
                i += 1;
                while i < ch.len() && ch[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < ch.len() && (ch[i] == 'e' || ch[i] == 'E') {
                let mut j = i + 1;
                if j < ch.len() && (ch[j] == '+' || ch[j] == '-') {
                    j += 1;
                }
                if j < ch.len() && ch[j].is_ascii_digit() {
                    while j < ch.len() && ch[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let s: String = ch[st..i].iter().map(|&x| if x == ',' { '.' } else { x }).collect();
            out.push(Tok::Num(s.parse().map_err(|_| E_SYN)?));
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                if i >= ch.len() {
                    return Err(E_SYN);
                }
                if ch[i] == '"' {
                    if i + 1 < ch.len() && ch[i + 1] == '"' {
                        s.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                s.push(ch[i]);
                i += 1;
            }
            out.push(Tok::Str(s));
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let st = i;
            while i < ch.len() && (ch[i].is_alphanumeric() || ch[i] == '_' || ch[i] == '$') {
                i += 1;
            }
            out.push(Tok::Ident(ch[st..i].iter().collect()));
            continue;
        }
        i += 1;
        out.push(match c {
            '(' => Tok::LP,
            ')' => Tok::RP,
            ';' | ',' => Tok::Sep,
            ':' => Tok::Colon,
            '%' => Tok::Pct,
            '+' => Tok::Op("+"),
            '-' => Tok::Op("-"),
            '*' => Tok::Op("*"),
            '/' => Tok::Op("/"),
            '^' => Tok::Op("^"),
            '&' => Tok::Op("&"),
            '=' => Tok::Op("="),
            '<' => {
                if i < ch.len() && ch[i] == '=' {
                    i += 1;
                    Tok::Op("<=")
                } else if i < ch.len() && ch[i] == '>' {
                    i += 1;
                    Tok::Op("<>")
                } else {
                    Tok::Op("<")
                }
            }
            '>' => {
                if i < ch.len() && ch[i] == '=' {
                    i += 1;
                    Tok::Op(">=")
                } else {
                    Tok::Op(">")
                }
            }
            _ => return Err(E_SYN),
        });
    }
    Ok(out)
}

// ---------- парсер ----------

#[derive(Clone, Debug)]
enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Cell(Pos),
    Range(Pos, Pos),
    Neg(Box<Expr>),
    Pct(Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Func(String, Vec<Expr>),
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.t.get(self.i).cloned();
        self.i += 1;
        t
    }
    fn binary(&mut self, ops: &[&str], sub: fn(&mut Parser) -> R<Expr>) -> R<Expr> {
        let mut l = sub(self)?;
        while let Some(Tok::Op(o)) = self.peek().cloned() {
            if ops.contains(&o) {
                self.i += 1;
                let r = sub(self)?;
                l = Expr::Bin(o, Box::new(l), Box::new(r));
            } else {
                break;
            }
        }
        Ok(l)
    }
    fn expr(&mut self) -> R<Expr> {
        self.binary(&["=", "<>", "<", ">", "<=", ">="], Parser::concat)
    }
    fn concat(&mut self) -> R<Expr> {
        self.binary(&["&"], Parser::add)
    }
    fn add(&mut self) -> R<Expr> {
        self.binary(&["+", "-"], Parser::mul)
    }
    fn mul(&mut self) -> R<Expr> {
        self.binary(&["*", "/"], Parser::pow)
    }
    fn pow(&mut self) -> R<Expr> {
        self.binary(&["^"], Parser::unary)
    }
    fn unary(&mut self) -> R<Expr> {
        match self.peek() {
            Some(Tok::Op("-")) => {
                self.i += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Op("+")) => {
                self.i += 1;
                self.unary()
            }
            _ => {
                let mut e = self.primary()?;
                while matches!(self.peek(), Some(Tok::Pct)) {
                    self.i += 1;
                    e = Expr::Pct(Box::new(e));
                }
                Ok(e)
            }
        }
    }
    fn primary(&mut self) -> R<Expr> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Str(s)) => Ok(Expr::Str(s)),
            Some(Tok::LP) => {
                let e = self.expr()?;
                match self.next() {
                    Some(Tok::RP) => Ok(e),
                    _ => Err(E_SYN),
                }
            }
            Some(Tok::Ident(name)) => {
                if matches!(self.peek(), Some(Tok::LP)) {
                    self.i += 1;
                    let mut args = vec![];
                    if matches!(self.peek(), Some(Tok::RP)) {
                        self.i += 1;
                    } else {
                        loop {
                            args.push(self.expr()?);
                            match self.next() {
                                Some(Tok::Sep) => continue,
                                Some(Tok::RP) => break,
                                _ => return Err(E_SYN),
                            }
                        }
                    }
                    return Ok(Expr::Func(name, args));
                }
                let up = norm(&name);
                if up == "ИСТИНА" || up == "TRUE" {
                    return Ok(Expr::Bool(true));
                }
                if up == "ЛОЖЬ" || up == "FALSE" {
                    return Ok(Expr::Bool(false));
                }
                let p = parse_cell_ref(&name).ok_or(E_NAME)?;
                if matches!(self.peek(), Some(Tok::Colon)) {
                    self.i += 1;
                    match self.next() {
                        Some(Tok::Ident(n2)) => {
                            let q = parse_cell_ref(&n2).ok_or(E_REF)?;
                            Ok(Expr::Range(p, q))
                        }
                        _ => Err(E_SYN),
                    }
                } else {
                    Ok(Expr::Cell(p))
                }
            }
            _ => Err(E_SYN),
        }
    }
}

fn parse(src: &str) -> R<Expr> {
    let mut p = Parser { t: lex(src)?, i: 0 };
    let e = p.expr()?;
    if p.i != p.t.len() {
        return Err(E_SYN);
    }
    Ok(e)
}

// ---------- преобразования значений ----------

fn parse_number(t: &str) -> Option<f64> {
    let t = t.trim();
    let (body, pct) = match t.strip_suffix('%') {
        Some(b) => (b.trim(), true),
        None => (t, false),
    };
    let b = body.replace(',', ".");
    if b.is_empty() || !b.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')) {
        return None;
    }
    let n: f64 = b.parse().ok()?;
    Some(if pct { n / 100.0 } else { n })
}

fn parse_literal(s: &str) -> Value {
    if let Some(rest) = s.strip_prefix('\'') {
        return Value::Text(rest.to_string());
    }
    let t = s.trim();
    let up = norm(t);
    if up == "ИСТИНА" || up == "TRUE" {
        return Value::Bool(true);
    }
    if up == "ЛОЖЬ" || up == "FALSE" {
        return Value::Bool(false);
    }
    match parse_number(t) {
        Some(n) => Value::Num(n),
        None => Value::Text(s.to_string()),
    }
}

fn num(v: &Value) -> R<f64> {
    match v {
        Value::Empty => Ok(0.0),
        Value::Num(n) => Ok(*n),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        Value::Text(s) => parse_number(s).ok_or(E_VAL),
        Value::Err(e) => Err(e),
    }
}

fn text(v: &Value) -> String {
    v.display()
}

fn truthy(v: &Value) -> R<bool> {
    match v {
        Value::Empty => Ok(false),
        Value::Bool(b) => Ok(*b),
        Value::Num(n) => Ok(*n != 0.0),
        Value::Text(s) => match norm(s.trim()).as_str() {
            "ИСТИНА" | "TRUE" => Ok(true),
            "ЛОЖЬ" | "FALSE" => Ok(false),
            _ => Err(E_VAL),
        },
        Value::Err(e) => Err(e),
    }
}

fn cmp_values(l: &Value, r: &Value) -> Ordering {
    use Value::*;
    fn rank(v: &Value) -> u8 {
        match v {
            Num(_) | Empty => 0,
            Text(_) => 1,
            Bool(_) => 2,
            Err(_) => 3,
        }
    }
    match (l, r) {
        (Num(a), Num(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Text(a), Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (Bool(a), Bool(b)) => a.cmp(b),
        (Empty, Empty) => Ordering::Equal,
        (Empty, Num(b)) => 0.0f64.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Num(a), Empty) => a.partial_cmp(&0.0).unwrap_or(Ordering::Equal),
        (Empty, Text(b)) => "".cmp(b.to_lowercase().as_str()),
        (Text(a), Empty) => a.to_lowercase().as_str().cmp(""),
        _ => rank(l).cmp(&rank(r)),
    }
}

fn fix(y: f64) -> f64 {
    (y * 1e9).round() / 1e9
}

fn round_to(x: f64, d: i32, mode: &str) -> f64 {
    let m = 10f64.powi(d);
    let y = fix(x * m);
    let r = match mode {
        "ROUNDUP" => y.abs().ceil() * y.signum(),
        "ROUNDDOWN" => y.abs().floor() * y.signum(),
        _ => y.round(),
    };
    r / m
}

/// Условие для СЧЁТЕСЛИ/СУММЕСЛИ.
fn matches_crit(v: &Value, crit: &Value) -> bool {
    let (op, operand): (&str, Value) = match crit {
        Value::Text(s) => {
            let t = s.trim();
            let (op, rest) = ["<=", ">=", "<>", "<", ">", "="]
                .iter()
                .find(|o| t.starts_with(**o))
                .map(|o| (*o, t[o.len()..].trim()))
                .unwrap_or(("=", t));
            (op, parse_literal(rest))
        }
        other => ("=", other.clone()),
    };
    if matches!(v, Value::Empty) {
        return false;
    }
    // числа сравниваем только с числами, текст — с текстом
    let same_kind = matches!((v, &operand), (Value::Num(_), Value::Num(_)) | (Value::Text(_), Value::Text(_)) | (Value::Bool(_), Value::Bool(_)));
    if !same_kind {
        return op == "<>";
    }
    let o = cmp_values(v, &operand);
    match op {
        "=" => o == Ordering::Equal,
        "<>" => o != Ordering::Equal,
        "<" => o == Ordering::Less,
        ">" => o == Ordering::Greater,
        "<=" => o != Ordering::Greater,
        ">=" => o != Ordering::Less,
        _ => false,
    }
}

// ---------- лист ----------

#[derive(Default, Clone)]
pub struct Sheet {
    pub cells: HashMap<Pos, String>,
    cache: HashMap<Pos, Value>,
}

enum Arg {
    Scalar(Value),
    Range(Vec<Value>),
}

type Cache = HashMap<Pos, Value>;
type Stack = HashSet<Pos>;

impl Sheet {
    pub fn raw(&self, p: Pos) -> &str {
        self.cells.get(&p).map(|s| s.as_str()).unwrap_or("")
    }
    pub fn value(&self, p: Pos) -> Value {
        self.cache.get(&p).cloned().unwrap_or(Value::Empty)
    }
    /// Записать ячейку и пересчитать лист.
    pub fn set(&mut self, p: Pos, s: String) {
        self.set_raw(p, s);
        self.recalc();
    }
    /// Записать без пересчёта (для массовой загрузки; потом вызвать recalc).
    pub fn set_raw(&mut self, p: Pos, s: String) {
        if s.is_empty() {
            self.cells.remove(&p);
        } else {
            self.cells.insert(p, s);
        }
    }
    pub fn extent(&self) -> (usize, usize) {
        let r = self.cells.keys().map(|k| k.0 + 1).max().unwrap_or(0);
        let c = self.cells.keys().map(|k| k.1 + 1).max().unwrap_or(0);
        (r, c)
    }
    pub fn recalc(&mut self) {
        let mut cache = Cache::new();
        let mut stack = Stack::new();
        let mut keys: Vec<Pos> = self.cells.keys().cloned().collect();
        keys.sort(); // сверху вниз: цепочки зависимостей остаются неглубокими
        for k in keys {
            self.cell(k, &mut cache, &mut stack);
        }
        self.cache = cache;
    }

    fn cell(&self, p: Pos, cache: &mut Cache, stack: &mut Stack) -> Value {
        if let Some(v) = cache.get(&p) {
            return v.clone();
        }
        let raw = match self.cells.get(&p) {
            Some(s) => s,
            None => return Value::Empty,
        };
        if !stack.insert(p) {
            return Value::Err(E_CYC);
        }
        let v = if let Some(f) = raw.strip_prefix('=') {
            match parse(f) {
                Ok(e) => match self.eval(&e, cache, stack) {
                    Value::Empty => Value::Num(0.0),
                    v => v,
                },
                Err(er) => Value::Err(er),
            }
        } else {
            parse_literal(raw)
        };
        stack.remove(&p);
        cache.insert(p, v.clone());
        v
    }

    fn range_bounds(p: Pos, q: Pos) -> (usize, usize, usize, usize) {
        (p.0.min(q.0), p.0.max(q.0), p.1.min(q.1), p.1.max(q.1))
    }

    /// Непустые ячейки диапазона (позиция, значение).
    fn range_cells(&self, p: Pos, q: Pos, cache: &mut Cache, stack: &mut Stack) -> Vec<(Pos, Value)> {
        let (r0, r1, c0, c1) = Self::range_bounds(p, q);
        let area = (r1 - r0 + 1).saturating_mul(c1 - c0 + 1);
        let mut keys: Vec<Pos> = if area <= self.cells.len() {
            let mut v = vec![];
            for r in r0..=r1 {
                for c in c0..=c1 {
                    if self.cells.contains_key(&(r, c)) {
                        v.push((r, c));
                    }
                }
            }
            v
        } else {
            self.cells.keys().cloned().filter(|k| k.0 >= r0 && k.0 <= r1 && k.1 >= c0 && k.1 <= c1).collect()
        };
        keys.sort();
        keys.into_iter().map(|k| (k, self.cell(k, cache, stack))).collect()
    }

    fn eval(&self, e: &Expr, cache: &mut Cache, stack: &mut Stack) -> Value {
        match e {
            Expr::Num(n) => Value::Num(*n),
            Expr::Str(s) => Value::Text(s.clone()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Cell(p) => self.cell(*p, cache, stack),
            Expr::Range(..) => Value::Err(E_VAL),
            Expr::Neg(x) => match num(&self.eval(x, cache, stack)) {
                Ok(n) => Value::Num(-n),
                Err(e) => Value::Err(e),
            },
            Expr::Pct(x) => match num(&self.eval(x, cache, stack)) {
                Ok(n) => Value::Num(n / 100.0),
                Err(e) => Value::Err(e),
            },
            Expr::Bin(op, a, b) => {
                let l = self.eval(a, cache, stack);
                let r = self.eval(b, cache, stack);
                if let Value::Err(e) = l {
                    return Value::Err(e);
                }
                if let Value::Err(e) = r {
                    return Value::Err(e);
                }
                match *op {
                    "&" => Value::Text(text(&l) + &text(&r)),
                    "+" | "-" | "*" | "/" | "^" => {
                        let (x, y) = match (num(&l), num(&r)) {
                            (Ok(x), Ok(y)) => (x, y),
                            (Err(e), _) | (_, Err(e)) => return Value::Err(e),
                        };
                        let res = match *op {
                            "+" => x + y,
                            "-" => x - y,
                            "*" => x * y,
                            "/" => {
                                if y == 0.0 {
                                    return Value::Err(E_DIV);
                                }
                                x / y
                            }
                            _ => x.powf(y),
                        };
                        if res.is_finite() { Value::Num(res) } else { Value::Err(E_NUM) }
                    }
                    _ => {
                        let o = cmp_values(&l, &r);
                        Value::Bool(match *op {
                            "=" => o == Ordering::Equal,
                            "<>" => o != Ordering::Equal,
                            "<" => o == Ordering::Less,
                            ">" => o == Ordering::Greater,
                            "<=" => o != Ordering::Greater,
                            _ => o != Ordering::Less,
                        })
                    }
                }
            }
            Expr::Func(name, args) => self.call(name, args, cache, stack),
        }
    }

    fn collect_args(&self, a: &[Expr], cache: &mut Cache, stack: &mut Stack) -> Vec<Arg> {
        a.iter()
            .map(|e| match e {
                Expr::Range(p, q) => Arg::Range(self.range_cells(*p, *q, cache, stack).into_iter().map(|(_, v)| v).collect()),
                _ => Arg::Scalar(self.eval(e, cache, stack)),
            })
            .collect()
    }

    fn call(&self, name: &str, a: &[Expr], cache: &mut Cache, stack: &mut Stack) -> Value {
        let f = match canon(name) {
            Some(f) => f,
            None => return Value::Err(E_NAME),
        };
        // функции с ленивыми аргументами
        match f {
            "IF" => {
                if a.len() < 2 || a.len() > 3 {
                    return Value::Err(E_VAL);
                }
                let c = self.eval(&a[0], cache, stack);
                return match truthy(&c) {
                    Err(e) => Value::Err(e),
                    Ok(true) => self.eval(&a[1], cache, stack),
                    Ok(false) => {
                        if a.len() == 3 { self.eval(&a[2], cache, stack) } else { Value::Bool(false) }
                    }
                };
            }
            "IFERROR" => {
                if a.len() != 2 {
                    return Value::Err(E_VAL);
                }
                let v = self.eval(&a[0], cache, stack);
                return if matches!(v, Value::Err(_)) { self.eval(&a[1], cache, stack) } else { v };
            }
            "COUNTIF" | "SUMIF" => return self.countif(f, a, cache, stack),
            _ => {}
        }

        let args = self.collect_args(a, cache, stack);

        match f {
            "SUM" | "AVERAGE" | "MIN" | "MAX" | "PRODUCT" | "COUNT" | "COUNTA" => {
                let mut nums: Vec<f64> = vec![];
                let mut counta = 0usize;
                for ar in &args {
                    match ar {
                        Arg::Scalar(v) => match v {
                            Value::Err(e) => return Value::Err(e),
                            Value::Empty => {}
                            other => {
                                counta += 1;
                                match num(other) {
                                    Ok(n) => nums.push(n),
                                    Err(e) => {
                                        if f != "COUNT" && f != "COUNTA" {
                                            return Value::Err(e);
                                        }
                                    }
                                }
                            }
                        },
                        Arg::Range(vs) => {
                            for v in vs {
                                match v {
                                    Value::Err(e) => return Value::Err(e),
                                    Value::Num(n) => {
                                        nums.push(*n);
                                        counta += 1;
                                    }
                                    Value::Empty => {}
                                    _ => counta += 1,
                                }
                            }
                        }
                    }
                }
                let sum: f64 = nums.iter().sum();
                return match f {
                    "SUM" => Value::Num(sum),
                    "PRODUCT" => Value::Num(if nums.is_empty() { 0.0 } else { nums.iter().product() }),
                    "COUNT" => Value::Num(nums.len() as f64),
                    "COUNTA" => Value::Num(counta as f64),
                    "AVERAGE" => {
                        if nums.is_empty() { Value::Err(E_DIV) } else { Value::Num(sum / nums.len() as f64) }
                    }
                    "MIN" => Value::Num(if nums.is_empty() { 0.0 } else { nums.iter().cloned().fold(f64::INFINITY, f64::min) }),
                    _ => Value::Num(if nums.is_empty() { 0.0 } else { nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max) }),
                };
            }
            "AND" | "OR" => {
                let mut bools = vec![];
                for ar in &args {
                    match ar {
                        Arg::Scalar(v) => match v {
                            Value::Empty => {}
                            other => match truthy(other) {
                                Ok(b) => bools.push(b),
                                Err(e) => return Value::Err(e),
                            },
                        },
                        Arg::Range(vs) => {
                            for v in vs {
                                match v {
                                    Value::Bool(b) => bools.push(*b),
                                    Value::Num(n) => bools.push(*n != 0.0),
                                    Value::Err(e) => return Value::Err(e),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if bools.is_empty() {
                    return Value::Err(E_VAL);
                }
                return Value::Bool(if f == "AND" { bools.iter().all(|b| *b) } else { bools.iter().any(|b| *b) });
            }
            "CONCATENATE" => {
                let mut s = String::new();
                for ar in &args {
                    match ar {
                        Arg::Scalar(Value::Err(e)) => return Value::Err(e),
                        Arg::Scalar(v) => s.push_str(&text(v)),
                        Arg::Range(vs) => {
                            for v in vs {
                                if let Value::Err(e) = v {
                                    return Value::Err(e);
                                }
                                s.push_str(&text(v));
                            }
                        }
                    }
                }
                return Value::Text(s);
            }
            _ => {}
        }

        // функции со скалярными аргументами
        let mut sv: Vec<Value> = vec![];
        for ar in &args {
            match ar {
                Arg::Scalar(v) => sv.push(v.clone()),
                Arg::Range(_) => return Value::Err(E_VAL),
            }
        }
        if let Some(Value::Err(e)) = sv.iter().find(|v| matches!(v, Value::Err(_))) {
            return Value::Err(e);
        }
        let n = |i: usize| -> R<f64> { sv.get(i).ok_or(E_VAL).and_then(num) };
        let res: R<Value> = (|| -> R<Value> {
            Ok(match (f, sv.len()) {
                ("ABS", 1) => Value::Num(n(0)?.abs()),
                ("SQRT", 1) => {
                    let x = n(0)?;
                    if x < 0.0 {
                        return Err(E_NUM);
                    }
                    Value::Num(x.sqrt())
                }
                ("INT", 1) => Value::Num(n(0)?.floor()),
                ("PI", 0) => Value::Num(std::f64::consts::PI),
                ("NOT", 1) => Value::Bool(!truthy(&sv[0])?),
                ("POWER", 2) => {
                    let r = n(0)?.powf(n(1)?);
                    if r.is_finite() { Value::Num(r) } else { return Err(E_NUM) }
                }
                ("MOD", 2) => {
                    let (x, y) = (n(0)?, n(1)?);
                    if y == 0.0 {
                        return Err(E_DIV);
                    }
                    Value::Num(x - y * (x / y).floor())
                }
                ("ROUND" | "ROUNDUP" | "ROUNDDOWN", 1 | 2) => {
                    let d = if sv.len() == 2 { n(1)?.trunc() as i32 } else { 0 };
                    Value::Num(round_to(n(0)?, d, f))
                }
                ("LEN", 1) => Value::Num(text(&sv[0]).chars().count() as f64),
                ("UPPER", 1) => Value::Text(text(&sv[0]).to_uppercase()),
                ("LOWER", 1) => Value::Text(text(&sv[0]).to_lowercase()),
                ("LEFT" | "RIGHT", 1 | 2) => {
                    let s = text(&sv[0]);
                    let k = if sv.len() == 2 { n(1)? } else { 1.0 };
                    if k < 0.0 {
                        return Err(E_VAL);
                    }
                    let cnt = s.chars().count();
                    let k = (k as usize).min(cnt);
                    Value::Text(if f == "LEFT" { s.chars().take(k).collect() } else { s.chars().skip(cnt - k).collect() })
                }
                _ => return Err(E_VAL),
            })
        })();
        res.unwrap_or_else(Value::Err)
    }

    fn countif(&self, f: &str, a: &[Expr], cache: &mut Cache, stack: &mut Stack) -> Value {
        if a.len() < 2 || (f == "SUMIF" && a.len() > 3) || (f == "COUNTIF" && a.len() != 2) {
            return Value::Err(E_VAL);
        }
        let (p, q) = match &a[0] {
            Expr::Range(p, q) => (*p, *q),
            _ => return Value::Err(E_VAL),
        };
        let crit = self.eval(&a[1], cache, stack);
        if let Value::Err(e) = crit {
            return Value::Err(e);
        }
        let sum_origin = if a.len() == 3 {
            match &a[2] {
                Expr::Range(sp, sq) => Some((sp.0.min(sq.0), sp.1.min(sq.1))),
                _ => return Value::Err(E_VAL),
            }
        } else {
            None
        };
        let (r0, _, c0, _) = Self::range_bounds(p, q);
        let mut count = 0.0;
        let mut sum = 0.0;
        for (pos, v) in self.range_cells(p, q, cache, stack) {
            if matches_crit(&v, &crit) {
                count += 1.0;
                if f == "SUMIF" {
                    let target = match sum_origin {
                        Some((sr, sc)) => (sr + (pos.0 - r0), sc + (pos.1 - c0)),
                        None => pos,
                    };
                    match self.cell(target, cache, stack) {
                        Value::Num(n) => sum += n,
                        Value::Err(e) => return Value::Err(e),
                        _ => {}
                    }
                }
            }
        }
        Value::Num(if f == "SUMIF" { sum } else { count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_one(cells: &[(&str, &str)], target: &str) -> String {
        let mut s = Sheet::default();
        for (k, v) in cells {
            s.set_raw(parse_cell_ref(k).unwrap(), v.to_string());
        }
        s.recalc();
        s.value(parse_cell_ref(target).unwrap()).display()
    }

    #[test]
    fn arithmetic() {
        assert_eq!(eval_one(&[("A1", "=2+3*4")], "A1"), "14");
        assert_eq!(eval_one(&[("A1", "=-2^2")], "A1"), "4");
        assert_eq!(eval_one(&[("A1", "=(1+2)*3")], "A1"), "9");
        assert_eq!(eval_one(&[("A1", "=0,1+0,2")], "A1"), "0,3");
        assert_eq!(eval_one(&[("A1", "=50%")], "A1"), "0,5");
        assert_eq!(eval_one(&[("A1", "=1/0")], "A1"), "#ДЕЛ/0!");
    }

    #[test]
    fn russian_functions() {
        let c = [("A1", "1"), ("A2", "2"), ("A3", "3,5"), ("B1", "=СУММ(A1:A3)")];
        assert_eq!(eval_one(&c, "B1"), "6,5");
        assert_eq!(eval_one(&[("A1", "=СРЗНАЧ(1;2;3)")], "A1"), "2");
        assert_eq!(eval_one(&[("A1", "=ЕСЛИ(5>3;\"да\";\"нет\")")], "A1"), "да");
        assert_eq!(eval_one(&[("A1", "=сумм(1;2)")], "A1"), "3");
        assert_eq!(eval_one(&[("A1", "=СЧЕТ(1;2;\"x\")")], "A1"), "2");
        assert_eq!(eval_one(&[("A1", "=ОКРУГЛ(2,675;2)")], "A1"), "2,68");
        assert_eq!(eval_one(&[("A1", "=ЛЕВСИМВ(\"привет\";3)&ПРАВСИМВ(\"мир\";1)")], "A1"), "приp".replace('p', "р"));
        assert_eq!(eval_one(&[("A1", "=ОСТАТ(-3;2)")], "A1"), "1");
        assert_eq!(eval_one(&[("A1", "=НЕЗНАЮ(1)")], "A1"), "#ИМЯ?");
    }

    #[test]
    fn conditional() {
        let c = [("A1", "1"), ("A2", "5"), ("A3", "7"), ("B1", "10"), ("B2", "20"), ("B3", "30")];
        let mut v = c.to_vec();
        v.push(("C1", "=СЧЁТЕСЛИ(A1:A3;\">2\")"));
        v.push(("C2", "=СУММЕСЛИ(A1:A3;\">2\";B1:B3)"));
        assert_eq!(eval_one(&v, "C1"), "2");
        assert_eq!(eval_one(&v, "C2"), "50");
    }

    #[test]
    fn cycles_and_chain() {
        assert_eq!(eval_one(&[("A1", "=B1"), ("B1", "=A1")], "A1"), "#ЦИКЛ!");
        let mut s = Sheet::default();
        s.set_raw((0, 0), "1".into());
        for r in 1..3000 {
            s.set_raw((r, 0), format!("=A{}+1", r));
        }
        s.recalc();
        assert_eq!(s.value((2999, 0)).display(), "3000");
    }

    #[test]
    fn translation() {
        assert_eq!(translate_formula("СУММ(A1:A3;2,5)", true), "SUM(A1:A3,2.5)");
        assert_eq!(translate_formula("SUM(A1:A3,IF(B1>0,1,2))", false), "СУММ(A1:A3;ЕСЛИ(B1>0;1;2))");
        assert_eq!(translate_formula("ЕСЛИ(A1=\"а;б\";ИСТИНА;1)", true), "IF(A1=\"а;б\",TRUE,1)");
    }
}
