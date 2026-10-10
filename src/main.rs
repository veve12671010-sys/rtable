mod workbook;

use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use workbook::{Cell, Sheet};

struct RusTabApp {
    sheet: Sheet,
    selected: (usize, usize),
    edit_value: String,
    status: String,
    current_file: Option<PathBuf>,
}

impl Default for RusTabApp {
    fn default() -> Self {
        let mut sheet = Sheet::new(40, 16);
        sheet.set(0, 0, "Название".into());
        sheet.set(0, 1, "Количество".into());
        sheet.set(0, 2, "Цена".into());
        sheet.set(0, 3, "=B2*C2".into());
        sheet.set(1, 0, "Пример товара".into());
        sheet.set(1, 1, "3".into());
        sheet.set(1, 2, "150".into());
        sheet.set(1, 3, "=B3*C3".into());
        Self {
            edit_value: sheet.get(0, 0).raw.clone(),
            sheet,
            selected: (0, 0),
            status: "Готово. Откройте файл Excel или начните вводить данные.".into(),
            current_file: None,
        }
    }
}

impl RusTabApp {
    fn select_cell(&mut self, row: usize, col: usize) {
        self.selected = (row, col);
        self.edit_value = self.sheet.get(row, col).raw.clone();
    }

    fn open_file(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Таблицы Excel", &["xlsx", "xls", "xlsm"])
            .add_filter("CSV", &["csv"])
            .pick_file()
        {
            match workbook::load(&path) {
                Ok(sheet) => {
                    self.sheet = sheet;
                    self.current_file = Some(path.clone());
                    self.select_cell(0, 0);
                    self.status = format!("Открыт файл: {}", path.display());
                }
                Err(e) => self.status = format!("Не удалось открыть файл: {e}"),
            }
        }
    }

    fn save_as(&mut self) {
        let path = FileDialog::new()
            .add_filter("Книга Excel", &["xlsx"])
            .set_file_name("Таблица.xlsx")
            .save_file();
        if let Some(path) = path {
            match workbook::save(&path, &self.sheet) {
                Ok(()) => {
                    self.current_file = Some(path.clone());
                    self.status = format!("Сохранено: {}", path.display());
                }
                Err(e) => self.status = format!("Ошибка сохранения: {e}"),
            }
        }
    }

    fn commit_edit(&mut self) {
        let (r, c) = self.selected;
        self.sheet.set(r, c, self.edit_value.clone());
    }
}

impl eframe::App for RusTabApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("RusTab");
                ui.separator();
                if ui.button("📂 Открыть").clicked() { self.open_file(); }
                if ui.button("💾 Сохранить как…").clicked() { self.save_as(); }
                if ui.button("＋ Новая таблица").clicked() {
                    self.sheet = Sheet::new(40, 16);
                    self.current_file = None;
                    self.select_cell(0, 0);
                    self.status = "Создана новая таблица".into();
                }
                ui.separator();
                ui.label("Файл → открыть .xlsx/.xls/.csv · Сохранить как → .xlsx");
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Ячейка: {}", workbook::cell_name(self.selected.0, self.selected.1)));
                });
            });
        });

        egui::TopBottomPanel::top("formula").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("fx");
                let response = ui.add_sized([80.0, 26.0], egui::Label::new(
                    workbook::cell_name(self.selected.0, self.selected.1)
                ));
                let _ = response;
                ui.add_sized([ui.available_width(), 26.0], egui::TextEdit::singleline(&mut self.edit_value)
                    .hint_text("Введите значение или формулу, например =A1+B1"));
                if ui.button("Применить").clicked() {
                    self.commit_edit();
                    self.status = "Изменение применено".into();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Демонстрационная версия. Двойной щелчок по ячейке — выбрать её; редактируйте значение в строке fx.");
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("sheet_grid").striped(true).spacing([1.0, 1.0]).show(ui, |ui| {
                    ui.label("#");
                    for c in 0..self.sheet.cols {
                        ui.add_sized([105.0, 24.0], egui::Label::new(workbook::col_name(c)));
                    }
                    ui.end_row();

                    for r in 0..self.sheet.rows {
                        ui.label((r + 1).to_string());
                        for c in 0..self.sheet.cols {
                            let selected = self.selected == (r, c);
                            let shown = self.sheet.display(r, c);
                            let button = egui::Button::new(shown)
                                .min_size(egui::vec2(105.0, 24.0))
                                .fill(if selected {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    ui.visuals().widgets.inactive.bg_fill
                                });
                            if ui.add(button).clicked() {
                                self.select_cell(r, c);
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 760.0])
            .with_title("RusTab — электронные таблицы"),
        ..Default::default()
    };
    eframe::run_native("RusTab", options, Box::new(|_cc| Ok(Box::new(RusTabApp::default()))))
}
