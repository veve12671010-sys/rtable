#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod engine;
mod xlsx_io;

use eframe::egui::{self, Align2, Color32, FontId, Key, Sense, Stroke};
use egui_extras::{Column, TableBuilder};
use engine::{cell_name, col_name, Pos, Sheet, Value, FUNCS};
use std::path::PathBuf;

#[cfg(windows)]
fn pick_open() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("Таблицы", &["xlsx", "xlsm", "xls", "xlsb", "ods"]).pick_file()
}
#[cfg(windows)]
fn pick_save() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("Книга Excel (*.xlsx)", &["xlsx"]).set_file_name("Книга1.xlsx").save_file()
}
#[cfg(not(windows))]
fn pick_open() -> Option<PathBuf> {
    None
}
#[cfg(not(windows))]
fn pick_save() -> Option<PathBuf> {
    Some(PathBuf::from("Книга1.xlsx"))
}

struct App {
    sheets: Vec<(String, Sheet)>,
    active: usize,
    sel: Pos,
    edit_buf: String,
    edit_target: (usize, Pos),
    focus_bar: bool,
    file: Option<PathBuf>,
    status: String,
}

impl App {
    fn new() -> Self {
        let mut app = App {
            sheets: vec![("Лист1".to_string(), Sheet::default())],
            active: 0,
            sel: (0, 0),
            edit_buf: String::new(),
            edit_target: (usize::MAX, (0, 0)),
            focus_bar: false,
            file: None,
            status: "Готово".into(),
        };
        if let Some(arg) = std::env::args().nth(1) {
            app.load_path(PathBuf::from(arg));
        }
        app
    }

    fn sheet(&self) -> &Sheet {
        &self.sheets[self.active].1
    }

    fn load_buf(&mut self) {
        self.edit_buf = self.sheet().raw(self.sel).to_string();
        self.edit_target = (self.active, self.sel);
    }

    fn commit(&mut self) {
        let (si, pos) = self.edit_target;
        if let Some((_, sh)) = self.sheets.get_mut(si) {
            if sh.raw(pos) != self.edit_buf {
                sh.set(pos, self.edit_buf.clone());
            }
        }
    }

    fn load_path(&mut self, p: PathBuf) {
        match xlsx_io::load(&p) {
            Ok(s) => {
                self.sheets = s;
                self.active = 0;
                self.sel = (0, 0);
                self.status = format!("Открыто: {}", p.display());
                self.file = Some(p);
                self.load_buf();
            }
            Err(e) => self.status = format!("Ошибка открытия: {e}"),
        }
    }

    fn open(&mut self) {
        self.commit();
        if let Some(p) = pick_open() {
            self.load_path(p);
        }
    }

    fn save(&mut self, save_as: bool) {
        self.commit();
        let is_xlsx = |p: &PathBuf| p.extension().map(|e| e.eq_ignore_ascii_case("xlsx")).unwrap_or(false);
        let path = match &self.file {
            Some(p) if !save_as && is_xlsx(p) => Some(p.clone()),
            _ => pick_save(),
        };
        if let Some(mut p) = path {
            if p.extension().is_none() {
                p.set_extension("xlsx");
            }
            match xlsx_io::save(&p, &self.sheets) {
                Ok(()) => {
                    self.status = format!("Сохранено: {}", p.display());
                    self.file = Some(p);
                }
                Err(e) => self.status = format!("Ошибка сохранения: {e}"),
            }
        }
    }

    fn new_book(&mut self) {
        self.commit();
        self.sheets = vec![("Лист1".to_string(), Sheet::default())];
        self.active = 0;
        self.sel = (0, 0);
        self.file = None;
        self.status = "Новая книга".into();
        self.load_buf();
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let (mut dr, mut dc) = (0i32, 0i32);
        let (mut del, mut f2) = (false, false);
        let mut typed: Option<String> = None;
        ctx.input(|i| {
            for ev in &i.events {
                match ev {
                    egui::Event::Key { key, pressed: true, modifiers, .. } if !modifiers.command => match key {
                        Key::ArrowUp => dr = -1,
                        Key::ArrowDown | Key::Enter => dr = 1,
                        Key::ArrowLeft => dc = -1,
                        Key::ArrowRight | Key::Tab => dc = 1,
                        Key::Delete | Key::Backspace => del = true,
                        Key::F2 => f2 = true,
                        _ => {}
                    },
                    egui::Event::Text(t) if !t.chars().all(|c| c.is_control()) => typed = Some(t.clone()),
                    _ => {}
                }
            }
        });
        if dr != 0 || dc != 0 {
            self.sel.0 = (self.sel.0 as i32 + dr).max(0) as usize;
            self.sel.1 = (self.sel.1 as i32 + dc).max(0) as usize;
        }
        if del {
            let sel = self.sel;
            self.sheets[self.active].1.set(sel, String::new());
            self.load_buf();
        }
        if f2 {
            self.focus_bar = true;
        }
        if let Some(t) = typed {
            self.commit();
            self.edit_buf = t;
            self.edit_target = (self.active, self.sel);
            self.focus_bar = true;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::S)) {
            self.save(false);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::O)) {
            self.open();
        }
        if !ctx.wants_keyboard_input() {
            self.handle_keys(ctx);
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Новый").clicked() {
                    self.new_book();
                }
                if ui.button("Открыть…").clicked() {
                    self.open();
                }
                if ui.button("Сохранить").clicked() {
                    self.save(false);
                }
                if ui.button("Сохранить как…").clicked() {
                    self.save(true);
                }
                ui.menu_button("Функции", |ui| {
                    for (ru, en) in FUNCS {
                        if ui.button(format!("{ru}   ({en})")).clicked() {
                            self.commit();
                            self.edit_buf = format!("={ru}(");
                            self.edit_target = (self.active, self.sel);
                            self.focus_bar = true;
                            ui.close_menu();
                        }
                    }
                });
                if let Some(f) = &self.file {
                    ui.label(f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                }
            });
        });

        let mut bar_focused = false;
        egui::TopBottomPanel::top("bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([60.0, 22.0], egui::Label::new(egui::RichText::new(cell_name(self.sel)).strong()));
                ui.label("fx");
                let resp = ui.add(egui::TextEdit::singleline(&mut self.edit_buf).desired_width(f32::INFINITY));
                if self.focus_bar {
                    resp.request_focus();
                    if let Some(mut st) = egui::TextEdit::load_state(ctx, resp.id) {
                        let n = self.edit_buf.chars().count();
                        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(n))));
                        st.store(ctx, resp.id);
                    }
                    self.focus_bar = false;
                }
                bar_focused = resp.has_focus();
                if resp.lost_focus() {
                    let (esc, enter, tab) = ctx.input(|i| (i.key_pressed(Key::Escape), i.key_pressed(Key::Enter), i.key_pressed(Key::Tab)));
                    if esc {
                        let (si, pos) = self.edit_target;
                        if let Some((_, sh)) = self.sheets.get(si) {
                            self.edit_buf = sh.raw(pos).to_string();
                        }
                    } else {
                        self.commit();
                        if enter {
                            self.sel = (self.edit_target.1 .0 + 1, self.edit_target.1 .1);
                        } else if tab {
                            self.sel = (self.edit_target.1 .0, self.edit_target.1 .1 + 1);
                        }
                    }
                }
            });
        });
        if !bar_focused && self.edit_target != (self.active, self.sel) {
            self.load_buf();
        }

        egui::TopBottomPanel::bottom("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut switch = None;
                for (i, (name, _)) in self.sheets.iter().enumerate() {
                    if ui.selectable_label(i == self.active, name).clicked() {
                        switch = Some(i);
                    }
                }
                if ui.button("+").clicked() {
                    let n = self.sheets.len() + 1;
                    self.sheets.push((format!("Лист{n}"), Sheet::default()));
                    switch = Some(self.sheets.len() - 1);
                }
                if let Some(i) = switch {
                    self.commit();
                    self.active = i;
                    self.sel = (0, 0);
                    self.load_buf();
                }
                ui.separator();
                ui.label(&self.status);
            });
        });

        let (ext_r, ext_c) = self.sheet().extent();
        let rows = (ext_r.max(self.sel.0) + 40).max(100);
        let cols = (ext_c.max(self.sel.1) + 3).clamp(26, 200);
        let sel = self.sel;
        let mut clicked: Option<Pos> = None;
        let mut dbl = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            let sheet = &self.sheets[self.active].1;
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let mut tb = TableBuilder::new(ui).resizable(true).column(Column::exact(44.0));
                for _ in 0..cols {
                    tb = tb.column(Column::initial(90.0).clip(true));
                }
                tb.header(22.0, |mut h| {
                    h.col(|_| {});
                    for c in 0..cols {
                        h.col(|ui| {
                            ui.centered_and_justified(|ui| ui.label(egui::RichText::new(col_name(c)).strong()));
                        });
                    }
                })
                .body(|body| {
                    body.rows(22.0, rows, |mut row| {
                        let r = row.index();
                        row.col(|ui| {
                            ui.centered_and_justified(|ui| ui.weak((r + 1).to_string()));
                        });
                        for c in 0..cols {
                            row.col(|ui| {
                                let rect = ui.max_rect();
                                let resp = ui.interact(rect, ui.id().with(("cell", r, c)), Sense::click());
                                let p = (r, c);
                                let v = sheet.value(p);
                                if p == sel {
                                    ui.painter().rect_filled(rect, 0.0, ui.visuals().selection.bg_fill.linear_multiply(0.35));
                                    ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, ui.visuals().selection.stroke.color));
                                } else {
                                    ui.painter().rect_stroke(rect, 0.0, Stroke::new(0.5, ui.visuals().widgets.noninteractive.bg_stroke.color));
                                }
                                let txt = v.display();
                                if !txt.is_empty() {
                                    let (pos, align) = match v {
                                        Value::Num(_) | Value::Bool(_) => (rect.right_center() - egui::vec2(4.0, 0.0), Align2::RIGHT_CENTER),
                                        _ => (rect.left_center() + egui::vec2(4.0, 0.0), Align2::LEFT_CENTER),
                                    };
                                    let color = if matches!(v, Value::Err(_)) { Color32::from_rgb(200, 50, 50) } else { ui.visuals().text_color() };
                                    ui.painter().with_clip_rect(rect).text(pos, align, txt, FontId::proportional(14.0), color);
                                }
                                if resp.clicked() {
                                    clicked = Some(p);
                                }
                                if resp.double_clicked() {
                                    dbl = true;
                                }
                            });
                        }
                    });
                });
            });
        });
        if let Some(p) = clicked {
            self.sel = p;
        }
        if dbl {
            self.focus_bar = true;
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 700.0]).with_title("Таблица"),
        ..Default::default()
    };
    eframe::run_native("Таблица", options, Box::new(|_cc| Box::new(App::new())))
}
